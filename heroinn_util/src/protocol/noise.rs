//! End-to-end transport encryption using Noise NN.
//!
//! Every main connection, reverse tunnel and reliable-UDP peer owns one
//! independent [`NoiseSession`]. The Noise transcript is preceded by a fixed,
//! canonical [`NoiseIntro`] so a peer cannot silently fall back to plaintext or
//! reuse a session for another transport/purpose.

use std::io::ErrorKind;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(test)]
use std::collections::VecDeque;
#[cfg(test)]
use std::sync::Condvar;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(test)]
use std::time::Instant;

use snow::{Builder, HandshakeState, TransportState, params::NoiseParams};
use uuid::Uuid;
use zeroize::Zeroize;

use crate::HeroinnProtocol;
use crate::error::{Error, Result};

const SUITE: &str = "Noise_NN_25519_ChaChaPoly_BLAKE2s";
const PROLOGUE_PREFIX: &[u8] = b"HEROINN-NOISE-NN-V1";
const INTRO_MAGIC: [u8; 8] = *b"HRNNOIS1";
const INTRO_VERSION: u8 = 1;
const SUITE_ID: u8 = 1;
const INTRO_LEN: usize = 24;
const INTRO_ROLE_RESERVED_MASK: u16 = 0xff00;
const MAX_NOISE_MESSAGE: usize = 10 * 1024 * 1024;
const RECORD_MAGIC: [u8; 4] = *b"HNR1";
const RECORD_VERSION: u8 = 1;
const RECORD_HEADER_LEN: usize = 30;
const RECORD_TAG_LEN: usize = 16;
const RECORD_DATA: u8 = 1;
const RECORD_FIN: u8 = 2;
/// Noise permits a 65,519-byte plaintext message for this suite.
pub const NOISE_MAX_PLAINTEXT_BYTES: usize = 65_519;

/// Time allowed for the preface and both NN handshake messages.
pub const NOISE_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
/// Plaintext application bytes carried by one TCP/HTTP record.
///
/// The Noise message limit applies to the authenticated plaintext, including
/// the 30-byte record header. The dispatch-approved stream limit is exactly
/// 60 KiB: it stays below the Noise plaintext limit while reducing a full
/// 64 KiB transport chunk to two records.
pub const STREAM_RECORD_PAYLOAD_BYTES: usize = 60 * 1024;
/// Backward-compatible name used by transport call sites.
pub const STREAM_RECORD_PAYLOAD: usize = STREAM_RECORD_PAYLOAD_BYTES;
/// Plaintext application bytes carried by one reliable-UDP record.
pub const UDP_RECORD_PAYLOAD: usize = 8 * 1024;
/// Largest encrypted stream record accepted before AEAD processing.
pub const STREAM_RECORD_MAX: usize = STREAM_RECORD_PAYLOAD + RECORD_HEADER_LEN + RECORD_TAG_LEN;
/// Largest encrypted UDP record accepted before AEAD processing.
pub const UDP_RECORD_MAX: usize = UDP_RECORD_PAYLOAD + RECORD_HEADER_LEN + RECORD_TAG_LEN;
const _: () = assert!(STREAM_RECORD_PAYLOAD + RECORD_HEADER_LEN <= NOISE_MAX_PLAINTEXT_BYTES);

/// Transport named in the authenticated prologue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoiseTransport {
    Tcp,
    Http,
    Udp,
}

impl NoiseTransport {
    fn as_u8(self) -> u8 {
        match self {
            Self::Tcp => 0,
            Self::Http => 1,
            Self::Udp => 2,
        }
    }

    fn from_u8(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::Tcp),
            1 => Ok(Self::Http),
            2 => Ok(Self::Udp),
            _ => Err(Error::Noise("unsupported transport in intro")),
        }
    }
}

impl From<&HeroinnProtocol> for NoiseTransport {
    fn from(protocol: &HeroinnProtocol) -> Self {
        match protocol {
            HeroinnProtocol::TCP => Self::Tcp,
            HeroinnProtocol::HTTP => Self::Http,
            HeroinnProtocol::UDP => Self::Udp,
        }
    }
}

/// Purpose of one independently keyed Noise session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoiseKind {
    Main,
    Tunnel { port: u16 },
}

impl NoiseKind {
    fn encode(self) -> (u8, u16) {
        match self {
            Self::Main => (0, 0),
            Self::Tunnel { port } => (1, port),
        }
    }

    fn decode(kind: u8, port: u16) -> Result<Self> {
        match (kind, port) {
            (0, 0) => Ok(Self::Main),
            (1, value) if value != 0 => Ok(Self::Tunnel { port: value }),
            _ => Err(Error::Noise("invalid session purpose in intro")),
        }
    }
}

/// Wire role of the peer that emitted a [`NoiseIntro`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NoiseRole {
    Initiator,
    Responder,
}

impl NoiseRole {
    fn encode(self) -> u16 {
        match self {
            Self::Initiator => 1,
            Self::Responder => 2,
        }
    }

    fn decode(flags: u16) -> Result<Self> {
        if flags & INTRO_ROLE_RESERVED_MASK != 0 {
            return Err(Error::Noise("unsupported noise role flags"));
        }
        match flags {
            1 => Ok(Self::Initiator),
            2 => Ok(Self::Responder),
            _ => Err(Error::Noise("invalid noise peer role")),
        }
    }

    fn peer(self) -> Self {
        match self {
            Self::Initiator => Self::Responder,
            Self::Responder => Self::Initiator,
        }
    }
}

/// Canonical preface sent before the NN handshake.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoiseIntro {
    transport: NoiseTransport,
    kind: NoiseKind,
    role: NoiseRole,
    session_id: u64,
}

impl NoiseIntro {
    pub fn new(transport: NoiseTransport, kind: NoiseKind) -> Self {
        let mut session_id = Uuid::new_v4().as_u128() as u64;
        if session_id == 0 {
            session_id = 1;
        }
        Self {
            transport,
            kind,
            role: NoiseRole::Initiator,
            session_id,
        }
    }

    #[cfg(test)]
    fn with_session_id(transport: NoiseTransport, kind: NoiseKind, session_id: u64) -> Self {
        Self {
            transport,
            kind,
            role: NoiseRole::Initiator,
            session_id: session_id.max(1),
        }
    }

    pub fn transport(&self) -> NoiseTransport {
        self.transport
    }

    pub fn kind(&self) -> NoiseKind {
        self.kind
    }

    pub fn session_id(&self) -> u64 {
        self.session_id
    }

