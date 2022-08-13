use controller::*;
//use dlg::file::FileDlg;
use eframe::egui::{self};
use egui_extras::{Size, StripBuilder , TableBuilder};
use heroinn_util::*;
use log::LevelFilter;
use simple_logger::SimpleLogger;

mod controller;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
enum SwitchDock {
    Hosts,
    Listener,
    Generator
}

#[derive(PartialEq)]
enum HeroinnPlatform{
    LinuxX64,
    WindowsX64
}

impl std::fmt::Debug for HeroinnPlatform{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LinuxX64 => write!(f, "Linux_x86_64"),
            Self::WindowsX64 => write!(f, "Windows_x86_64"),
        }
    }
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
    SimpleLogger::new().with_utc_timestamps().with_utc_timestamps().with_colors(true).init().unwrap();
	::log::set_max_level(LevelFilter::Info);
    
    let mut options = eframe::NativeOptions::default();
    options.initial_window_size = Some(egui::Vec2::new(1375.0,610.0));
    eframe::run_native(
        "Heroinn",
        options,
        Box::new(|_cc| Box::new(HeroinnApp::default())),
    );
}
struct HeroinnApp{
    initilized : bool,
    switch : SwitchDock,
    resizable: bool,
    text_listen_port : String,
    combox_listen_protocol : HeroinnProtocol,
    text_generator_port : String,
    text_generator_address : String,
    text_generator_remark : String,
    combox_generator_protocol : HeroinnProtocol,
    combox_generator_platform : HeroinnPlatform,
    host_image : egui_extras::RetainedImage,
    listener_image : egui_extras::RetainedImage,
}

impl Default for HeroinnApp {
    fn default() -> Self { 
        Self { 
            initilized : false , 
            switch : SwitchDock::Hosts, 
            resizable: true,
            text_listen_port : String::new(),
            combox_listen_protocol : HeroinnProtocol::TCP,
            text_generator_port : String::new(),
            text_generator_address : String::new(),
            text_generator_remark : String::new(),
            combox_generator_protocol : HeroinnProtocol::TCP,
            combox_generator_platform : HeroinnPlatform::WindowsX64,
            host_image : egui_extras::RetainedImage::from_image_bytes(
                "host.ico",
                include_bytes!("res/host.ico"),
            ).unwrap(),
            listener_image : egui_extras::RetainedImage::from_image_bytes(
                "host.ico",
                include_bytes!("res/listen.ico"),
            ).unwrap()
        } 
    }
}

impl eframe::App for HeroinnApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {

            if !self.initilized {
                ui.ctx().set_visuals(egui::Visuals::dark());
                self.initilized = true;
            }

            self.ui(ctx , ui);
            ctx.request_repaint();
        });
    }
}
impl HeroinnApp {
    fn ui(&mut self, ctx: &egui::Context , ui: &mut egui::Ui) {

        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.switch, SwitchDock::Hosts, "Hosts");
            ui.selectable_value(&mut self.switch, SwitchDock::Listener, "Listener");
            ui.selectable_value(&mut self.switch,SwitchDock::Generator,"Generator");

