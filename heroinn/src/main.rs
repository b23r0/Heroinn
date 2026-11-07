#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use controller::*;
use eframe::egui::{self};
use egui_extras::{Column, Size, StripBuilder, TableBuilder};
use heroinn_util::{
    ConnectionInfo, Socks5ProxyConfig,
    dna::{find_client_template, replace_connection_info_to_new_file},
    ftp::method::transfer_speed,
    packet::ProxyDisplay,
    *,
};
use std::path::PathBuf;

mod controller;
mod msgbox;

#[derive(Clone, Copy, Debug, PartialEq)]
enum SwitchDock {
    Hosts,
    Listener,
    Generator,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(protocol: &str, address: &str) -> ProxyDisplay {
        ProxyDisplay {
            protocol: protocol.to_string(),
            address: address.to_string(),
            source: Some("system".to_string()),
        }
    }

    #[test]
    fn proxy_columns_are_immediately_before_remark() {
        let remark = HOST_COLUMN_TITLES
            .iter()
            .position(|title| *title == "Remark")
            .unwrap();
        assert_eq!(HOST_COLUMN_TITLES[remark - 2], "System Proxy");
        assert_eq!(HOST_COLUMN_TITLES[remark - 1], "SOCKS5 Proxy");
    }

    #[test]
    fn proxy_cells_are_plain_bounded_text() {
        let unsafe_text = display("HTTP\nInjected", "proxy.example:8080\u{0}hidden");
        let system = system_proxy_cell_text(Some(&unsafe_text));
        assert!(!system.contains('\n'));
        assert!(!system.contains('\u{0}'));
        assert_eq!(system_proxy_cell_text(None), "None");
        assert_eq!(socks5_proxy_cell_text(None), "Disabled");

        let long = "x".repeat(MAX_PROXY_CELL_CHARS * 2);
        assert_eq!(
            sanitize_ui_text(&long).chars().count(),
            MAX_PROXY_CELL_CHARS
        );
    }

    #[test]
    fn all_client_reported_text_is_plain_and_bounded() {
        let unsafe_text = "name\nwith\u{0}controls".repeat(MAX_PROXY_CELL_CHARS);
        let sanitized = sanitize_ui_text(&unsafe_text);
        assert!(!sanitized.contains('\n'));
        assert!(!sanitized.contains('\u{0}'));
        assert_eq!(sanitized.chars().count(), MAX_PROXY_CELL_CHARS);
    }

    #[test]
    fn generator_rejects_udp_and_partial_authentication() {
        assert_eq!(
            socks5_from_generator_fields(
                &HeroinnProtocol::UDP,
                true,
                "proxy.example",
                "1080",
                "user",
                "password",
            )
            .unwrap(),
            None
        );
        assert!(
            socks5_from_generator_fields(
                &HeroinnProtocol::HTTP,
                true,
                "proxy.example",
                "1080",
                "user",
                "",
            )
            .is_err()
        );
        let proxy = socks5_from_generator_fields(
            &HeroinnProtocol::TCP,
            true,
            "[::1]",
            "1080",
            "user",
            "password",
        )
        .unwrap()
        .unwrap();
        assert_eq!(proxy.authority(), "[::1]:1080");
    }
}

struct TransferRateDialog {
    clientid: String,
    value: String,
    unit: TransferRateUnit,
    error: Option<String>,
}

/// Decode an embedded `.ico` resource into GPU-resident egui texture.
fn load_icon(ctx: &egui::Context, name: &str, bytes: &[u8]) -> egui::TextureHandle {
    let img = image::load_from_memory(bytes)
        .expect("failed to decode embedded icon")
        .to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    ctx.load_texture(name, color, egui::TextureOptions::LINEAR)
}

#[derive(PartialEq)]
enum HeroinnPlatform {
    LinuxX64,
    WindowsX64,
    BSDX64,
    DARWINX64,
}