    pub fn encode(self) -> [u8; INTRO_LEN] {
        let (kind, port) = self.kind.encode();
        let mut out = [0u8; INTRO_LEN];
        out[..8].copy_from_slice(&INTRO_MAGIC);
        out[8] = INTRO_VERSION;
        out[9] = SUITE_ID;
        out[10] = self.transport.as_u8();
        out[11] = kind;
        out[12..14].copy_from_slice(&self.role.encode().to_be_bytes());
        out[14..16].copy_from_slice(&port.to_be_bytes());
        out[16..24].copy_from_slice(&self.session_id.to_be_bytes());
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != INTRO_LEN {
            return Err(Error::Noise("invalid intro length"));
        }
        if bytes[..8] != INTRO_MAGIC {
            return Err(Error::Noise("invalid intro magic"));
        }
        if bytes[8] != INTRO_VERSION {
            return Err(Error::Noise("unsupported noise version"));
        }
        if bytes[9] != SUITE_ID {
            return Err(Error::Noise("unsupported noise suite"));
        }
        let flags = u16::from_be_bytes([bytes[12], bytes[13]]);
        let role = NoiseRole::decode(flags)?;
        let transport = NoiseTransport::from_u8(bytes[10])?;
        let port = u16::from_be_bytes([bytes[14], bytes[15]]);
        let kind = NoiseKind::decode(bytes[11], port)?;
        let session_id = u64::from_be_bytes(bytes[16..24].try_into().expect("fixed intro length"));
        if session_id == 0 {
            return Err(Error::Noise("invalid zero session id"));
        }
        Ok(Self {
            transport,
            kind,
            role,
            session_id,
        })
    }

    fn validate(
        &self,
        transport: NoiseTransport,
        kind: NoiseKind,
        local_role: NoiseRole,
    ) -> Result<()> {
        if self.transport != transport {
            return Err(Error::Noise("intro transport mismatch"));
        }
        if self.kind != kind {
            return Err(Error::Noise("intro purpose mismatch"));
        }
        if self.role != local_role.peer() {
            return Err(Error::Noise("intro peer role mismatch"));
        }
        Ok(())
    }

    fn prologue(&self) -> Vec<u8> {
        let mut prologue = Vec::with_capacity(PROLOGUE_PREFIX.len() + INTRO_LEN);
        prologue.extend_from_slice(PROLOGUE_PREFIX);
        prologue.extend_from_slice(&self.encode());
        prologue
    }
}

/// Transport-neutral carrier used while the Noise handshake is established.
pub(crate) trait NoiseWire: Send + Sync {
    fn send_plain(&self, payload: &[u8]) -> Result<()>;
    /// Whether a completed Noise message must wait for the carrier to flush.
    ///
    /// WebSocket has an owned writer actor with a bounded queue: enqueueing
    /// preserves ordering and the actor flushes each batch without a
    /// per-message barrier. Other carriers, including the plain TCP writer,
    /// retain synchronous flush semantics.
    fn flush_after_send(&self) -> bool {
        true
    }
    /// Best-effort bounded send used by shutdown paths. Implementations that
    /// cannot bound their write keep the default blocking behaviour.
    fn send_plain_timeout(&self, payload: &[u8], timeout: Duration) -> Result<()> {
        let _ = timeout;
        self.send_plain(payload)
    }
    fn recv_plain(&self) -> Result<Vec<u8>>;
    fn recv_plain_timeout(&self, timeout: Duration) -> Result<Vec<u8>>;
    fn flush_plain(&self) -> Result<()>;
}

#[derive(Clone, Copy)]
struct RecordHeader {
    kind: u8,
    message_id: u64,
    fragment_index: u32,
    fragment_count: u32,
    total_len: u32,
    payload_len: u32,
}

impl RecordHeader {
    fn encode(self) -> [u8; RECORD_HEADER_LEN] {
        let mut out = [0u8; RECORD_HEADER_LEN];
        out[..4].copy_from_slice(&RECORD_MAGIC);
        out[4] = RECORD_VERSION;
        out[5] = self.kind;
        out[6..8].copy_from_slice(&0u16.to_be_bytes());
        out[8..16].copy_from_slice(&self.message_id.to_be_bytes());
        out[16..20].copy_from_slice(&self.fragment_index.to_be_bytes());
        out[20..24].copy_from_slice(&self.fragment_count.to_be_bytes());
        out[24..28].copy_from_slice(&self.total_len.to_be_bytes());
        out[28..30].copy_from_slice(&(self.payload_len as u16).to_be_bytes());
        out
    }

    fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < RECORD_HEADER_LEN {
            return Err(Error::Noise("truncated record header"));
        }
        if bytes[..4] != RECORD_MAGIC {
            return Err(Error::Noise("invalid record magic"));
        }
        if bytes[4] != RECORD_VERSION {
            return Err(Error::Noise("unsupported record version"));
        }
        if bytes[6..8] != [0, 0] {
            return Err(Error::Noise("unsupported record flags"));
        }
        let payload_len = u16::from_be_bytes([bytes[28], bytes[29]]) as u32;
        Ok(Self {
            kind: bytes[5],
            message_id: u64::from_be_bytes(bytes[8..16].try_into().expect("fixed header")),
            fragment_index: u32::from_be_bytes(bytes[16..20].try_into().expect("fixed header")),
            fragment_count: u32::from_be_bytes(bytes[20..24].try_into().expect("fixed header")),
            total_len: u32::from_be_bytes(bytes[24..28].try_into().expect("fixed header")),
            payload_len,
        })
    }
}

struct ReceiveBuffer {
    message_id: u64,
    next_fragment: u32,
    fragment_count: u32,
    total_len: usize,
    bytes: Vec<u8>,
}

impl Drop for ReceiveBuffer {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}

struct NoiseState {
    transport: TransportState,
    send_message_id: u64,
    recv_message_id: u64,
    receive: Option<ReceiveBuffer>,
    send_fin: bool,
    recv_fin: bool,
    failed: bool,
}

/// One independently keyed encrypted message stream.
pub struct NoiseSession {
    wire: Arc<dyn NoiseWire>,
    state: Mutex<NoiseState>,
    send_lock: Mutex<()>,
    recv_lock: Mutex<()>,
    record_payload: usize,
    record_max: usize,
}

impl NoiseSession {
    fn new(
        wire: Arc<dyn NoiseWire>,
        transport: TransportState,
        record_payload: usize,
        record_max: usize,
    ) -> Self {
        Self {
            wire,
            state: Mutex::new(NoiseState {
                transport,
                send_message_id: 0,
                recv_message_id: 0,
                receive: None,
                send_fin: false,
                recv_fin: false,
                failed: false,
            }),
            send_lock: Mutex::new(()),
            recv_lock: Mutex::new(()),
            record_payload,
            record_max,
        }
    }

