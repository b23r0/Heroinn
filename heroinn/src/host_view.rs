//! Host-platform presentation helpers.
//!
//! Platform icons are drawn with egui's painter so they follow the active
//! theme, DPI scale, and font fallback without shipping platform artwork.

use eframe::egui::{Align2, Color32, FontId, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, pos2};
use heroinn_util::packet::HostInfo;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostPlatform {
    Linux,
    Windows,
    Bsd,
    Unknown,
}

impl HostPlatform {
    pub fn from_info(info: &HostInfo) -> Self {
        Self::from_parts(info.platform_id.as_deref(), &info.os)
    }

    pub fn from_parts(platform_id: Option<&str>, os: &str) -> Self {
        if let Some(platform_id) = platform_id.filter(|value| !value.trim().is_empty())
            && let Some(platform) = platform_from_tokens(platform_id)
        {
            return platform;
        }

        platform_from_tokens(os).unwrap_or(Self::Unknown)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Linux => "Linux",
            Self::Windows => "Windows",
            Self::Bsd => "BSD",
            Self::Unknown => "Unknown",
        }
    }

    /// Accent color used by the vector icon and tests.
    pub fn accent(self, dark: bool) -> Color32 {
        match (self, dark) {
            (Self::Linux, true) => Color32::from_rgb(86, 227, 137),
            (Self::Linux, false) => Color32::from_rgb(21, 128, 61),
            (Self::Windows, true) => Color32::from_rgb(86, 166, 255),
            (Self::Windows, false) => Color32::from_rgb(0, 95, 184),
            (Self::Bsd, true) => Color32::from_rgb(255, 123, 114),
            (Self::Bsd, false) => Color32::from_rgb(190, 52, 47),
            (Self::Unknown, true) => Color32::from_rgb(160, 174, 192),
            (Self::Unknown, false) => Color32::from_rgb(82, 98, 118),
        }
    }
}

pub const HOST_ICON_COLUMN_WIDTH: f32 = 48.0;
pub const HOST_ICON_SIZE: f32 = 24.0;
pub const HOST_ROW_HEIGHT: f32 = 30.0;

#[derive(Clone, Copy, Debug)]
pub struct HostColumnSpec {
    pub title: &'static str,
    pub initial: f32,
    pub minimum: f32,
}

pub const HOST_COLUMNS: [HostColumnSpec; 11] = [
    HostColumnSpec {
        title: "",
        initial: HOST_ICON_COLUMN_WIDTH,
        minimum: HOST_ICON_COLUMN_WIDTH,
    },
    HostColumnSpec {
        title: "Peer IP",
        initial: 145.0,
        minimum: 90.0,
    },
    HostColumnSpec {
        title: "Whoami",
        initial: 100.0,
        minimum: 70.0,
    },
    HostColumnSpec {
        title: "Host Name",
        initial: 135.0,
        minimum: 90.0,
    },
    HostColumnSpec {
        title: "OS",
        initial: 180.0,
        minimum: 100.0,
    },
    HostColumnSpec {
        title: "Protocol",
        initial: 85.0,
        minimum: 70.0,
    },
    HostColumnSpec {
        title: "Transfer Rate",
        initial: 105.0,
        minimum: 80.0,
    },
    HostColumnSpec {
        title: "Last Heartbeat",
        initial: 115.0,
        minimum: 90.0,
    },
    HostColumnSpec {
        title: "System Proxy",
        initial: 170.0,
        minimum: 130.0,
    },
    HostColumnSpec {
        title: "SOCKS5 Proxy",
        initial: 170.0,
        minimum: 130.0,
    },
    HostColumnSpec {
        title: "Remark",
        initial: 180.0,
        minimum: 120.0,
    },
];

pub fn host_table_min_width(column_spacing: f32) -> f32 {
    HOST_COLUMNS
        .iter()
        .map(|column| column.initial)
        .sum::<f32>()
        + column_spacing * (HOST_COLUMNS.len().saturating_sub(1) as f32)
}

fn platform_from_tokens(value: &str) -> Option<HostPlatform> {
    for token in value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
    {
        match token.to_ascii_lowercase().as_str() {
            "linux" | "ubuntu" | "debian" | "fedora" | "rhel" | "redhat" | "arch" | "alpine"
            | "suse" | "opensuse" | "gentoo" | "mint" | "kali" | "manjaro" | "nixos" | "rocky"
            | "alma" | "almalinux" => {
                return Some(HostPlatform::Linux);
            }
            "windows" => return Some(HostPlatform::Windows),
            "bsd" | "freebsd" | "openbsd" | "netbsd" | "dragonfly" | "hardenedbsd"
            | "midnightbsd" => return Some(HostPlatform::Bsd),
            _ => {}
        }
    }
    None
}

fn icon_fill(accent: Color32, dark: bool) -> Color32 {
    let alpha = if dark { 54 } else { 30 };
    Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), alpha)
}