impl std::fmt::Debug for HeroinnPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LinuxX64 => write!(f, "Linux_x86_64"),
            Self::WindowsX64 => write!(f, "Windows_x86_64"),
            Self::BSDX64 => write!(f, "BSD_x86_64"),
            Self::DARWINX64 => write!(f, "OSX_x86_64"),
        }
    }
}

impl HeroinnPlatform {
    fn template_file_names(&self) -> &'static [&'static str] {
        match self {
            Self::LinuxX64 => &["heroinn_client_linux", "heroinn_client"],
            Self::WindowsX64 => &["heroinn_client_windows.exe", "heroinn_client.exe"],
            Self::BSDX64 => &["heroinn_client_bsd", "heroinn_client"],
            Self::DARWINX64 => &["heroinn_client_darwin", "heroinn_client"],
        }
    }

    fn default_output_name(&self) -> &'static str {
        match self {
            Self::LinuxX64 => "heroinn_client_linux",
            Self::WindowsX64 => "heroinn_client.exe",
            Self::BSDX64 => "heroinn_client_bsd",
            Self::DARWINX64 => "heroinn_client_darwin",
        }
    }
}

fn client_template_search_dirs() -> Vec<PathBuf> {
    let mut directories = Vec::new();

    if let Some(directory) = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(PathBuf::from))
    {
        directories.push(directory);
    }

    if let Ok(directory) = std::env::current_dir()
        && !directories.contains(&directory)
    {
        directories.push(directory);
    }

    directories
}

fn doc_link_label<'a>(title: &'a str, search_term: &'a str) -> impl egui::Widget + 'a {
    let label = format!("{}:", title);
    let url = String::new();
    move |ui: &mut egui::Ui| {
        ui.hyperlink_to(label, url).on_hover_ui(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("");
                ui.code(search_term);
            });
        })
    }
}

fn main() {
    #[cfg(debug_assertions)]
    {
        simple_logger::SimpleLogger::new()
            .with_threads(true)
            .with_utc_timestamps()
            .with_colors(true)
            .init()
            .unwrap();
        ::log::set_max_level(log::LevelFilter::Debug);
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1375.0, 610.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Heroinn",
        options,
        Box::new(|_cc| Ok(Box::new(HeroinnApp::default()))),
    )
    .unwrap();
}
struct HeroinnApp {
    initilized: bool,
    switch: SwitchDock,
    resizable: bool,
    text_listen_port: String,
    combox_listen_protocol: HeroinnProtocol,
    text_generator_port: String,
    text_generator_address: String,
    text_generator_remark: String,
    use_socks5_proxy: bool,
    text_socks5_host: String,
    text_socks5_port: String,
    text_socks5_username: String,
    text_socks5_password: String,
    combox_generator_protocol: HeroinnProtocol,
    combox_generator_platform: HeroinnPlatform,
    host_image: Option<egui::TextureHandle>,
    listener_image: Option<egui::TextureHandle>,
    transfer_rate_dialog: Option<TransferRateDialog>,
}

impl Default for HeroinnApp {
    fn default() -> Self {
        Self {
            initilized: false,
            switch: SwitchDock::Hosts,
            resizable: true,
            text_listen_port: String::new(),
            combox_listen_protocol: HeroinnProtocol::TCP,
            text_generator_port: String::new(),
            text_generator_address: String::new(),
            text_generator_remark: String::new(),
            use_socks5_proxy: false,
            text_socks5_host: String::new(),
            text_socks5_port: String::new(),
            text_socks5_username: String::new(),
            text_socks5_password: String::new(),
            combox_generator_protocol: HeroinnProtocol::TCP,
            combox_generator_platform: HeroinnPlatform::WindowsX64,
            host_image: None,
            listener_image: None,
            transfer_rate_dialog: None,
        }
    }
}

const HOST_TABLE_MIN_WIDTH: f32 = 1500.0;
const HOST_COLUMN_TITLES: [&str; 11] = [
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
];
const MAX_PROXY_CELL_CHARS: usize = 96;

fn sanitize_ui_text(input: &str) -> String {
    input
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(MAX_PROXY_CELL_CHARS)
        .collect()
}