    pub fn send(&self, payload: &[u8]) -> Result<()> {
        if payload.len() > MAX_NOISE_MESSAGE {
            return Err(Error::PacketTooLarge(payload.len()));
        }
        let _send = self
            .send_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let fragment_count = fragment_count(payload.len(), self.record_payload);
        let message_id = {
            let state = lock_state(&self.state);
            if state.failed {
                return Err(Error::Noise("session is failed"));
            }
            if state.send_fin {
                return Err(Error::Noise("send side is closed"));
            }
            state.send_message_id
        };

        for index in 0..fragment_count {
            let start = index as usize * self.record_payload;
            let end = payload.len().min(start + self.record_payload);
            let fragment = &payload[start..end];
            let header = RecordHeader {
                kind: RECORD_DATA,
                message_id,
                fragment_index: index,
                fragment_count,
                total_len: payload.len() as u32,
                payload_len: fragment.len() as u32,
            };
            self.send_record(header, fragment)?;
            if lock_state(&self.state).failed {
                return Err(Error::Noise("session is failed"));
            }
        }

        {
            let mut state = lock_state(&self.state);
            state.send_message_id = state.send_message_id.wrapping_add(1);
        }
        if self.wire.flush_after_send()
            && let Err(error) = self.wire.flush_plain()
        {
            lock_state(&self.state).failed = true;
            return Err(error);
        }
        Ok(())
    }

    fn send_record(&self, header: RecordHeader, fragment: &[u8]) -> Result<()> {
        let mut ciphertext = self.seal_record(header, fragment)?;
        let sent = self.wire.send_plain(&ciphertext);
        ciphertext.zeroize();
        if let Err(error) = sent {
            lock_state(&self.state).failed = true;
            return Err(error);
        }
        Ok(())
    }

    fn seal_record(&self, header: RecordHeader, fragment: &[u8]) -> Result<Vec<u8>> {
        let mut plaintext = Vec::with_capacity(RECORD_HEADER_LEN + fragment.len());
        plaintext.extend_from_slice(&header.encode());
        plaintext.extend_from_slice(fragment);

        let mut ciphertext = vec![0u8; plaintext.len() + RECORD_TAG_LEN];
        let written = {
            let mut state = lock_state(&self.state);
            if state.failed {
                plaintext.zeroize();
                return Err(Error::Noise("session is failed"));
            }
            match state.transport.write_message(&plaintext, &mut ciphertext) {
                Ok(written) => written,
                Err(_) => {
                    state.failed = true;
                    plaintext.zeroize();
                    return Err(Error::Noise("record encryption failed"));
                }
            }
        };
        plaintext.zeroize();
        ciphertext.truncate(written);
        Ok(ciphertext)
    }

    pub fn recv(&self) -> Result<Vec<u8>> {
        let _recv = self
            .recv_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        loop {
            if lock_state(&self.state).recv_fin {
                return Err(Error::Io(std::io::Error::from(ErrorKind::UnexpectedEof)));
            }

            let mut ciphertext = self.wire.recv_plain()?;
            if ciphertext.len() > self.record_max {
                ciphertext.zeroize();
                lock_state(&self.state).failed = true;
                return Err(Error::Noise("encrypted record exceeds limit"));
            }
            if ciphertext.len() < RECORD_TAG_LEN {
                ciphertext.zeroize();
                lock_state(&self.state).failed = true;
                return Err(Error::Noise("truncated encrypted record"));
            }

            let mut plaintext = vec![0u8; ciphertext.len()];
            let opened = {
                let mut state = lock_state(&self.state);
                if state.failed {
                    ciphertext.zeroize();
                    return Err(Error::Noise("session is failed"));
                }
                match state.transport.read_message(&ciphertext, &mut plaintext) {
                    Ok(opened) => opened,
                    Err(_) => {
                        state.failed = true;
                        ciphertext.zeroize();
                        plaintext.zeroize();
                        return Err(Error::Noise("record authentication failed"));
                    }
                }
            };
            ciphertext.zeroize();
            plaintext.truncate(opened);

            let result = self.consume_plaintext(&plaintext);
            plaintext.zeroize();
            match result {
                Ok(Some(message)) => return Ok(message),
                Ok(None) => continue,
                Err(Error::Io(error)) if error.kind() == ErrorKind::UnexpectedEof => {
                    return Err(Error::Io(error));
                }
                Err(error) => {
                    lock_state(&self.state).failed = true;
                    return Err(error);
                }
            }
        }
    }

    fn consume_plaintext(&self, plaintext: &[u8]) -> Result<Option<Vec<u8>>> {
        let header = RecordHeader::decode(plaintext)?;
        if header.payload_len as usize != plaintext.len().saturating_sub(RECORD_HEADER_LEN) {
            return Err(Error::Noise("record payload length mismatch"));
        }
        let payload = &plaintext[RECORD_HEADER_LEN..];

        match header.kind {
            RECORD_DATA => {
                if header.total_len as usize > MAX_NOISE_MESSAGE {
                    return Err(Error::PacketTooLarge(header.total_len as usize));
                }
                let expected_count = fragment_count(header.total_len as usize, self.record_payload);
                if header.fragment_count != expected_count {
                    return Err(Error::Noise("invalid fragment count"));
                }
                if header.fragment_index >= header.fragment_count {
                    return Err(Error::Noise("fragment index out of range"));
                }
                let start = header.fragment_index as usize * self.record_payload;
                let expected_len = (header.total_len as usize)
                    .saturating_sub(start)
                    .min(self.record_payload);
                if payload.len() != expected_len {
                    return Err(Error::Noise("invalid fragment payload length"));
                }

                let mut state = lock_state(&self.state);
                if state.receive.is_none() {
                    if header.message_id != state.recv_message_id {
                        return Err(Error::Noise("record sequence mismatch"));
                    }
                    if header.fragment_index != 0 {
                        return Err(Error::Noise("first fragment is missing"));
                    }
                    state.receive = Some(ReceiveBuffer {
                        message_id: header.message_id,
                        next_fragment: 0,
                        fragment_count: header.fragment_count,
                        total_len: header.total_len as usize,
                        bytes: Vec::with_capacity(payload.len().min(header.total_len as usize)),
                    });
                }

                let receive = state.receive.as_mut().expect("receive was inserted");
                if receive.message_id != header.message_id {
                    return Err(Error::Noise("interleaved message fragments"));
                }
                if receive.fragment_count != header.fragment_count
                    || receive.total_len != header.total_len as usize
                {
                    return Err(Error::Noise("fragment metadata mismatch"));
                }
                if receive.next_fragment != header.fragment_index {
                    return Err(Error::Noise("fragment sequence mismatch"));
                }
                receive.bytes.extend_from_slice(payload);
                receive.next_fragment += 1;

                if receive.next_fragment == receive.fragment_count {
                    let mut done = state.receive.take().expect("receive is complete");
                    state.recv_message_id = state.recv_message_id.wrapping_add(1);
                    Ok(Some(std::mem::take(&mut done.bytes)))
                } else {
                    Ok(None)
                }
            }
            RECORD_FIN => {
                if !payload.is_empty()
                    || header.total_len != 0
                    || header.fragment_count != 1
                    || header.fragment_index != 0
                {
                    return Err(Error::Noise("malformed FIN record"));
                }
                let mut state = lock_state(&self.state);
                if state.receive.is_some() {
                    return Err(Error::Noise("FIN arrived before the final fragment"));
                }
                if header.message_id != state.recv_message_id {
                    return Err(Error::Noise("FIN record sequence mismatch"));
                }
                state.recv_fin = true;
                Err(Error::Io(std::io::Error::from(ErrorKind::UnexpectedEof)))
            }
            _ => Err(Error::Noise("unknown record kind")),
        }
    }

