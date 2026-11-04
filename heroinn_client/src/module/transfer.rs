//! Client-wide transfer accounting and throttling.
//!
//! Every control message, shell byte and file-transfer byte crosses this
//! module, so one token bucket enforces the aggregate limit in both directions.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use heroinn_util::{DEFAULT_TRANSFER_RATE, Error, Result, protocol::Transport};

/// Smallest burst retained so control traffic can make progress at low limits.
const MIN_BURST_BYTES: u64 = 4 * 1024;

/// Largest burst retained for high limits.
const MAX_BURST_BYTES: u64 = 256 * 1024;

/// Longest single sleep, keeping rate changes responsive.
const MAX_SLEEP: Duration = Duration::from_millis(50);

static G_LIMITER: LazyLock<TransferLimiter> =
    LazyLock::new(|| TransferLimiter::new(DEFAULT_TRANSFER_RATE));
static G_IN_BYTES: AtomicU64 = AtomicU64::new(0);
static G_OUT_BYTES: AtomicU64 = AtomicU64::new(0);

struct Bucket {
    rate: u64,
    tokens: f64,
    last_refill: Instant,
}

impl Bucket {
    fn new(rate: u64, now: Instant) -> Self {
        let rate = rate.max(1);
        Self {
            rate,
            tokens: Self::capacity(rate) as f64,
            last_refill: now,
        }
    }

    fn capacity(rate: u64) -> u64 {
        rate.clamp(MIN_BURST_BYTES, MAX_BURST_BYTES)
    }

    fn refill(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.last_refill);
        if elapsed.is_zero() {
            return;
        }

        let capacity = Self::capacity(self.rate) as f64;
        self.tokens = (self.tokens + elapsed.as_secs_f64() * self.rate as f64).min(capacity);
        self.last_refill = now;
    }

    fn set_rate(&mut self, rate: u64, now: Instant) {
        self.refill(now);
        self.rate = rate.max(1);
        self.tokens = self.tokens.min(Self::capacity(self.rate) as f64);
    }

    fn take(&mut self, bytes: u64, now: Instant) -> Option<Duration> {
        self.refill(now);

        let bytes = bytes as f64;
        if self.tokens >= bytes {
            self.tokens -= bytes;
            return None;
        }

        let missing = bytes - self.tokens;
        Some(Duration::from_secs_f64(missing / self.rate as f64))
    }
}

struct TransferLimiter {
    bucket: Mutex<Bucket>,
}

impl TransferLimiter {
    fn new(rate: u64) -> Self {
        Self {
            bucket: Mutex::new(Bucket::new(rate, Instant::now())),
        }
    }

    fn lock_bucket(&self) -> std::sync::MutexGuard<'_, Bucket> {
        self.bucket
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn set_rate(&self, rate: u64) {
        let now = Instant::now();
        let mut bucket = self.lock_bucket();
        bucket.set_rate(rate, now);
    }

    fn rate(&self) -> u64 {
        self.lock_bucket().rate
    }

    /// Block until `bytes` may be transferred, splitting work into bursts.
    fn acquire(&self, bytes: u64) {
        let mut remaining = bytes;

        while remaining > 0 {
            let (chunk, wait) = {
                let mut bucket = self.lock_bucket();
                let chunk = remaining.min(Bucket::capacity(bucket.rate));
                (chunk, bucket.take(chunk, Instant::now()))
            };

            match wait {
                Some(wait) => std::thread::sleep(wait.min(MAX_SLEEP)),
                None => remaining -= chunk,
            }
        }
    }
}

/// Change the aggregate limit for all client traffic.
pub fn set_rate(bytes_per_sec: u64) -> Result<()> {
    if bytes_per_sec == 0 {
        return Err(Error::Other(
            "transfer rate must be greater than zero".to_string(),
        ));
    }

    G_LIMITER.set_rate(bytes_per_sec);
    Ok(())
}

/// Current aggregate limit in bytes per second.
pub fn current_rate() -> u64 {
    G_LIMITER.rate()
}

/// Send through the shared limiter and account for successfully sent bytes.
pub fn send(transport: &mut Transport, buf: &[u8]) -> Result<()> {
    G_LIMITER.acquire(buf.len() as u64);
    transport.send(buf)?;
    G_OUT_BYTES.fetch_add(buf.len() as u64, Ordering::Relaxed);
    Ok(())
}

/// Receive through the shared limiter and account for received bytes.
pub fn recv(transport: &mut Transport) -> Result<Vec<u8>> {
    let buf = transport.recv()?;
    G_LIMITER.acquire(buf.len() as u64);
    G_IN_BYTES.fetch_add(buf.len() as u64, Ordering::Relaxed);
    Ok(buf)
}

/// Atomically take the traffic counters accumulated since the last call.
pub fn take_rates() -> (u64, u64) {
    (
        G_IN_BYTES.swap(0, Ordering::Relaxed),
        G_OUT_BYTES.swap(0, Ordering::Relaxed),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limit_is_one_mebibyte() {
        let limiter = TransferLimiter::new(DEFAULT_TRANSFER_RATE);
        assert_eq!(limiter.rate(), 1024 * 1024);
    }

    #[test]
    fn changing_the_limit_updates_the_bucket() {
        let limiter = TransferLimiter::new(DEFAULT_TRANSFER_RATE);
        limiter.set_rate(256 * 1024);
        assert_eq!(limiter.rate(), 256 * 1024);
        assert!(set_rate(0).is_err());
    }

    #[test]
    fn bucket_consumes_tokens_and_refills_over_time() {
        let start = Instant::now();
        let mut bucket = Bucket::new(100 * 1024, start);

        assert!(
            bucket.take(100 * 1024, start).is_none(),
            "initial burst should be available"
        );

        let wait = bucket
            .take(50 * 1024, start)
            .expect("the empty bucket must ask callers to wait");
        assert!(wait >= Duration::from_millis(450));

        assert!(
            bucket
                .take(50 * 1024, start + Duration::from_millis(500))
                .is_none(),
            "half a second at 100 KiB/s should refill 50 KiB"
        );
    }

    #[test]
    fn burst_capacity_is_bounded() {
        assert_eq!(Bucket::capacity(1), MIN_BURST_BYTES);
        assert_eq!(Bucket::capacity(10 * 1024 * 1024), MAX_BURST_BYTES);
    }

    #[test]
    fn concurrent_transfers_share_one_rate_budget() {
        let rate = 64 * 1024;
        let limiter = std::sync::Arc::new(TransferLimiter::new(rate));

        // Drain the initial burst so the measurement starts with an empty bucket.
        limiter.acquire(rate);

        let start = Instant::now();
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let limiter = limiter.clone();
                std::thread::spawn(move || limiter.acquire(rate / 2))
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // Two independent 32 KiB transfers would finish together in ~0.5 s.
        // Sharing one 64 KiB/s bucket must serialize them into ~1 s.
        assert!(
            start.elapsed() >= Duration::from_millis(900),
            "concurrent transfers must not exceed the aggregate limit"
        );
    }
}
