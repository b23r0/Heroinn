use std::{sync::{Arc, mpsc::{channel, Sender}, RwLock}};

use eframe::{egui, App};
use egui_extras::{Size, StripBuilder};
use heroinn_util::{protocol::{tcp::{TcpConnection}, Client}, rpc::{RpcClient, RpcMessage}, ftp::DiskInfo};
use log::LevelFilter;
use simple_logger::SimpleLogger;
use lazy_static::*;

lazy_static!{
    static ref G_RPCCLIENT : Arc<RwLock<RpcClient>> = Arc::new(RwLock::new(RpcClient::new()));
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
enum SwitchDock {
    List,
    Transfer
}

#[derive(PartialEq)]
enum FSType{
    Local,
    Remote
}

impl std::fmt::Debug for FSType{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => write!(f, "Local FS"),
            Self::Remote => write!(f, "Remote FS"),
        }
    }
}

struct FtpApp{
    initilized : bool,
    switch : SwitchDock,
    cur_path : String,
    remote_path : String,
    remote_disk_info : Vec<DiskInfo>,
    sender : Sender<RpcMessage>
}

impl FtpApp{
    pub fn new(sender : Sender<RpcMessage>) -> Self{
        let msg = RpcMessage::build_call("get_disk_info" , vec![]);
        sender.send(msg.clone()).unwrap();
        if let Err(e) =  G_RPCCLIENT.read().unwrap().wait_msg(&msg.id, 10){
            std::process::exit(0);
        };

        Self{ 
            initilized : false,
            switch : SwitchDock::List,
            cur_path : String::from("C:\\"),
            remote_path : String::from("C:\\"),
            remote_disk_info : vec![],
            sender
        }
    }
}

impl App for FtpApp{
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        egui::CentralPanel::default().show(ctx, |ui| {

            if !self.initilized {
                ui.ctx().set_visuals(egui::Visuals::dark());
                self.initilized = true;
            }

            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.switch, SwitchDock::List, "List");
                ui.selectable_value(&mut self.switch, SwitchDock::Transfer, "Transfer");
    
                let visuals = ui.ctx().style().visuals.clone();
                match visuals.light_dark_small_toggle_button(ui){
                    Some(v) => ui.ctx().set_visuals(v),
                    None => {},
                };
            });

            ui.separator();

            match self.switch {
                SwitchDock::List => {

                    StripBuilder::new(ui)
                    .size(Size::remainder())
                    .vertical(|mut strip| {
                        strip.strip(|builder| {
                            builder.sizes(Size::remainder(), 2).horizontal(|mut strip| {
                                strip.cell(|ui| {
                                    self.render_file_table("1", ctx, ui , FSType::Local , vec![1,2,3]);
                                });
                                strip.cell(|ui| {
                                    self.render_file_table("2", ctx, ui , FSType::Remote , vec![1,2,3]);
                                });
                            });
                        });
                    });
                },
                SwitchDock::Transfer => {
                    StripBuilder::new(ui)
                    .size(Size::exact(30.0))
                    .size(Size::exact(10.0))
                    .size(Size::remainder())
                    .vertical(|mut strip|{
                        strip.cell(|ui|{
                            ui.vertical_centered(|ui| {
                                ui.heading("Transfer List");
                            });
                        });
                        strip.cell(|ui|{
                            ui.separator();
                        });
                        strip.cell(|ui|{
                            self.transfer_table("3", ctx, ui, vec![1 ,2 ,3]);
                        });
                    });

                },
            }
        });


    }
}

impl FtpApp{

    fn render_file_table(&mut self , id : &str , ctx :&egui::Context , ui : &mut egui::Ui , typ : FSType , files : Vec<u32>){
        egui::CentralPanel::default()
            .show_inside(ui, |ui| {

            ui.vertical_centered(|ui| {
                ui.heading(format!("{:?}" ,typ));
            });
            ui.separator();
                
            StripBuilder::new(ui)
            .size(Size::exact(20.0))
            .size(Size::exact(5.0))
            .size(Size::remainder())
            .vertical(|mut strip| {
                strip.strip(|builder| {
                    builder
                    .size(Size::exact(80.0))
                    .size(Size::remainder())
                    .size(Size::exact(80.0))
                    .horizontal(|mut strip|{
                        strip.cell(|ui|{
                            ui.label("Current Path :");
                        });
                        strip.cell(|ui|{
                            if typ == FSType::Local{
                                ui.label(&self.cur_path);
                            } else {
                                ui.label(&self.remote_path);
                            }
                            
                        });
                        strip.cell(|ui|{
                            if ui.button("Directory up").clicked(){
        
                            }
                        });     
                    });
                });
                strip.cell(|ui|{
                    ui.separator();
                });
                strip.strip(|builder| {
                    builder
                    .size(Size::remainder())
                    .vertical(|mut strip|{
                        strip.cell(|ui|{
                            self.file_table(id,ctx, ui, files);
                        });
                    });
                });
            });
        });
    }