    pub fn finish(&self) -> Result<()> {
        let _send = self
            .send_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        {
            let state = lock_state(&self.state);
            if state.send_fin {
                return Ok(());
            }
            if state.failed {
                return Err(Error::Noise("session is failed"));
            }
        }
        let message_id = lock_state(&self.state).send_message_id;
        let header = RecordHeader {
            kind: RECORD_FIN,
            message_id,
            fragment_index: 0,
            fragment_count: 1,
            total_len: 0,
            payload_len: 0,
        };
        let mut ciphertext = self.seal_record(header, &[])?;
        let sent = self
            .wire
            .send_plain_timeout(&ciphertext, Duration::from_secs(1));
        ciphertext.zeroize();
        sent?;
        {
            let mut state = lock_state(&self.state);
            state.send_message_id = state.send_message_id.wrapping_add(1);
            state.send_fin = true;
        }
        self.wire.flush_plain()
    }

    pub fn close(&self) {
        // Shutdown must never be held hostage by a full peer or a stalled
        // writer. If another thread is already inside send(), abort the
        // session immediately; the transport close will wake that writer.
        // Otherwise try to enqueue FIN with a short bound and leave the final
        // flush to the transport's own bounded close path.
        let send = match self.send_lock.try_lock() {
            Ok(send) => send,
            Err(std::sync::TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => {
                lock_state(&self.state).failed = true;
                return;
            }
        };
        let _send = send;
        {
            let state = lock_state(&self.state);
            if state.send_fin || state.failed {
                return;
            }
        }
        let message_id = lock_state(&self.state).send_message_id;
        let header = RecordHeader {
            kind: RECORD_FIN,
            message_id,
            fragment_index: 0,
            fragment_count: 1,
            total_len: 0,
            payload_len: 0,
        };
        if let Ok(mut ciphertext) = self.seal_record(header, &[]) {
            let _ = self
                .wire
                .send_plain_timeout(&ciphertext, Duration::from_millis(250));
            ciphertext.zeroize();
        }
        let mut state = lock_state(&self.state);
        state.send_message_id = state.send_message_id.wrapping_add(1);
        state.send_fin = true;
    }

    pub fn remote_finished(&self) -> bool {
        lock_state(&self.state).recv_fin
    }

    #[cfg(test)]
    fn send_arbitrary_for_test(&self, header: RecordHeader, payload: &[u8]) -> Result<()> {
        self.send_record(header, payload)
    }
}

fn fragment_count(len: usize, record_payload: usize) -> u32 {
    if len == 0 {
        1
    } else {
        len.div_ceil(record_payload) as u32
    }
}

fn lock_state(state: &Mutex<NoiseState>) -> std::sync::MutexGuard<'_, NoiseState> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn params() -> Result<NoiseParams> {
    SUITE
        .parse()
        .map_err(|_| Error::Noise("noise suite is unavailable"))
}

fn build_with_prologue(intro: &NoiseIntro, initiator: bool) -> Result<HandshakeState> {
    let prologue = intro.prologue();
    let builder = Builder::new(params()?)
        .prologue(&prologue)
        .map_err(|_| Error::Noise("noise prologue setup failed"))?;
    if initiator {
        builder
            .build_initiator()
            .map_err(|_| Error::Noise("noise initiator setup failed"))
    } else {
        builder
            .build_responder()
            .map_err(|_| Error::Noise("noise responder setup failed"))
    }
}

/// Run the initiating side after the transport route is established.
pub(crate) fn handshake_initiator(
    wire: Arc<dyn NoiseWire>,
    intro: NoiseIntro,
    record_payload: usize,
    record_max: usize,
) -> Result<NoiseSession> {
    if intro.role != NoiseRole::Initiator {
        return Err(Error::Noise("initiator intro role mismatch"));
    }
    let mut handshake = build_with_prologue(&intro, true)?;

    wire.send_plain(&intro.encode())?;
    let mut message = [0u8; 128];
    let written = handshake
        .write_message(&[], &mut message)
        .map_err(|_| Error::Noise("noise first message failed"))?;
    wire.send_plain(&message[..written])?;
    wire.flush_plain()?;
    message.zeroize();

    let mut reply = wire.recv_plain_timeout(NOISE_HANDSHAKE_TIMEOUT)?;
    let mut plaintext = vec![0u8; reply.len()];
    let opened = handshake
        .read_message(&reply, &mut plaintext)
        .map_err(|_| Error::Noise("noise final handshake verification failed"))?;
    reply.zeroize();
    plaintext.zeroize();
    if opened != 0 {
        return Err(Error::Noise(
            "noise final handshake carried unexpected data",
        ));
    }

    let transport = handshake
        .into_transport_mode()
        .map_err(|_| Error::Noise("noise initiator transport setup failed"))?;
    Ok(NoiseSession::new(
        wire,
        transport,
        record_payload,
        record_max,
    ))
}

/// Run the responding side after reading the fixed intro from the wire.
pub(crate) fn handshake_responder(
    wire: Arc<dyn NoiseWire>,
    intro_bytes: &[u8],
    expected_transport: NoiseTransport,
    expected_kind: NoiseKind,
    record_payload: usize,
    record_max: usize,
) -> Result<NoiseSession> {
    let intro = NoiseIntro::decode(intro_bytes)?;
    intro.validate(expected_transport, expected_kind, NoiseRole::Responder)?;

    let mut handshake = build_with_prologue(&intro, false)?;

    let mut first = wire.recv_plain_timeout(NOISE_HANDSHAKE_TIMEOUT)?;
    let mut plaintext = vec![0u8; first.len()];
    let opened = handshake
        .read_message(&first, &mut plaintext)
        .map_err(|_| Error::Noise("noise first handshake verification failed"))?;
    first.zeroize();
    plaintext.zeroize();
    if opened != 0 {
        return Err(Error::Noise(
            "noise first handshake carried unexpected data",
        ));
    }

    let mut reply = [0u8; 128];
    let written = handshake
        .write_message(&[], &mut reply)
        .map_err(|_| Error::Noise("noise final message failed"))?;
    wire.send_plain(&reply[..written])?;
    wire.flush_plain()?;
    reply.zeroize();

    let transport = handshake
        .into_transport_mode()
        .map_err(|_| Error::Noise("noise responder transport setup failed"))?;
    Ok(NoiseSession::new(
        wire,
        transport,
        record_payload,
        record_max,
    ))
}

#[cfg(test)]
struct MemoryQueue {
    values: Mutex<VecDeque<Vec<u8>>>,
    ready: Condvar,
}

#[cfg(test)]
impl MemoryQueue {
    fn new() -> Self {
        Self {
            values: Mutex::new(VecDeque::new()),
            ready: Condvar::new(),
        }
    }