fn draw_linux(ui: &Ui, rect: Rect, accent: Color32) {
    let painter = ui.painter();
    let frame = rect.shrink(2.0);
    painter.rect_filled(frame, 3.0, icon_fill(accent, ui.visuals().dark_mode));
    painter.line_segment(
        [frame.left_top(), frame.right_top()],
        Stroke::new(1.8, accent),
    );
    painter.line_segment(
        [frame.right_top(), frame.right_bottom()],
        Stroke::new(1.8, accent),
    );
    painter.line_segment(
        [frame.right_bottom(), frame.left_bottom()],
        Stroke::new(1.8, accent),
    );
    painter.line_segment(
        [frame.left_bottom(), frame.left_top()],
        Stroke::new(1.8, accent),
    );
    painter.text(
        frame.center() + Vec2::new(-1.0, -3.0),
        Align2::LEFT_CENTER,
        ">",
        FontId::monospace(11.0),
        accent,
    );
    painter.text(
        frame.center() + Vec2::new(4.0, 5.0),
        Align2::CENTER_CENTER,
        "_",
        FontId::monospace(10.0),
        accent,
    );
}

fn draw_windows(ui: &Ui, rect: Rect, accent: Color32) {
    let painter = ui.painter();
    let frame = rect.shrink(3.0);
    painter.rect_filled(frame, 1.0, icon_fill(accent, ui.visuals().dark_mode));
    let gap = 1.5;
    let pane = (frame.width() - gap) / 2.0;
    for (x, y) in [
        (0.0, 0.0),
        (pane + gap, 0.0),
        (0.0, pane + gap),
        (pane + gap, pane + gap),
    ] {
        let pane_rect =
            Rect::from_min_size(pos2(frame.left() + x, frame.top() + y), Vec2::splat(pane));
        painter.rect_filled(pane_rect, 1.0, accent);
    }
}

fn draw_bsd(ui: &Ui, rect: Rect, accent: Color32) {
    let painter = ui.painter();
    let center = rect.center();
    let shield = vec![
        pos2(center.x, rect.top() + 2.0),
        pos2(rect.right() - 3.0, rect.top() + 5.0),
        pos2(rect.right() - 5.0, rect.bottom() - 6.0),
        pos2(center.x, rect.bottom() - 2.0),
        pos2(rect.left() + 5.0, rect.bottom() - 6.0),
        pos2(rect.left() + 3.0, rect.top() + 5.0),
    ];
    painter.add(Shape::convex_polygon(
        shield,
        icon_fill(accent, ui.visuals().dark_mode),
        Stroke::new(1.8, accent),
    ));
    painter.text(
        center,
        Align2::CENTER_CENTER,
        "B",
        FontId::proportional(12.0),
        accent,
    );
}

fn draw_unknown(ui: &Ui, rect: Rect, accent: Color32) {
    let painter = ui.painter();
    let screen = Rect::from_min_max(
        pos2(rect.left() + 3.0, rect.top() + 4.0),
        pos2(rect.right() - 3.0, rect.bottom() - 6.0),
    );
    painter.rect_filled(screen, 2.0, icon_fill(accent, ui.visuals().dark_mode));
    painter.line_segment(
        [screen.left_top(), screen.right_top()],
        Stroke::new(1.5, accent),
    );
    painter.line_segment(
        [screen.right_top(), screen.right_bottom()],
        Stroke::new(1.5, accent),
    );
    painter.line_segment(
        [screen.right_bottom(), screen.left_bottom()],
        Stroke::new(1.5, accent),
    );
    painter.line_segment(
        [screen.left_bottom(), screen.left_top()],
        Stroke::new(1.5, accent),
    );
    let x = center_x(rect);
    painter.line_segment(
        [pos2(x, rect.bottom() - 6.0), pos2(x, rect.bottom() - 2.0)],
        Stroke::new(1.5, accent),
    );
    painter.text(
        screen.center(),
        Align2::CENTER_CENTER,
        "?",
        FontId::proportional(11.0),
        accent,
    );
}

fn center_x(rect: Rect) -> f32 {
    (rect.left() + rect.right()) / 2.0
}

pub fn draw_host_platform_icon(ui: &mut Ui, info: &HostInfo) -> Response {
    let platform = HostPlatform::from_info(info);
    let (response, _) = ui.allocate_painter(Vec2::splat(HOST_ICON_SIZE), Sense::hover());
    let rect = Rect::from_center_size(response.rect.center(), Vec2::splat(HOST_ICON_SIZE));
    let accent = platform.accent(ui.visuals().dark_mode);
    match platform {
        HostPlatform::Linux => draw_linux(ui, rect, accent),
        HostPlatform::Windows => draw_windows(ui, rect, accent),
        HostPlatform::Bsd => draw_bsd(ui, rect, accent),
        HostPlatform::Unknown => draw_unknown(ui, rect, accent),
    }
    let os = sanitize_tooltip_os(&info.os);
    response.on_hover_text(format!("{} — {}", platform.label(), os))
}