    fn file_table(&mut self,id : &str ,_ : &egui::Context , ui: &mut egui::Ui , files : Vec<u32>) {
        ui.push_id(id, |ui| {
            egui_extras::TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right().with_cross_align(egui::Align::Center))
            .column(Size::initial(50.0).at_least(50.0))
            .column(Size::initial(100.0).at_least(50.0))
            .column(Size::initial(50.0).at_least(50.0))
            .column(Size::initial(50.0).at_least(50.0))
            .column(Size::initial(150.0).at_least(50.0))
            .column(Size::initial(55.0).at_least(50.0))
            .resizable(true)
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.heading("");
                });
                header.col(|ui| {
                    ui.heading("Name");
                });
                header.col(|ui| {
                    ui.heading("Type");
                });
                header.col(|ui| {
                    ui.heading("Size");
                });
                header.col(|ui| {
                    ui.heading("Last Modified");
                });
                header.col(|ui| {
                    ui.heading("");
                });
            })
            .body(|mut body| {

                for i in &self.remote_disk_info {
                    let row_height = 20.0;
                    body.row(row_height, |mut row| {
                        
                        row.col(|_| {
                            //ui.add(
                            //    egui::Image::new(self.listener_image.texture_id(ctx), egui::Vec2::new(30.0, 30.0))
                            //);
                        });

                        row.col(|ui| {
                            ui.label(i.name.clone());
                        });
                        row.col(|ui| {
                            ui.label(i.typ.clone());
                        });

                        row.col(|ui| {
                            ui.label(format!("{}", i.size));
                        });

                        row.col(|ui| {
                            ui.label("");
                        });

                        row.col(|ui| {
                            if ui.button("...").clicked(){
                            };
                        });
                    });
                }
            });
        });
    }

    fn transfer_table(&mut self,id : &str ,_ : &egui::Context , ui: &mut egui::Ui , files : Vec<u32>) {
        ui.push_id(id, |ui| {
            egui_extras::TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right().with_cross_align(egui::Align::Center))
            .column(Size::initial(50.0).at_least(50.0))
            .column(Size::initial(100.0).at_least(50.0))
            .column(Size::initial(100.0).at_least(50.0))
            .column(Size::initial(290.0).at_least(50.0))
            .column(Size::initial(140.0).at_least(50.0))
            .column(Size::initial(100.0).at_least(50.0))
            .column(Size::initial(150.0).at_least(50.0))
            .column(Size::initial(55.0).at_least(50.0))
            .resizable(true)
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.heading("");
                });
                header.col(|ui| {
                    ui.heading("Name");
                });
                header.col(|ui| {
                    ui.heading("Type");
                });
                header.col(|ui| {
                    ui.heading("Local Path");
                });
                header.col(|ui| {
                    ui.heading("Size");
                });
                header.col(|ui| {
                    ui.heading("Speed");
                });
                header.col(|ui| {
                    ui.heading("Remaind Time");
                });
                header.col(|ui| {
                    ui.heading("");
                });
            })
            .body(|mut body| {

                for _ in files {
                    let row_height = 20.0;
                    body.row(row_height, |mut row| {
                        
                        row.col(|_| {
                            //ui.add(
                            //    egui::Image::new(self.listener_image.texture_id(ctx), egui::Vec2::new(30.0, 30.0))
                            //);
                        });

                        row.col(|ui| {
                            ui.label("test.txt");
                        });
                        row.col(|ui| {
                            ui.label("Download");
                        });
                        row.col(|ui| {
                            ui.label("C:\\test.txt");
                        });
                        row.col(|ui| {
                            ui.label("10 kb");
                        });

                        row.col(|ui| {
                            ui.label("1 kb/s");
                        });

                        row.col(|ui| {
                            ui.label("10 s");
                        });

                        row.col(|ui| {
                            if ui.button("Cancel").clicked(){
                            };
                        });
                    });
                }
            });
        });
    }
}

fn main() {
    SimpleLogger::new().with_utc_timestamps().with_utc_timestamps().with_colors(true).init().unwrap();
	::log::set_max_level(LevelFilter::Info);

    let args : Vec<String> = std::env::args().collect();

    if args.is_empty(){
        return;
    }

    let mut s = TcpConnection::connect(&format!("127.0.0.1:{}" , args[0])).unwrap();
    let mut s2 = s.clone();

    std::thread::spawn(move || {
        loop{
            let data = match s.recv(){
                Ok(p) => p,
                Err(_) => {
                    std::process::exit(0);
                },
            };
    
            let msg = RpcMessage::parse(&data).unwrap();
    
            G_RPCCLIENT.write().unwrap().write(&msg);
        }
    });

    let (sender , receiver) = channel::<RpcMessage>();

    std::thread::spawn(move || {
        loop{
            let msg = receiver.recv().unwrap();
            s2.send(&mut msg.serialize().unwrap()).unwrap();
        }
    });

    let mut options = eframe::NativeOptions::default();
    options.initial_window_size = Some(egui::Vec2::new(1060.0,300.0));
    eframe::run_native(
        "Heroinn",
        options,
        Box::new(|_cc| Box::new(FtpApp::new(sender))),
    );
}