fn proxy_cell_text(display: Option<&ProxyDisplay>, absent: &str) -> String {
    let Some(display) = display else {
        return absent.to_string();
    };
    let protocol = sanitize_ui_text(&display.protocol);
    let address = sanitize_ui_text(&display.address);
    if address.is_empty() {
        protocol
    } else {
        format!("{protocol} {address}")
    }
}

fn system_proxy_cell_text(display: Option<&ProxyDisplay>) -> String {
    proxy_cell_text(display, "None")
}

fn socks5_proxy_cell_text(display: Option<&ProxyDisplay>) -> String {
    proxy_cell_text(display, "Disabled")
}

fn socks5_from_generator_fields(
    protocol: &HeroinnProtocol,
    enabled: bool,
    host: &str,
    port: &str,
    username: &str,
    password: &str,
) -> Result<Option<Socks5ProxyConfig>> {
    if !enabled || matches!(protocol, HeroinnProtocol::UDP) {
        return Ok(None);
    }
    let port = port.trim().parse::<u16>().map_err(|_| {
        heroinn_util::Error::Other(
            "SOCKS5 proxy port must be an integer between 1 and 65535".to_string(),
        )
    })?;
    if username.is_empty() && password.is_empty() {
        Socks5ProxyConfig::new(host, port).map(Some)
    } else {
        Socks5ProxyConfig::with_auth(host, port, username, password).map(Some)
    }
}