    fn push(&self, value: Vec<u8>) {
        self.values
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push_back(value);
        self.ready.notify_all();
    }

    fn len(&self) -> usize {
        self.values
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }

    fn pop_timeout(&self, timeout: Duration) -> Result<Vec<u8>> {
        let deadline = Instant::now() + timeout;
        let mut values = self
            .values
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        loop {
            if let Some(value) = values.pop_front() {
                return Ok(value);
            }
            let now = Instant::now();
            if now >= deadline {
                return Err(Error::Timeout);
            }
            let (next, _) = self
                .ready
                .wait_timeout(values, deadline.saturating_duration_since(now))
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            values = next;
        }
    }
}

#[cfg(test)]
struct MemoryWire {
    sent: Arc<MemoryQueue>,
    received: Arc<MemoryQueue>,
    flush_after_send: bool,
    flushes: Arc<AtomicUsize>,
}

#[cfg(test)]
struct SendGate {
    armed: std::sync::atomic::AtomicBool,
    entered: Mutex<Option<std::sync::mpsc::SyncSender<()>>>,
    release: Mutex<std::sync::mpsc::Receiver<()>>,
}

#[cfg(test)]
struct GatedWire {
    inner: MemoryWire,
    gate: Arc<SendGate>,
}

#[cfg(test)]
impl NoiseWire for GatedWire {
    fn send_plain(&self, payload: &[u8]) -> Result<()> {
        self.inner.send_plain(payload)?;
        if self.gate.armed.load(std::sync::atomic::Ordering::Acquire) {
            if let Some(entered) = self
                .gate
                .entered
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .take()
            {
                let _ = entered.send(());
            }
            let _ = self
                .gate
                .release
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .recv_timeout(Duration::from_secs(5));
        }
        Ok(())
    }

    fn recv_plain(&self) -> Result<Vec<u8>> {
        self.inner.recv_plain()
    }

    fn recv_plain_timeout(&self, timeout: Duration) -> Result<Vec<u8>> {
        self.inner.recv_plain_timeout(timeout)
    }

    fn flush_plain(&self) -> Result<()> {
        self.inner.flush_plain()
    }
}

#[cfg(test)]
impl NoiseWire for MemoryWire {
    fn send_plain(&self, payload: &[u8]) -> Result<()> {
        self.sent.push(payload.to_vec());
        Ok(())
    }

    fn recv_plain(&self) -> Result<Vec<u8>> {
        self.received.pop_timeout(Duration::from_secs(10))
    }

    fn recv_plain_timeout(&self, timeout: Duration) -> Result<Vec<u8>> {
        self.received.pop_timeout(timeout)
    }

    fn flush_after_send(&self) -> bool {
        self.flush_after_send
    }

    fn flush_plain(&self) -> Result<()> {
        self.flushes.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }
}

#[cfg(test)]
fn memory_pair() -> (Arc<MemoryWire>, Arc<MemoryWire>) {
    memory_pair_with_flush_policy(true)
}

#[cfg(test)]
fn memory_pair_with_flush_policy(flush_after_send: bool) -> (Arc<MemoryWire>, Arc<MemoryWire>) {
    let client_to_server = Arc::new(MemoryQueue::new());
    let server_to_client = Arc::new(MemoryQueue::new());
    let client_flushes = Arc::new(AtomicUsize::new(0));
    let server_flushes = Arc::new(AtomicUsize::new(0));
    (
        Arc::new(MemoryWire {
            sent: client_to_server.clone(),
            received: server_to_client.clone(),
            flush_after_send,
            flushes: client_flushes,
        }),
        Arc::new(MemoryWire {
            sent: server_to_client,
            received: client_to_server,
            flush_after_send,
            flushes: server_flushes,
        }),
    )
}

#[cfg(test)]
fn test_sessions(
    payload: usize,
    transport: NoiseTransport,
    kind: NoiseKind,
) -> (NoiseSession, NoiseSession, Arc<MemoryWire>, Arc<MemoryWire>) {
    test_sessions_with_flush_policy(payload, transport, kind, true)
}