            let visuals = ui.ctx().style().visuals.clone();
            match visuals.light_dark_small_toggle_button(ui){
                Some(v) => ui.ctx().set_visuals(v),
                None => {},
            };
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
                                self.host_table(ctx , ui);
                            });
                        });
                        strip.cell(|ui| {
                            ui.vertical_centered(|ui| {
                                ui.hyperlink_to("(github)","https://github.com/b23r0/Heroinn")
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
                            .horizontal(|mut strip|{
                                strip.cell(|ui|{
                                    ui.label("Protocol : ");
                                });
                                strip.cell(|ui|{
                                    egui::ComboBox::from_label("")
                                    .selected_text(format!("{:?}", self.combox_listen_protocol))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.combox_listen_protocol, HeroinnProtocol::TCP, "TCP");
                                    });
                                });
                                strip.cell(|ui|{
                                    ui.label("Port : ");
                                });
                                strip.cell(|ui|{
                                    ui.add(egui::TextEdit::singleline(&mut self.text_listen_port).hint_text("9001"));
                                });
                                strip.cell(|ui|{
                                    if ui.button("Add a Listener").clicked(){
                                        match self.text_listen_port.parse::<u16>(){
                                            Ok(port) => {
                                                match add_listener(&self.combox_listen_protocol, port){
                                                    Ok(_) => {},
                                                    Err(e) => {
                                                        msgbox::error(&"Listener".to_string(), &format!("{}" , e));
                                                    },
                                                };
                                            },
                                            Err(e) => {
                                                msgbox::error(&"Listener".to_string(), &format!("{}" , e));
                                            },
                                        };
                                    };
                                });
                            });

                        });
                        strip.cell(|ui|{
                            ui.separator();
                        });
                        strip.cell(|ui| {
                            self.listen_table(ctx , ui);
                        });
                        strip.cell(|ui| {
                            ui.vertical_centered(|ui| {
                                ui.hyperlink_to("(github)","https://github.com/b23r0/Heroinn")
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
                            egui::Grid::new("my_grid")
                            .num_columns(2)
                            .spacing([300.0, 10.0])
                            .striped(true)
                            .show(ui, |ui| {
                                ui.add(doc_link_label("Address", "label,heading"));
                                ui.add(egui::TextEdit::singleline(&mut self.text_generator_address).hint_text("127.0.0.1"));
                                ui.end_row();
            
                                ui.add(doc_link_label("Port", "label,heading"));
                                ui.add(egui::TextEdit::singleline(&mut self.text_generator_port).hint_text("9001"));
                                ui.end_row();
            
                                ui.add(doc_link_label("Protocol", "label,heading"));
                                egui::ComboBox::from_id_source(1)
                                .selected_text(format!("{:?}", self.combox_generator_protocol))
                                .width(280.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.combox_listen_protocol, HeroinnProtocol::TCP, format!("{:?}" , HeroinnProtocol::TCP));
                                });
                                ui.end_row();

                                ui.add(doc_link_label("Platform", "label,heading"));
                                egui::ComboBox::from_id_source(2)
                                .width(280.0)
                                .selected_text(format!("{:?}" , self.combox_generator_platform))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.combox_generator_platform, HeroinnPlatform::WindowsX64, format!("{:?}" , HeroinnPlatform::WindowsX64));
                                    ui.selectable_value(&mut self.combox_generator_platform, HeroinnPlatform::LinuxX64, format!("{:?}" , HeroinnPlatform::LinuxX64));
                                });
                                ui.end_row();

                                ui.add(doc_link_label("Remark", "label,heading"));
                                ui.add(egui::TextEdit::singleline(&mut self.text_generator_remark).hint_text("default"));
                                ui.end_row();
                            });
                            ui.separator();
                        });

                        strip.cell(|ui| {
                            ui.vertical_centered(|ui| {
                                ui.hyperlink_to("(github)","https://github.com/b23r0/Heroinn")
                            });
                        });
                });
            }
        }
    }

    fn listen_table(&mut self,ctx: &egui::Context , ui: &mut egui::Ui) {
        TableBuilder::new(ui)
            .striped(true)
            .cell_layout(egui::Layout::left_to_right().with_cross_align(egui::Align::Center))
            .column(Size::initial(320.0).at_least(50.0))
            .column(Size::initial(320.0).at_least(50.0))
            .column(Size::initial(100.0).at_least(50.0))
            .column(Size::initial(488.0).at_least(50.0))
            .column(Size::initial(100.0).at_least(50.0))
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
                            ui.add(
                                egui::Image::new(self.listener_image.texture_id(ctx), egui::Vec2::new(30.0, 30.0))
                            );
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
                            if ui.button("Remove").clicked(){
                                match remove_listener(listener.id){
                                    Ok(_) => {},
                                    Err(e) => {
                                        msgbox::error(&"Listener".to_string(), &format!("{}" , e));
                                    },
                                };
                            };
                        });
                    });
                }
            });
    }

    fn host_table(&mut self,ctx: &egui::Context , ui: &mut egui::Ui) {
        TableBuilder::new(ui)
            .striped(true)
            .cell_layout(egui::Layout::left_to_right().with_cross_align(egui::Align::Center))
            .column(Size::initial(50.0).at_least(50.0))
            .column(Size::initial(120.0).at_least(50.0))
            .column(Size::initial(150.0).at_least(50.0))
            .column(Size::initial(150.0).at_least(50.0))
            .column(Size::initial(210.0).at_least(50.0))
            .column(Size::initial(100.0).at_least(50.0))
            .column(Size::initial(100.0).at_least(50.0))
            .column(Size::initial(100.0).at_least(50.0))
            .column(Size::initial(150.0).at_least(50.0))
            .column(Size::initial(150.0).at_least(50.0))
            .resizable(self.resizable)
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.heading("");
                });
                header.col(|ui| {
                    ui.heading("Peer IP");
                });
                header.col(|ui| {
                    ui.heading("Network Card IP");
                });
                header.col(|ui| {
                    ui.heading("Host Name");
                });
                header.col(|ui| {
                    ui.heading("OS");
                });
                header.col(|ui| {
                    ui.heading("Protocol");
                });
                header.col(|ui| {
                    ui.heading("In Rate");
                });
                header.col(|ui| {
                    ui.heading("Out Rate");
                });
                header.col(|ui| {
                    ui.heading("Last Heartbeat");
                });
                header.col(|ui| {
                    ui.heading("Remark");
                });
            })
            .body(|mut body| {

                for info in all_host() {

                    let clientid = info .clientid.clone();

                    let menu = |ui : &mut egui::Ui| {
                        if ui.button("Shell").clicked() {
                            match open_shell(&clientid){
                                Ok(_) => {},
                                Err(e) => {
                                    msgbox::error(&"Shell".to_string(), &format!("{}" , e));
                                },
                            };
                            ui.close_menu();
                        }
                        if ui.button("File").clicked() {
                            match open_ftp(&clientid){
                                Ok(_) => {},
                                Err(e) => {
                                    msgbox::error(&"Shell".to_string(), &format!("{}" , e));
                                },
                            };
                            ui.close_menu();
                        }
                    };

                    let row_height = 30.0;
                    body.row(row_height, |mut row| {

                        row.col(|ui| {
                            ui.add(
                                egui::Image::new(self.host_image.texture_id(ctx), egui::Vec2::new(30.0, 30.0))
                            );
                        }).context_menu(menu);

                        row.col(|ui| {
                            ui.label(format!("{}", info.peer_addr));
                        }).context_menu(menu);

                        row.col(|ui| {
                            ui.label(info.info.ip);
                        }).context_menu(menu);
                        row.col(|ui| {
                            ui.label(info.info.host_name);
                        }).context_menu(menu);
                        row.col(|ui| {
                            ui.label(info.info.os);
                        }).context_menu(menu);
                        row.col(|ui| {
                            ui.label(format!("{:?}", info.proto));
                        }).context_menu(menu);

                        row.col(|ui| {
                            let secs = cur_timestamp_secs() - info.last_heartbeat;
                            let in_rate = info.in_rate / (secs + HEART_BEAT_TIME);
                            ui.label(format!("{} byte/s", in_rate));
                        }).context_menu(menu);

                        row.col(|ui| {
                            let secs = cur_timestamp_secs() - info.last_heartbeat;
                            let out_rate = info.out_rate / (secs + HEART_BEAT_TIME);
                            ui.label(format!("{} byte/s", out_rate));
                        }).context_menu(menu);

                        row.col(|ui| {
                            let secs = cur_timestamp_secs() - info.last_heartbeat;
                            if secs > 30 {
                                remove_host(info.clientid);
                            }
                            ui.label(format!("{} s" , secs));
                        }).context_menu(menu);
                        row.col(|ui| {
                            ui.label(info.info.remark);
                        }).context_menu(menu);
                    });
                }
            });
    }
}