fn sanitize_tooltip_os(os: &str) -> String {
    let mut output = String::new();
    for character in os
        .chars()
        .filter(|character| !character.is_control())
        .take(96)
    {
        output.push(character);
    }
    if output.is_empty() {
        "unknown OS".to_string()
    } else {
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn info(platform_id: Option<&str>, os: &str) -> HostInfo {
        HostInfo {
            ip: "127.0.0.1".to_string(),
            host_name: "host".to_string(),
            os: os.to_string(),
            platform_id: platform_id.map(str::to_string),
            whoami: "user".to_string(),
            remark: String::new(),
            system_proxy: None,
            socks5_proxy: None,
            capabilities: None,
        }
    }

    #[test]
    fn platform_id_wins_and_os_aliases_are_case_insensitive() {
        assert_eq!(
            HostPlatform::from_parts(Some("linux"), "Windows 11"),
            HostPlatform::Linux
        );
        for alias in [
            "Linux",
            "Ubuntu 24.04",
            "Debian",
            "Fedora",
            "RHEL 9",
            "Arch Linux",
            "Alpine",
            "SUSE",
            "Gentoo",
            "Linux Mint",
            "Kali",
            "Manjaro",
            "NixOS",
            "Rocky Linux",
            "AlmaLinux",
        ] {
            assert_eq!(
                HostPlatform::from_parts(None, alias),
                HostPlatform::Linux,
                "{alias}"
            );
        }
        assert_eq!(
            HostPlatform::from_parts(None, "Microsoft Windows 11"),
            HostPlatform::Windows
        );
        for alias in [
            "FreeBSD",
            "OpenBSD",
            "NetBSD",
            "DragonFly BSD",
            "HardenedBSD",
            "MidnightBSD",
        ] {
            assert_eq!(
                HostPlatform::from_parts(None, alias),
                HostPlatform::Bsd,
                "{alias}"
            );
        }
        assert_eq!(
            HostPlatform::from_parts(None, "Plan 9"),
            HostPlatform::Unknown
        );
        assert_eq!(
            HostPlatform::from_parts(Some("unknown-id"), "Debian"),
            HostPlatform::Linux
        );
    }

    #[test]
    fn platform_icons_have_distinct_accents_and_labels() {
        let mut colors = HashSet::new();
        let mut labels = HashSet::new();
        for platform in [
            HostPlatform::Linux,
            HostPlatform::Windows,
            HostPlatform::Bsd,
            HostPlatform::Unknown,
        ] {
            labels.insert(platform.label());
            for dark in [false, true] {
                colors.insert(platform.accent(dark).to_array());
            }
        }
        assert_eq!(labels.len(), 4);
        assert_eq!(colors.len(), 8);
    }

    #[test]
    fn table_specs_are_complete_and_use_a_48_point_icon_column() {
        assert_eq!(HOST_COLUMNS.len(), 11);
        assert_eq!(HOST_COLUMNS[0].initial, HOST_ICON_COLUMN_WIDTH);
        assert_eq!(HOST_COLUMNS[0].minimum, HOST_ICON_COLUMN_WIDTH);
        assert_eq!(
            HOST_COLUMNS
                .iter()
                .map(|column| column.title)
                .collect::<Vec<_>>(),
            vec![
                "",
                "Peer IP",
                "Whoami",
                "Host Name",
                "OS",
                "Protocol",
                "Transfer Rate",
                "Last Heartbeat",
                "System Proxy",
                "SOCKS5 Proxy",
                "Remark",
            ]
        );
        assert!(HOST_COLUMNS.iter().all(|column| column.minimum > 0.0));
        assert_eq!(
            host_table_min_width(8.0),
            HOST_COLUMNS
                .iter()
                .map(|column| column.initial)
                .sum::<f32>()
                + 8.0 * 10.0
        );
        assert!(host_table_min_width(12.0) > host_table_min_width(8.0));
    }

    #[test]
    fn tooltip_os_is_control_free_and_bounded() {
        let os = format!("Linux\n\u{0}{}", "x".repeat(120));
        let text = sanitize_tooltip_os(&os);
        assert!(!text.contains('\n'));
        assert!(!text.contains('\u{0}'));
        assert!(text.chars().count() <= 96);
        assert_eq!(sanitize_tooltip_os(""), "unknown OS");
        assert_eq!(
            HostPlatform::from_info(&info(None, "FreeBSD 14")),
            HostPlatform::Bsd
        );
    }

    #[test]
    fn tooltip_and_host_info_do_not_expose_new_credentials() {
        let host = info(Some("linux"), "Linux");
        assert_eq!(HostPlatform::from_info(&host), HostPlatform::Linux);
        assert!(!format!("{:?}", host).contains("password"));
    }
}