impl eframe::App for HeroinnApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if !self.initilized {
            ctx.set_theme(egui::Theme::Dark);
            self.initilized = true;
        }

        if self.host_image.is_none() {
            self.host_image = Some(load_icon(&ctx, "host.ico", include_bytes!("res/host.ico")));
        }
        if self.listener_image.is_none() {
            self.listener_image = Some(load_icon(
                &ctx,
                "listen.ico",
                include_bytes!("res/listen.ico"),
            ));
        }

        egui::CentralPanel::default().show(ui, |ui| {
            self.ui(ui);
            ctx.request_repaint();
        });
    }
}
impl HeroinnApp {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.switch, SwitchDock::Hosts, "Hosts");
            ui.selectable_value(&mut self.switch, SwitchDock::Listener, "Listener");
            ui.selectable_value(&mut self.switch, SwitchDock::Generator, "Generator");

            let dark = matches!(ui.ctx().theme(), egui::Theme::Dark);
            if ui
                .button(if dark { "Light" } else { "Dark" })
                .on_hover_text("Toggle light/dark theme")
                .clicked()
            {
                ui.ctx().set_theme(if dark {
                    egui::Theme::Light
                } else {
                    egui::Theme::Dark
                });
            }
        });

        ui.separator();

        match self.switch {
            SwitchDock::Hosts => {
                StripBuilder::new(ui)
                    .size(Size::remainder())
                    .size(Size::exact(15.0))
                    .vertical(|mut strip| {
                        strip.cell(|ui| {
                            ui.vertical_centered(|ui| {
                                self.host_table(ui);
                            });
                        });
                        strip.cell(|ui| {
                            ui.vertical_centered(|ui| {
                                ui.hyperlink_to("(github)", "https://github.com/b23r0/Heroinn")
                            });
                        });
                    });
            }
            SwitchDock::Listener => {
                StripBuilder::new(ui)
                    .size(Size::exact(20.0))
                    .size(Size::exact(5.0))
                    .size(Size::remainder())
                    .size(Size::exact(15.0))
                    .vertical(|mut strip| {
                        strip.strip(|builder| {
                            builder
                                .size(Size::exact(70.0))
                                .size(Size::exact(120.0))
                                .size(Size::exact(50.0))
                                .size(Size::remainder())
                                .size(Size::exact(100.0))
                                .horizontal(|mut strip| {
                                    strip.cell(|ui| {
                                        ui.label("Protocol : ");
                                    });
                                    strip.cell(|ui| {
                                        egui::ComboBox::from_label("")
                                            .selected_text(format!(
                                                "{:?}",
                                                self.combox_listen_protocol
                                            ))
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(
                                                    &mut self.combox_listen_protocol,
                                                    HeroinnProtocol::TCP,
                                                    "TCP",
                                                );
                                                ui.selectable_value(
                                                    &mut self.combox_listen_protocol,
                                                    HeroinnProtocol::HTTP,
                                                    "HTTP",
                                                );
                                                ui.selectable_value(
                                                    &mut self.combox_listen_protocol,
                                                    HeroinnProtocol::UDP,
                                                    "UDP",
                                                );
                                            });
                                    });
                                    strip.cell(|ui| {
                                        ui.label("Port : ");
                                    });
                                    strip.cell(|ui| {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut self.text_listen_port)
                                                .hint_text("9001"),
                                        );
                                    });
                                    strip.cell(|ui| {
                                        if ui.button("Add a Listener").clicked() {
                                            match self.text_listen_port.parse::<u16>() {
                                                Ok(port) => {
                                                    match add_listener(
                                                        &self.combox_listen_protocol,
                                                        port,
                                                    ) {
                                                        Ok(_) => {}
                                                        Err(e) => {
                                                            msgbox::error(
                                                                &"Listener".to_string(),
                                                                &format!("{}", e),
                                                            );
                                                        }
                                                    };
                                                }
                                                Err(e) => {
                                                    msgbox::error(
                                                        &"Listener".to_string(),
                                                        &format!("{}", e),
                                                    );
                                                }
                                            };
                                        };
                                    });
                                });
                        });
                        strip.cell(|ui| {
                            ui.separator();
                        });
                        strip.cell(|ui| {
                            self.listen_table(ui);
                        });
                        strip.cell(|ui| {
                            ui.vertical_centered(|ui| {
                                ui.hyperlink_to("(github)", "https://github.com/b23r0/Heroinn")
                            });
                        });
                    });
            }
            SwitchDock::Generator => {
                StripBuilder::new(ui)
                    .size(Size::remainder())
                    .size(Size::exact(15.0))
                    .vertical(|mut strip| {
                        strip.cell(|ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("generator_fields_scroll")
                                .auto_shrink([false, false])
                                .max_height((ui.available_height() - 70.0).max(100.0))
                                .show(ui, |ui| {
                                    egui::Grid::new("my_grid")
                                        .num_columns(2)
                                        .spacing([300.0, 10.0])
                                        .striped(true)
                                        .show(ui, |ui| {
                                    ui.add(doc_link_label("Address", "label,heading"));
                                    ui.add(
                                        egui::TextEdit::singleline(
                                            &mut self.text_generator_address,
                                        )
                                        .hint_text("127.0.0.1"),
                                    );
                                    ui.end_row();

                                    ui.add(doc_link_label("Port", "label,heading"));
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.text_generator_port)
                                            .hint_text("9001"),
                                    );
                                    ui.end_row();

                                    ui.add(doc_link_label("Protocol", "label,heading"));
                                    egui::ComboBox::from_id_salt(1)
                                        .selected_text(format!(
                                            "{:?}",
                                            self.combox_generator_protocol
                                        ))
                                        .width(280.0)
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(
                                                &mut self.combox_generator_protocol,
                                                HeroinnProtocol::TCP,
                                                format!("{:?}", HeroinnProtocol::TCP),
                                            );
                                            ui.selectable_value(
                                                &mut self.combox_generator_protocol,
                                                HeroinnProtocol::HTTP,
                                                format!("{:?}", HeroinnProtocol::HTTP),
                                            );
                                            ui.selectable_value(
                                                &mut self.combox_generator_protocol,
                                                HeroinnProtocol::UDP,
                                                format!("{:?}", HeroinnProtocol::UDP),
                                            );
                                        });
                                    ui.end_row();

                                    if matches!(
                                        self.combox_generator_protocol,
                                        HeroinnProtocol::UDP
                                    ) {
                                        self.use_socks5_proxy = false;
                                        self.text_socks5_host.clear();
                                        self.text_socks5_port.clear();
                                        self.text_socks5_username.clear();
                                        self.text_socks5_password.clear();
                                    } else {
                                        ui.add(doc_link_label(
                                            "Use SOCKS5 proxy",
                                            "label,heading",
                                        ));
                                        ui.checkbox(&mut self.use_socks5_proxy, "");
                                        ui.end_row();

                                        ui.add(doc_link_label("SOCKS5 Host", "label,heading"));
                                        ui.add_enabled(
                                            self.use_socks5_proxy,
                                            egui::TextEdit::singleline(
                                                &mut self.text_socks5_host,
                                            )
                                            .hint_text("proxy.example or [::1]"),
                                        );
                                        ui.end_row();

                                        ui.add(doc_link_label("SOCKS5 Port", "label,heading"));
                                        ui.add_enabled(
                                            self.use_socks5_proxy,
                                            egui::TextEdit::singleline(
                                                &mut self.text_socks5_port,
                                            )
                                            .hint_text("1080"),
                                        );
                                        ui.end_row();

                                        ui.add(doc_link_label(
                                            "SOCKS5 Username",
                                            "label,heading",
                                        ));
                                        ui.add_enabled(
                                            self.use_socks5_proxy,
                                            egui::TextEdit::singleline(
                                                &mut self.text_socks5_username,
                                            )
                                            .hint_text("optional"),
                                        );
                                        ui.end_row();

                                        ui.add(doc_link_label(
                                            "SOCKS5 Password",
                                            "label,heading",
                                        ));
                                        ui.add_enabled(
                                            self.use_socks5_proxy,
                                            egui::TextEdit::singleline(
                                                &mut self.text_socks5_password,
                                            )
                                            .password(true)
                                            .hint_text("optional"),
                                        );
                                        ui.end_row();

                                        if self.use_socks5_proxy {
                                            ui.label("Credential warning:");
                                            ui.colored_label(
                                                egui::Color32::YELLOW,
                                                "SOCKS5 credentials are embedded in the generated client binary and may be extracted.",
                                            );
                                            ui.end_row();
                                        }
                                    }

                                    ui.add(doc_link_label("Platform", "label,heading"));
                                    egui::ComboBox::from_id_salt(2)
                                        .width(280.0)
                                        .selected_text(format!(
                                            "{:?}",
                                            self.combox_generator_platform
                                        ))
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(
                                                &mut self.combox_generator_platform,
                                                HeroinnPlatform::WindowsX64,
                                                format!("{:?}", HeroinnPlatform::WindowsX64),
                                            );
                                            ui.selectable_value(
                                                &mut self.combox_generator_platform,
                                                HeroinnPlatform::LinuxX64,
                                                format!("{:?}", HeroinnPlatform::LinuxX64),
                                            );
                                            ui.selectable_value(
                                                &mut self.combox_generator_platform,
                                                HeroinnPlatform::BSDX64,
                                                format!("{:?}", HeroinnPlatform::BSDX64),
                                            );
                                            ui.selectable_value(
                                                &mut self.combox_generator_platform,
                                                HeroinnPlatform::DARWINX64,
                                                format!("{:?}", HeroinnPlatform::DARWINX64),
                                            );
                                        });
                                    ui.end_row();

                                    ui.add(doc_link_label("Remark", "label,heading"));
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.text_generator_remark)
                                            .hint_text("default"),
                                    );
                                            ui.end_row();
                                        });
                                });
                            ui.separator();

                            StripBuilder::new(ui)
                                .size(Size::exact(570.0))
                                .size(Size::exact(160.0))
                                .horizontal(|mut strip| {
                                    strip.cell(|_ui| {});
                                    strip.cell(|ui| {
                                        if ui.button("Generate").clicked() {
                                            let socks5_proxy = match socks5_from_generator_fields(
                                                &self.combox_generator_protocol,
                                                self.use_socks5_proxy,
                                                &self.text_socks5_host,
                                                &self.text_socks5_port,
                                                &self.text_socks5_username,
                                                &self.text_socks5_password,
                                            ) {
                                                Ok(proxy) => proxy,
                                                Err(e) => {
                                                    msgbox::error(
                                                        &"Generator".to_string(),
                                                        &format!("{}", e),
                                                    );
                                                    return;
                                                }
                                            };
                                            let connection_info = match ConnectionInfo::new_with_socks5(
                                                self.combox_generator_protocol.clone(),
                                                &self.text_generator_address,
                                                &self.text_generator_port,
                                                &self.text_generator_remark,
                                                socks5_proxy,
                                            ) {
                                                Ok(info) => info,
                                                Err(e) => {
                                                    msgbox::error(
                                                        &"Generator".to_string(),
                                                        &format!("{}", e),
                                                    );
                                                    return;
                                                }
                                            };

                                            let slave_file_path = match find_client_template(
                                                self.combox_generator_platform
                                                    .template_file_names(),
                                                &client_template_search_dirs(),
                                            ) {
                                                Ok(path) => path,
                                                Err(e) => {
                                                    msgbox::error(
                                                        &"Generator".to_string(),
                                                        &format!("{}", e),
                                                    );
                                                    return;
                                                }
                                            };

                                            let dialog = rfd::FileDialog::new()
                                                .set_directory(
                                                    std::env::current_dir()
                                                        .unwrap_or_else(|_| PathBuf::from(".")),
                                                )
                                                .set_file_name(
                                                    self.combox_generator_platform
                                                        .default_output_name(),
                                                );
                                            let new_path = match dialog.save_file() {
                                                Some(path) => path,
                                                None => return,
                                            };

                                            match replace_connection_info_to_new_file(
                                                &slave_file_path,
                                                &new_path,
                                                connection_info,
                                            ) {
                                                Ok(_) => {
                                                    msgbox::info(
                                                        &"Generator".to_string(),
                                                        &"Success!".to_string(),
                                                    );
                                                }
                                                Err(e) => {
                                                    msgbox::error(
                                                        &"Generator".to_string(),
                                                        &format!("{}", e),
                                                    );
                                                }
                                            }
                                        }
                                    });
                                });
                        });

                        strip.cell(|ui| {
                            ui.vertical_centered(|ui| {
                                ui.hyperlink_to("(github)", "https://github.com/b23r0/Heroinn")
                            });
                        });
                    });
            }
        }

        let ctx = ui.ctx().clone();
        self.show_transfer_rate_dialog(&ctx);
    }

    fn listen_table(&mut self, ui: &mut egui::Ui) {
        TableBuilder::new(ui)
            .striped(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::initial(320.0).at_least(50.0))
            .column(Column::initial(320.0).at_least(50.0))
            .column(Column::initial(100.0).at_least(50.0))
            .column(Column::initial(488.0).at_least(50.0))
            .column(Column::initial(100.0).at_least(50.0))
            .resizable(self.resizable)
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.heading("");
                });
                header.col(|ui| {
                    ui.heading("Protocol");
                });
                header.col(|ui| {
                    ui.heading("Port");
                });
                header.col(|ui| {
                    ui.heading("Status");
                });
                header.col(|ui| {
                    ui.heading("");
                });
            })
            .body(|mut body| {
                for listener in all_listener() {
                    let row_height = 30.0;
                    body.row(row_height, |mut row| {
                        row.col(|ui| {
                            if let Some(tex) = &self.listener_image {
                                ui.add(egui::Image::from_texture(egui::load::SizedTexture::new(
                                    tex.id(),
                                    egui::Vec2::new(30.0, 30.0),
                                )));
                            }
                        });

                        row.col(|ui| {
                            ui.label(format!("{:?}", listener.protocol));
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", listener.addr.port()));
                        });

                        row.col(|ui| {
                            ui.label("Running");
                        });

                        row.col(|ui| {
                            if ui.button("Remove").clicked() {
                                match remove_listener(listener.id) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        msgbox::error(&"Listener".to_string(), &format!("{}", e));
                                    }
                                };
                            };
                        });
                    });
                }
            });
    }

    fn host_table(&mut self, ui: &mut egui::Ui) {
        let rate_dialog_request = std::cell::RefCell::new(None::<String>);

        egui::ScrollArea::horizontal()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_min_width(HOST_TABLE_MIN_WIDTH);
                TableBuilder::new(ui)
                    .striped(true)
                    // egui >= 0.20 no longer upgrades a hover-only response to a clickable
                    // one inside `context_menu`, so the cells must sense clicks themselves
                    // for the per-row context menu to open.
                    .sense(egui::Sense::click())
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::initial(48.0).at_least(48.0))
                    .column(Column::initial(145.0).at_least(90.0))
                    .column(Column::initial(100.0).at_least(70.0))
                    .column(Column::initial(135.0).at_least(90.0))
                    .column(Column::initial(180.0).at_least(100.0))
                    .column(Column::initial(85.0).at_least(70.0))
                    .column(Column::initial(105.0).at_least(80.0))
                    .column(Column::initial(115.0).at_least(90.0))
                    .column(Column::initial(170.0).at_least(130.0))
                    .column(Column::initial(170.0).at_least(130.0))
                    .column(Column::initial(180.0).at_least(120.0))
                    .resizable(self.resizable)
                    .header(20.0, |mut header| {
                        for title in HOST_COLUMN_TITLES {
                            header.col(|ui| {
                                ui.heading(title);
                            });
                        }
                    })
                    .body(|mut body| {
                        for info in all_host() {
                            let clientid = info.clientid.clone();

                            let menu = |ui: &mut egui::Ui| {
                                if ui.button("Shell").clicked() {
                                    let clientid = clientid.clone();
                                    std::thread::spawn(move || {
                                        if let Err(e) = open_shell(&clientid) {
                                            msgbox::error(&"Shell".to_string(), &format!("{}", e));
                                        }
                                    });
                                    ui.close();
                                }
                                if ui.button("File").clicked() {
                                    match open_ftp(&clientid) {
                                        Ok(_) => {}
                                        Err(e) => {
                                            msgbox::error(&"Shell".to_string(), &format!("{}", e));
                                        }
                                    };
                                    ui.close();
                                }
                                let rate_requested = std::cell::Cell::new(false);
                                ui.menu_button("Setting", |ui| {
                                    if ui.button("Set Tranfer Rate").clicked() {
                                        *rate_dialog_request.borrow_mut() = Some(clientid.clone());
                                        rate_requested.set(true);
                                    }
                                });
                                if rate_requested.get() {
                                    ui.close();
                                }
                            };

                            let row_height = 30.0;
                            body.row(row_height, |mut row| {
                                row.col(|ui| {
                                    if let Some(tex) = &self.host_image {
                                        ui.add(egui::Image::from_texture(
                                            egui::load::SizedTexture::new(
                                                tex.id(),
                                                egui::Vec2::new(30.0, 30.0),
                                            ),
                                        ));
                                    }
                                })
                                .1
                                .context_menu(menu);

                                row.col(|ui| {
                                    ui.label(sanitize_ui_text(&format!("{}", info.peer_addr)));
                                })
                                .1
                                .context_menu(menu);

                                row.col(|ui| {
                                    ui.label(sanitize_ui_text(&info.info.whoami));
                                })
                                .1
                                .context_menu(menu);
                                row.col(|ui| {
                                    ui.label(sanitize_ui_text(&info.info.host_name));
                                })
                                .1
                                .context_menu(menu);
                                row.col(|ui| {
                                    ui.label(sanitize_ui_text(&info.info.os));
                                })
                                .1
                                .context_menu(menu);
                                row.col(|ui| {
                                    ui.label(format!("{:?}", info.proto));
                                })
                                .1
                                .context_menu(menu);

                                row.col(|ui| {
                                    let secs = cur_timestamp_secs() - info.last_heartbeat;
                                    let window = (secs + HEART_BEAT_TIME) as f64;
                                    let in_rate = info.in_rate as f64 / window;
                                    let out_rate = info.out_rate as f64 / window;
                                    ui.label(transfer_speed(in_rate + out_rate)).on_hover_text(
                                        format!(
                                            "In {} / Out {}",
                                            transfer_speed(in_rate),
                                            transfer_speed(out_rate)
                                        ),
                                    );
                                })
                                .1
                                .context_menu(menu);

                                row.col(|ui| {
                                    let secs = cur_timestamp_secs() - info.last_heartbeat;
                                    if secs > 30 {
                                        remove_host(info.clientid);
                                    }
                                    ui.label(format!("{} s", secs));
                                })
                                .1
                                .context_menu(menu);
                                row.col(|ui| {
                                    ui.label(system_proxy_cell_text(
                                        info.info.system_proxy.as_ref(),
                                    ));
                                })
                                .1
                                .context_menu(menu);
                                row.col(|ui| {
                                    ui.label(socks5_proxy_cell_text(
                                        info.info.socks5_proxy.as_ref(),
                                    ));
                                })
                                .1
                                .context_menu(menu);
                                row.col(|ui| {
                                    ui.label(sanitize_ui_text(&info.info.remark));
                                })
                                .1
                                .context_menu(menu);
                            });
                        }
                    });
            });

        if let Some(clientid) = rate_dialog_request.borrow_mut().take() {
            self.open_transfer_rate_dialog(clientid);
        }
    }

    fn open_transfer_rate_dialog(&mut self, clientid: String) {
        let transfer_rate = get_hostinfo_by_clientid(&clientid)
            .map(|host| host.transfer_rate)
            .unwrap_or(DEFAULT_TRANSFER_RATE);
        let (value, unit) = format_transfer_rate_input(transfer_rate);

        self.transfer_rate_dialog = Some(TransferRateDialog {
            clientid,
            value,
            unit,
            error: None,
        });
    }

    fn show_transfer_rate_dialog(&mut self, ctx: &egui::Context) {
        let mut submit = false;
        let mut cancel = false;

        if let Some(dialog) = self.transfer_rate_dialog.as_mut() {
            egui::Window::new("Set Tranfer Rate")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    let host = get_hostinfo_by_clientid(&dialog.clientid)
                        .map(|host| format!("{} ({})", host.info.host_name, host.peer_addr))
                        .unwrap_or_else(|| dialog.clientid.clone());

                    ui.label(host);
                    ui.label("Applies to all client traffic, including files and shells.");
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("Rate:");
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut dialog.value)
                                .desired_width(120.0)
                                .hint_text("1"),
                        );
                        if response.changed() {
                            dialog.error = None;
                        }

                        let selected_unit = dialog.unit.label();
                        egui::ComboBox::from_id_salt("transfer_rate_unit")
                            .selected_text(selected_unit)
                            .width(70.0)
                            .show_ui(ui, |ui| {
                                for unit in TransferRateUnit::ALL {
                                    if ui
                                        .selectable_value(&mut dialog.unit, unit, unit.label())
                                        .clicked()
                                    {
                                        dialog.error = None;
                                    }
                                }
                            });
                    });

                    if let Some(error) = &dialog.error {
                        let color = ui.visuals().error_fg_color;
                        ui.colored_label(color, error);
                    }

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            submit = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
        }

        if submit {
            let outcome = match self.transfer_rate_dialog.as_ref() {
                Some(dialog) => parse_transfer_rate(&dialog.value, dialog.unit).and_then(|bytes| {
                    set_transfer_rate(&dialog.clientid, bytes).map_err(|e| format!("{}", e))
                }),
                None => Ok(()),
            };

            match outcome {
                Ok(()) => self.transfer_rate_dialog = None,
                Err(error) => {
                    if let Some(dialog) = self.transfer_rate_dialog.as_mut() {
                        dialog.error = Some(error);
                    }
                }
            }
        } else if cancel {
            self.transfer_rate_dialog = None;
        }
    }
}