#[cfg(test)]
fn test_sessions_with_flush_policy(
    payload: usize,
    transport: NoiseTransport,
    kind: NoiseKind,
    flush_after_send: bool,
) -> (NoiseSession, NoiseSession, Arc<MemoryWire>, Arc<MemoryWire>) {
    let (client_wire, server_wire) = memory_pair_with_flush_policy(flush_after_send);
    let client_arc: Arc<dyn NoiseWire> = client_wire.clone();
    let server_arc: Arc<dyn NoiseWire> = server_wire.clone();
    let intro = NoiseIntro::new(transport, kind);
    let client = std::thread::spawn(move || {
        handshake_initiator(client_arc, intro, payload, payload + 46).unwrap()
    });
    let intro_bytes = server_wire
        .recv_plain_timeout(NOISE_HANDSHAKE_TIMEOUT)
        .unwrap();
    let server = handshake_responder(
        server_arc,
        &intro_bytes,
        transport,
        kind,
        payload,
        payload + 46,
    )
    .unwrap();
    let client = client.join().unwrap();
    (client, server, client_wire, server_wire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intro_has_a_fixed_versioned_layout_and_rejects_mismatch() {
        let intro = NoiseIntro::with_session_id(
            NoiseTransport::Tcp,
            NoiseKind::Tunnel { port: 4567 },
            0x0102030405060708,
        );
        let encoded = intro.encode();
        assert_eq!(encoded.len(), INTRO_LEN);
        assert_eq!(&encoded[..8], b"HRNNOIS1");
        assert_eq!(NoiseIntro::decode(&encoded).unwrap(), intro);
        assert!(matches!(
            intro.validate(NoiseTransport::Http, intro.kind(), NoiseRole::Responder,),
            Err(Error::Noise(_))
        ));
        assert!(matches!(
            intro.validate(intro.transport(), NoiseKind::Main, NoiseRole::Responder,),
            Err(Error::Noise(_))
        ));
        assert!(
            intro
                .validate(intro.transport(), intro.kind(), NoiseRole::Initiator)
                .is_err()
        );

        let mut bad = encoded;
        bad[8] = 2;
        assert!(matches!(NoiseIntro::decode(&bad), Err(Error::Noise(_))));

        let mut bad_role = encoded;
        bad_role[12..14].copy_from_slice(&0u16.to_be_bytes());
        assert!(matches!(
            NoiseIntro::decode(&bad_role),
            Err(Error::Noise(_))
        ));
    }

    #[test]
    fn initiator_intro_binds_the_wire_peer_role() {
        let intro = NoiseIntro::new(NoiseTransport::Tcp, NoiseKind::Main);
        let encoded = intro.encode();
        assert_eq!(encoded.len(), INTRO_LEN);
        assert_eq!(&encoded[12..14], &[0, 1]);
    }

    #[test]
    fn empty_and_boundary_sized_messages_round_trip() {
        let (client, server, _, _) = test_sessions(4096, NoiseTransport::Tcp, NoiseKind::Main);
        for payload in [
            Vec::new(),
            vec![0x11],
            vec![0x22; 4096],
            vec![0x33; 4097],
            vec![0x44; 32 * 1024],
        ] {
            client.send(&payload).unwrap();
            assert_eq!(server.recv().unwrap(), payload);
        }
        client.send(&vec![0x55; 32 * 1024]).unwrap();
        assert_eq!(server.recv().unwrap(), vec![0x55; 32 * 1024]);
    }

    #[test]
    fn async_carrier_skips_per_message_flush_and_finish_still_flushes() {
        let (client, server, client_wire, _) = test_sessions_with_flush_policy(
            STREAM_RECORD_PAYLOAD,
            NoiseTransport::Http,
            NoiseKind::Main,
            false,
        );
        let initial_flushes = client_wire.flushes.load(Ordering::Acquire);

        client.send(b"first").unwrap();
        client.send(b"second").unwrap();
        assert_eq!(
            client_wire.flushes.load(Ordering::Acquire),
            initial_flushes,
            "an actor-owned carrier must not wait on a flush after every message"
        );
        assert_eq!(server.recv().unwrap(), b"first");
        assert_eq!(server.recv().unwrap(), b"second");

        client.finish().unwrap();
        assert!(
            client_wire.flushes.load(Ordering::Acquire) > initial_flushes,
            "FIN must force the queued records to the carrier"
        );
    }

    #[test]
    fn four_eight_sixteen_and_thirty_two_kib_records_round_trip() {
        for record_payload in [4 * 1024, 8 * 1024, 16 * 1024, 32 * 1024] {
            let (client, server, _, _) =
                test_sessions(record_payload, NoiseTransport::Tcp, NoiseKind::Main);
            for payload_len in [
                record_payload - 1,
                record_payload,
                record_payload + 1,
                record_payload * 3 + 17,
            ] {
                let payload = (0..payload_len)
                    .map(|index| (index % 251) as u8)
                    .collect::<Vec<_>>();
                client.send(&payload).unwrap();
                assert_eq!(server.recv().unwrap(), payload);
            }
        }
    }

    #[test]
    fn stream_record_limit_is_explicitly_bounded_and_within_noise_plaintext() {
        let stream_record = STREAM_RECORD_PAYLOAD;
        let plaintext_limit = NOISE_MAX_PLAINTEXT_BYTES;
        assert_eq!(STREAM_RECORD_PAYLOAD_BYTES, 60 * 1024);
        assert_eq!(STREAM_RECORD_PAYLOAD, STREAM_RECORD_PAYLOAD_BYTES);
        assert_eq!(STREAM_RECORD_MAX, 60 * 1024 + 30 + 16);
        assert_eq!(UDP_RECORD_PAYLOAD, 8 * 1024);
        assert_eq!(UDP_RECORD_MAX, 8 * 1024 + 30 + 16);
        assert_eq!(NOISE_MAX_PLAINTEXT_BYTES, 65_519);
        assert!(
            stream_record + RECORD_HEADER_LEN <= plaintext_limit,
            "the stream record plaintext must fit one Noise message"
        );
        assert!(
            stream_record <= 60 * 1024,
            "the dispatch-approved stream record cap is exactly 60 KiB"
        );
    }

    #[test]
    fn sixty_kib_record_and_one_byte_overflow_fragment_exactly() {
        let (client, server, client_wire, _) =
            test_sessions(STREAM_RECORD_PAYLOAD, NoiseTransport::Tcp, NoiseKind::Main);

        client.send(&vec![0x61; STREAM_RECORD_PAYLOAD]).unwrap();
        assert_eq!(
            client_wire.sent.len(),
            1,
            "exactly 60 KiB must produce exactly one record"
        );
        assert_eq!(server.recv().unwrap(), vec![0x61; STREAM_RECORD_PAYLOAD]);
        assert_eq!(client_wire.sent.len(), 0);

        let payload = (0..STREAM_RECORD_PAYLOAD + 1)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        let expected = payload.clone();
        client.send(&payload).unwrap();
        assert_eq!(
            client_wire.sent.len(),
            2,
            "60 KiB + 1 must produce exactly two records"
        );
        assert_eq!(server.recv().unwrap(), expected);
        assert_eq!(client_wire.sent.len(), 0);
    }

    #[test]
    fn sixty_kib_reassembly_only_preallocates_authenticated_fragments() {
        let (_, server, _, _) =
            test_sessions(STREAM_RECORD_PAYLOAD, NoiseTransport::Tcp, NoiseKind::Main);
        let header = RecordHeader {
            kind: RECORD_DATA,
            message_id: 0,
            fragment_index: 0,
            fragment_count: fragment_count(MAX_NOISE_MESSAGE, STREAM_RECORD_PAYLOAD),
            total_len: MAX_NOISE_MESSAGE as u32,
            payload_len: STREAM_RECORD_PAYLOAD as u32,
        };
        let mut plaintext = header.encode().to_vec();
        plaintext.extend(std::iter::repeat_n(0x7b, STREAM_RECORD_PAYLOAD));
        assert!(server.consume_plaintext(&plaintext).unwrap().is_none());

        let state = lock_state(&server.state);
        let buffer = state
            .receive
            .as_ref()
            .expect("the authenticated first fragment created a receive buffer");
        assert_eq!(buffer.bytes.len(), STREAM_RECORD_PAYLOAD);
        assert!(
            buffer.bytes.capacity() <= STREAM_RECORD_PAYLOAD,
            "reassembly preallocated the unauthenticated 10 MiB total length"
        );
    }

    #[test]
    fn ten_mebibyte_message_is_fragmented_and_reassembled() {
        let (client, server, _, _) =
            test_sessions(STREAM_RECORD_PAYLOAD, NoiseTransport::Tcp, NoiseKind::Main);
        let payload = (0..MAX_NOISE_MESSAGE)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        assert_eq!(payload.len(), 10 * 1024 * 1024);
        let expected = payload.clone();
        let sender = std::thread::spawn(move || client.send(&payload));
        assert_eq!(server.recv().unwrap(), expected);
        sender.join().unwrap().unwrap();
    }

    #[test]
    fn one_byte_over_ten_mebibytes_is_rejected_without_partial_state() {
        let (client, server, client_wire, _) =
            test_sessions(STREAM_RECORD_PAYLOAD, NoiseTransport::Tcp, NoiseKind::Main);
        assert!(matches!(
            client.send(&vec![0x5a; MAX_NOISE_MESSAGE + 1]),
            Err(Error::PacketTooLarge(size)) if size == MAX_NOISE_MESSAGE + 1
        ));
        assert!(
            client_wire
                .sent
                .pop_timeout(Duration::from_millis(50))
                .is_err(),
            "an oversized logical message emitted a partial record"
        );

        client.send(b"after-oversize").unwrap();
        assert_eq!(server.recv().unwrap(), b"after-oversize");
    }

    #[test]
    fn reassembly_buffer_grows_only_with_authenticated_fragments() {
        let (_, server, _, _) = test_sessions(4096, NoiseTransport::Tcp, NoiseKind::Main);
        let one_fragment = 4096;
        let header = RecordHeader {
            kind: RECORD_DATA,
            message_id: 0,
            fragment_index: 0,
            fragment_count: fragment_count(MAX_NOISE_MESSAGE, one_fragment),
            total_len: MAX_NOISE_MESSAGE as u32,
            payload_len: one_fragment as u32,
        };
        let mut plaintext = header.encode().to_vec();
        plaintext.extend(std::iter::repeat_n(0x7b, one_fragment));
        assert!(server.consume_plaintext(&plaintext).unwrap().is_none());

        let state = lock_state(&server.state);
        let buffer = state
            .receive
            .as_ref()
            .expect("first authenticated fragment created the reassembly buffer");
        assert_eq!(buffer.bytes.len(), one_fragment);
        assert!(
            buffer.bytes.capacity() <= one_fragment,
            "reassembly buffer preallocated the unauthenticated logical length"
        );
    }

    #[test]
    fn role_mismatch_is_rejected_for_tcp_http_and_udp() {
        for transport in [
            NoiseTransport::Tcp,
            NoiseTransport::Http,
            NoiseTransport::Udp,
        ] {
            let (client_wire, server_wire) = memory_pair();
            let intro = NoiseIntro::new(transport, NoiseKind::Main);
            let client_arc: Arc<dyn NoiseWire> = client_wire;
            let sender = std::thread::spawn(move || {
                let _ = handshake_initiator(
                    client_arc,
                    intro,
                    STREAM_RECORD_PAYLOAD,
                    STREAM_RECORD_MAX,
                );
            });

            let mut intro_bytes = server_wire
                .recv_plain_timeout(NOISE_HANDSHAKE_TIMEOUT)
                .unwrap();
            intro_bytes[12..14].copy_from_slice(&2u16.to_be_bytes());
            let reply_queue = server_wire.sent.clone();
            let server_arc: Arc<dyn NoiseWire> = server_wire;
            let result = handshake_responder(
                server_arc,
                &intro_bytes,
                transport,
                NoiseKind::Main,
                STREAM_RECORD_PAYLOAD,
                STREAM_RECORD_MAX,
            );
            assert!(
                matches!(
                    result,
                    Err(Error::Noise(message)) if message == "intro peer role mismatch"
                ),
                "role mismatch was not rejected for {transport:?}"
            );

            reply_queue.push(vec![0u8; 64]);
            sender.join().unwrap();
        }
    }

    #[test]
    fn prologue_mismatch_fails_for_tcp_http_and_udp() {
        for transport in [
            NoiseTransport::Tcp,
            NoiseTransport::Http,
            NoiseTransport::Udp,
        ] {
            let (client_wire, server_wire) = memory_pair();
            let client_arc: Arc<dyn NoiseWire> = client_wire.clone();
            let client_intro =
                NoiseIntro::with_session_id(transport, NoiseKind::Main, 0x1111111111111111);
            let wire_intro =
                NoiseIntro::with_session_id(transport, NoiseKind::Main, 0x2222222222222222);
            let sender = std::thread::spawn(move || {
                let mut handshake = build_with_prologue(&client_intro, true).unwrap();
                client_arc.send_plain(&wire_intro.encode()).unwrap();
                let mut message = [0u8; 128];
                let written = handshake.write_message(&[], &mut message).unwrap();
                client_arc.send_plain(&message[..written]).unwrap();
                client_arc.flush_plain().unwrap();
                let reply = client_arc
                    .recv_plain_timeout(NOISE_HANDSHAKE_TIMEOUT)
                    .unwrap();
                let mut plaintext = vec![0u8; reply.len()];
                handshake.read_message(&reply, &mut plaintext).is_err()
            });

            let intro_bytes = server_wire
                .recv_plain_timeout(NOISE_HANDSHAKE_TIMEOUT)
                .unwrap();
            let server_arc: Arc<dyn NoiseWire> = server_wire;
            let _ = handshake_responder(
                server_arc,
                &intro_bytes,
                transport,
                NoiseKind::Main,
                STREAM_RECORD_PAYLOAD,
                STREAM_RECORD_MAX,
            );
            assert!(
                sender.join().unwrap(),
                "prologue mismatch was not rejected for {transport:?}"
            );
        }
    }

    #[test]
    fn tampering_and_out_of_order_records_fail_closed() {
        let (client, server, client_wire, server_wire) =
            test_sessions(STREAM_RECORD_PAYLOAD, NoiseTransport::Tcp, NoiseKind::Main);
        client.send(&vec![0x5a; STREAM_RECORD_PAYLOAD + 1]).unwrap();
        let mut first = client_wire
            .sent
            .pop_timeout(Duration::from_secs(1))
            .unwrap();
        let second = client_wire
            .sent
            .pop_timeout(Duration::from_secs(1))
            .unwrap();
        let middle = first.len() / 2;
        first[middle] ^= 0x80;
        server_wire.received.push(second);
        server_wire.received.push(first);
        assert!(matches!(server.recv(), Err(Error::Noise(_))));
    }

    #[test]
    fn replay_and_cross_session_injection_are_rejected() {
        let (client, server, client_wire, server_wire) =
            test_sessions(4096, NoiseTransport::Tcp, NoiseKind::Main);
        client.send(b"first").unwrap();
        let first = client_wire
            .sent
            .pop_timeout(Duration::from_secs(1))
            .unwrap();
        let replay = first.clone();
        server_wire.received.push(first);
        assert_eq!(server.recv().unwrap(), b"first");
        server_wire.received.push(replay);
        assert!(matches!(server.recv(), Err(Error::Noise(_))));

        let (other_client, _, other_wire, _) =
            test_sessions(4096, NoiseTransport::Tcp, NoiseKind::Main);
        other_client.send(b"other session").unwrap();
        let foreign = other_wire.sent.pop_timeout(Duration::from_secs(1)).unwrap();
        let (_, second_server, _, second_server_wire) =
            test_sessions(4096, NoiseTransport::Tcp, NoiseKind::Main);
        second_server_wire.received.push(foreign);
        assert!(matches!(second_server.recv(), Err(Error::Noise(_))));
    }

    #[test]
    fn oversized_and_bad_fragment_metadata_are_rejected() {
        let (client, server, _, _) = test_sessions(1024, NoiseTransport::Tcp, NoiseKind::Main);
        let header = RecordHeader {
            kind: RECORD_DATA,
            message_id: 0,
            fragment_index: 0,
            fragment_count: 1,
            total_len: (MAX_NOISE_MESSAGE + 1) as u32,
            payload_len: 0,
        };
        client.send_arbitrary_for_test(header, &[]).unwrap();
        assert!(matches!(
            server.recv(),
            Err(Error::PacketTooLarge(size)) if size == MAX_NOISE_MESSAGE + 1
        ));

        let (client, server, _, _) = test_sessions(1024, NoiseTransport::Tcp, NoiseKind::Main);
        let header = RecordHeader {
            kind: RECORD_DATA,
            message_id: 0,
            fragment_index: 1,
            fragment_count: 2,
            total_len: 10,
            payload_len: 0,
        };
        client.send_arbitrary_for_test(header, &[]).unwrap();
        assert!(matches!(server.recv(), Err(Error::Noise(_))));

        assert!(matches!(
            client.send(&vec![0u8; MAX_NOISE_MESSAGE + 1]),
            Err(Error::PacketTooLarge(size)) if size == MAX_NOISE_MESSAGE + 1
        ));
    }

    #[test]
    fn truncated_and_duplicated_ciphertext_is_rejected() {
        let (client, server, client_wire, server_wire) =
            test_sessions(1024, NoiseTransport::Tcp, NoiseKind::Main);
        client.send(b"payload").unwrap();
        let mut ciphertext = client_wire
            .sent
            .pop_timeout(Duration::from_secs(1))
            .unwrap();
        ciphertext.pop();
        server_wire.received.push(ciphertext);
        assert!(matches!(server.recv(), Err(Error::Noise(_))));

        let (client, server, client_wire, server_wire) =
            test_sessions(1024, NoiseTransport::Tcp, NoiseKind::Main);
        client.send(b"payload").unwrap();
        let ciphertext = client_wire
            .sent
            .pop_timeout(Duration::from_secs(1))
            .unwrap();
        server_wire.received.push(ciphertext.clone());
        assert_eq!(server.recv().unwrap(), b"payload");
        server_wire.received.push(ciphertext);
        assert!(matches!(server.recv(), Err(Error::Noise(_))));
    }

    #[test]
    fn authenticated_fin_is_single_use_and_does_not_hide_empty_messages() {
        let (client, server, _, _) = test_sessions(1024, NoiseTransport::Tcp, NoiseKind::Main);
        client.send(&[]).unwrap();
        assert_eq!(server.recv().unwrap(), Vec::<u8>::new());
        client.finish().unwrap();
        assert!(matches!(
            server.recv(),
            Err(Error::Io(ref error)) if error.kind() == ErrorKind::UnexpectedEof
        ));
        assert!(server.remote_finished());
        assert!(matches!(server.recv(), Err(Error::Io(_))));
        server.send(b"reverse-direction").unwrap();
        assert_eq!(client.recv().unwrap(), b"reverse-direction");
        assert!(client.finish().is_ok());
        assert!(client.send(b"after-fin").is_err());
    }

    #[test]
    fn every_handshake_and_intro_generates_distinct_ephemeral_material() {
        let (client1, _, wire1, _) = test_sessions(1024, NoiseTransport::Http, NoiseKind::Main);
        let (client2, _, wire2, _) = test_sessions(1024, NoiseTransport::Http, NoiseKind::Main);
        client1.send(b"same").unwrap();
        client2.send(b"same").unwrap();
        let first = wire1.sent.pop_timeout(Duration::from_secs(1)).unwrap();
        let second = wire2.sent.pop_timeout(Duration::from_secs(1)).unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn intro_purpose_must_match_the_transport_session() {
        let (client_wire, server_wire) = memory_pair();
        let intro = NoiseIntro::new(NoiseTransport::Tcp, NoiseKind::Tunnel { port: 1234 });
        let client_arc: Arc<dyn NoiseWire> = client_wire;
        let intro_clone = intro;
        let sender = std::thread::spawn(move || {
            let _ = handshake_initiator(
                client_arc,
                intro_clone,
                STREAM_RECORD_PAYLOAD,
                STREAM_RECORD_MAX,
            );
        });
        let first = server_wire
            .recv_plain_timeout(NOISE_HANDSHAKE_TIMEOUT)
            .unwrap();
        let reply_queue = server_wire.sent.clone();
        let server_arc: Arc<dyn NoiseWire> = server_wire;
        let result = handshake_responder(
            server_arc,
            &first,
            NoiseTransport::Tcp,
            NoiseKind::Main,
            STREAM_RECORD_PAYLOAD,
            STREAM_RECORD_MAX,
        );
        assert!(matches!(result, Err(Error::Noise(_))));
        reply_queue.push(vec![0u8; 64]);
        sender.join().unwrap();
    }

    #[test]
    fn close_does_not_wait_for_an_inflight_sender() {
        let client_to_server = Arc::new(MemoryQueue::new());
        let server_to_client = Arc::new(MemoryQueue::new());
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let gate = Arc::new(SendGate {
            armed: std::sync::atomic::AtomicBool::new(false),
            entered: Mutex::new(None),
            release: Mutex::new(release_rx),
        });
        let client_wire = Arc::new(GatedWire {
            inner: MemoryWire {
                sent: client_to_server.clone(),
                received: server_to_client.clone(),
                flush_after_send: true,
                flushes: Arc::new(AtomicUsize::new(0)),
            },
            gate: gate.clone(),
        });
        let server_wire = Arc::new(GatedWire {
            inner: MemoryWire {
                sent: server_to_client,
                received: client_to_server,
                flush_after_send: true,
                flushes: Arc::new(AtomicUsize::new(0)),
            },
            gate: gate.clone(),
        });

        let intro = NoiseIntro::new(NoiseTransport::Tcp, NoiseKind::Main);
        let client_arc: Arc<dyn NoiseWire> = client_wire.clone();
        let client_handshake = std::thread::spawn(move || {
            handshake_initiator(client_arc, intro, STREAM_RECORD_PAYLOAD, STREAM_RECORD_MAX)
                .unwrap()
        });
        let intro_bytes = server_wire
            .recv_plain_timeout(NOISE_HANDSHAKE_TIMEOUT)
            .unwrap();
        let server_arc: Arc<dyn NoiseWire> = server_wire;
        let _server = handshake_responder(
            server_arc,
            &intro_bytes,
            NoiseTransport::Tcp,
            NoiseKind::Main,
            STREAM_RECORD_PAYLOAD,
            STREAM_RECORD_MAX,
        )
        .unwrap();
        let client = Arc::new(client_handshake.join().unwrap());

        let (entered_tx, entered_rx) = std::sync::mpsc::sync_channel(1);
        *gate
            .entered
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(entered_tx);
        gate.armed.store(true, std::sync::atomic::Ordering::Release);

        let sender_client = client.clone();
        let sender =
            std::thread::spawn(move || sender_client.send(&vec![0x5a; STREAM_RECORD_PAYLOAD + 1]));
        entered_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("sender did not reach the wire");

        let started = Instant::now();
        client.close();
        assert!(
            started.elapsed() < Duration::from_millis(500),
            "close waited for the in-flight sender"
        );
        release_tx.send(()).unwrap();
        assert!(matches!(sender.join().unwrap(), Err(Error::Noise(_))));
    }
}
