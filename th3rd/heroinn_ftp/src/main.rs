#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{
    collections::HashMap,
    sync::{
        mpsc::{channel, Sender},
        Arc, RwLock,
    },
};

use eframe::{egui, App};
use egui_extras::{Size, StripBuilder};
use heroinn_util::{
    ftp::{
        method::{join_path, transfer_size, transfer_speed},
        FTPId, FTPPacket, FileInfo,
    },
    protocol::{tcp::TcpConnection, Client},
    rpc::{RpcClient, RpcMessage},
};
use lazy_static::*;

mod controller;
mod msgbox;
use controller::*;

lazy_static! {
    static ref G_RPCCLIENT: Arc<RpcClient> = Arc::new(RpcClient::new());
    static ref G_TRANSFER: Arc<RwLock<HashMap<String, TransferInfo>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
enum SwitchDock {
    List,
    Transfer,
}

#[derive(PartialEq)]
enum FSType {
    Local,
    Remote,
}

impl std::fmt::Debug for FSType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => write!(f, "Local FS"),
            Self::Remote => write!(f, "Remote FS"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransferInfo {
    pub typ: String,
    pub local_path: String,
    pub remote_path: String,
    pub size: f64,
    pub remaind_size: f64,
    pub speed: f64,
    pub remaind_time: f64,
}

struct FtpApp {
    initilized: bool,
    title: String,
    switch: SwitchDock,
    local_path: String,
    remote_path: String,
    local_disk_info: Vec<FileInfo>,
    remote_disk_info: Vec<FileInfo>,
    sender: Sender<FTPPacket>,
    drive_image: egui_extras::RetainedImage,
    folder_image: egui_extras::RetainedImage,
    file_image: egui_extras::RetainedImage,
    local_folder_strace: Vec<String>,
    remote_folder_strace: Vec<String>,
}

impl FtpApp {
    const ROOT_FLAG: &'static str = "[DISK]";

    pub fn new(sender: Sender<FTPPacket>) -> Self {
        let remote_disk_info = match get_remote_disk_info(&sender) {
            Ok(p) => p,
            Err(e) => {
                msgbox::error(
                    &"heroinn FTP".to_string(),
                    &format!("get disk info error : {}", e),
                );
                std::process::exit(0);
            }
        };

        let local_disk_info = get_local_disk_info().unwrap();

        Self {
            initilized: false,
            switch: SwitchDock::List,
            local_path: String::from(FtpApp::ROOT_FLAG),
            remote_path: String::from(FtpApp::ROOT_FLAG),
            remote_disk_info,
            sender,
            local_disk_info,
            drive_image: egui_extras::RetainedImage::from_image_bytes(
                "drive.ico",
                include_bytes!("res/drive.ico"),
            )
            .unwrap(),
            folder_image: egui_extras::RetainedImage::from_image_bytes(
                "folder.ico",
                include_bytes!("res/folder.ico"),
            )
            .unwrap(),
            file_image: egui_extras::RetainedImage::from_image_bytes(
                "file.ico",
                include_bytes!("res/file.ico"),
            )
            .unwrap(),
            title: String::from("Heroinn FTP"),
            local_folder_strace: vec![],
            remote_folder_strace: vec![],
        }
    }
}

impl App for FtpApp {
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
                match visuals.light_dark_small_toggle_button(ui) {
                    Some(v) => ui.ctx().set_visuals(v),
                    None => {}
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
                                        self.render_file_table("1", ctx, ui, FSType::Local);
                                    });
                                    strip.cell(|ui| {
                                        self.render_file_table("2", ctx, ui, FSType::Remote);
                                    });
                                });
                            });
                        });
                }
                SwitchDock::Transfer => {
                    StripBuilder::new(ui)
                        .size(Size::exact(30.0))
                        .size(Size::exact(10.0))
                        .size(Size::remainder())
                        .vertical(|mut strip| {
                            strip.cell(|ui| {
                                ui.vertical_centered(|ui| {
                                    ui.heading("Transfer List");
                                });
                            });
                            strip.cell(|ui| {
                                ui.separator();
                            });
                            strip.cell(|ui| {
                                self.transfer_table("3", ctx, ui);
                            });
                        });
                }
            }
        });

        ctx.request_repaint();
    }
}

impl FtpApp {
    fn render_file_table(&mut self, id: &str, ctx: &egui::Context, ui: &mut egui::Ui, typ: FSType) {
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
                    .size(Size::exact(120.0))
                    .horizontal(|mut strip|{
                        strip.cell(|ui|{
                            ui.label("Current Path :");
                        });
                        strip.cell(|ui|{
                            if typ == FSType::Local{
                                ui.label(&self.local_path);
                            } else {
                                ui.label(&self.remote_path);
                            }
                            
                        });
                        strip.strip(|builder|{
                            builder
                            .size(Size::exact(60.0))
                            .size(Size::exact(60.0))
                            .horizontal(|mut strip|{
                                strip.cell(|ui|{
                                    if ui.button("Refresh").clicked(){
                                        if typ == FSType::Local{
                                            self.local_disk_info = FtpApp::refresh_local_path(&self.local_path);
                                        } else {
                                            self.remote_disk_info = FtpApp::refresh_remote_path(&self.remote_path , &self.sender);
                                        }
                                    }
                                });

                                strip.cell(|ui|{
                                    if ui.button("Go Back").clicked(){
                                        if typ == FSType::Local{
        
                                            if self.local_folder_strace.is_empty(){
                                                return;
                                            }
        
                                            let parent_path = self.local_folder_strace.pop().unwrap();
        
                                            if parent_path == FtpApp::ROOT_FLAG{
                                                self.local_disk_info = get_local_disk_info().unwrap();
                                                self.local_path = String::from(FtpApp::ROOT_FLAG);
                                                return;
                                            }
        
                                            let fullpath = join_path(vec![parent_path, ".".to_string()]).unwrap()[0].clone();
        
                                            self.local_disk_info = match get_local_folder_info(&fullpath){
                                                Ok(p) => p,
                                                Err(e) => {
                                                    msgbox::error(&"heroinn FTP".to_string(), &format!("get folder info faild : {}" ,e));
                                                    ui.close_menu();
                                                    return;
                                                },
                                            };
        
                                            self.local_path = fullpath;
                                        } else {
        
                                            if self.remote_folder_strace.is_empty(){
                                                return;
                                            }
        
                                            let parent_path = self.remote_folder_strace.pop().unwrap();
        
                                            if parent_path == FtpApp::ROOT_FLAG{
                                                self.remote_disk_info = match get_remote_disk_info(&self.sender){
                                                    Ok(p) => p,
                                                    Err(e) => {
                                                        msgbox::error(&self.title.to_string(),&format!("get disk info error : {}" , e));
                                                        return;
                                                    },
                                                };
        
                                                self.remote_path = String::from(FtpApp::ROOT_FLAG);
                                                return;
                                            }
                                            
                                            let fullpath = match get_remote_join_path(&self.sender ,&parent_path, &".".to_string()){
                                                Ok(p) => p,
                                                Err(e) => {
                                                    msgbox::error(&self.title.to_string(), &format!("join remote path faild : {}" ,e));
                                                    ui.close_menu();
                                                    return;
                                                },
                                            };
                                            log::debug!("remote full path : {}" , fullpath);
        
                                            self.remote_disk_info = match get_remote_folder_info(&self.sender , &fullpath){
                                                Ok(p) => p,
                                                Err(e) => {
                                                    msgbox::error(&self.title.to_string(), &format!("get remote folder info faild : {}" ,e));
                                                    ui.close_menu();
                                                    return;
                                                },
                                            };
            
                                            self.remote_path = fullpath;
                                        }
                                    }
                                });
                            });



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
                            if typ == FSType::Remote{
                                self.file_table(id,ctx, ui, typ);
                            } else {
                                self.file_table(id,ctx, ui , typ);
                            }
                        });
                    });
                });
            });
        });
    }

    fn file_table(&mut self, id: &str, ctx: &egui::Context, ui: &mut egui::Ui, typ: FSType) {
        ui.push_id(id, |ui| {
            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Size::initial(50.0).at_least(50.0))
                .column(Size::initial(110.0).at_least(50.0))
                .column(Size::initial(50.0).at_least(50.0))
                .column(Size::initial(90.0).at_least(50.0))
                .column(Size::initial(165.0).at_least(50.0))
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
                })
                .body(|mut body| {
                    let files = if typ == FSType::Remote {
                        self.remote_disk_info.clone()
                    } else {
                        self.local_disk_info.clone()
                    };

                    for i in files {
                        let filename = i.name.clone();

                        let mut menu = |ui: &mut egui::Ui| {
                            if (i.typ == "FOLDER"
                                || i.typ == "SSD"
                                || i.typ == "HDD" || i.typ == "Unknown Drive") && ui.button("Open").clicked() {
                                if typ == FSType::Local {
                                    let fullpath = join_path(vec![
                                        self.local_path.clone(),
                                        filename.clone(),
                                    ])
                                    .unwrap()[0]
                                        .clone();

                                    self.local_disk_info =
                                        match get_local_folder_info(&fullpath) {
                                            Ok(p) => p,
                                            Err(e) => {
                                                msgbox::error(
                                                    &self.title.to_string(),
                                                    &format!("get folder info faild : {}", e),
                                                );
                                                ui.close_menu();
                                                return;
                                            }
                                        };

                                    self.local_folder_strace.push(self.local_path.clone());
                                    self.local_path = fullpath;
                                } else {
                                    let fullpath = match get_remote_join_path(
                                        &self.sender,
                                        &self.remote_path,
                                        &filename,
                                    ) {
                                        Ok(p) => p,
                                        Err(e) => {
                                            msgbox::error(
                                                &self.title.to_string(),
                                                &format!("join remote path faild : {}", e),
                                            );
                                            ui.close_menu();
                                            return;
                                        }
                                    };

                                    self.remote_disk_info =
                                        match get_remote_folder_info(&self.sender, &fullpath) {
                                            Ok(p) => p,
                                            Err(e) => {
                                                msgbox::error(
                                                    &self.title.to_string(),
                                                    &format!(
                                                        "get remote folder info faild : {}",
                                                        e
                                                    ),
                                                );
                                                ui.close_menu();
                                                return;
                                            }
                                        };

                                    self.remote_folder_strace.push(self.remote_path.clone());
                                    self.remote_path = fullpath;
                                }
                                ui.close_menu();
                            }

                            if i.typ == "FILE" {
                                if typ == FSType::Remote {
                                    if ui.button("Download").clicked() {
                                        if self.local_path != FtpApp::ROOT_FLAG
                                            && self.remote_path != FtpApp::ROOT_FLAG
                                        {
                                            let remote_path = match get_remote_join_path(
                                                &self.sender,
                                                &self.remote_path,
                                                &filename,
                                            ) {
                                                Ok(p) => p,
                                                Err(e) => {
                                                    msgbox::error(
                                                        &self.title.to_string(),
                                                        &format!("join remote path faild : {}", e),
                                                    );
                                                    ui.close_menu();
                                                    return;
                                                }
                                            };

                                            let local_path = join_path(vec![
                                                self.local_path.clone(),
                                                filename.clone(),
                                            ])
                                            .unwrap()[0]
                                                .clone();
                                            match download_file(
                                                &self.sender,
                                                &local_path,
                                                &remote_path,
                                            ) {
                                                Ok(_) => {
                                                    self.switch = SwitchDock::Transfer;
                                                }
                                                Err(e) => {
                                                    msgbox::error(
                                                        &self.title.to_string(),
                                                        &format!(
                                                            "download remote file faild : {}",
                                                            e
                                                        ),
                                                    );
                                                    ui.close_menu();
                                                    return;
                                                }
                                            };
                                        } else {
                                            msgbox::error(
                                                &self.title,
                                                &"not allow download to root path".to_string(),
                                            );
                                        }

                                        ui.close_menu();
                                    }
                                } else if ui.button("Upload").clicked() {
                                    if self.local_path != FtpApp::ROOT_FLAG
                                        && self.remote_path != FtpApp::ROOT_FLAG
                                    {
                                        let remote_path = match get_remote_join_path(
                                            &self.sender,
                                            &self.remote_path,
                                            &filename,
                                        ) {
                                            Ok(p) => p,
                                            Err(e) => {
                                                msgbox::error(
                                                    &self.title.to_string(),
                                                    &format!("join remote path faild : {}", e),
                                                );
                                                ui.close_menu();
                                                return;
                                            }
                                        };

                                        let local_path = join_path(vec![
                                            self.local_path.clone(),
                                            filename.clone(),
                                        ])
                                        .unwrap()[0]
                                            .clone();
                                        match upload_file(
                                            &self.sender,
                                            &local_path,
                                            &remote_path,
                                        ) {
                                            Ok(_) => {
                                                self.switch = SwitchDock::Transfer;
                                            }
                                            Err(e) => {
                                                msgbox::error(
                                                    &self.title.to_string(),
                                                    &format!(
                                                        "download remote file faild : {}",
                                                        e
                                                    ),
                                                );
                                                ui.close_menu();
                                                return;
                                            }
                                        };
                                    } else {
                                        msgbox::error(
                                            &self.title,
                                            &"not allow download to root path".to_string(),
                                        );
                                    }

                                    ui.close_menu();
                                }

                                if ui.button("Delete").clicked() {
                                    if typ == FSType::Remote {
                                        let fullpath = match get_remote_join_path(
                                            &self.sender,
                                            &self.remote_path,
                                            &filename,
                                        ) {
                                            Ok(p) => p,
                                            Err(e) => {
                                                msgbox::error(
                                                    &self.title.to_string(),
                                                    &format!("join remote path faild : {}", e),
                                                );
                                                ui.close_menu();
                                                return;
                                            }
                                        };

                                        match delete_remote_file(&self.sender, &fullpath) {
                                            Ok(p) => p,
                                            Err(e) => {
                                                msgbox::error(
                                                    &self.title.to_string(),
                                                    &format!(
                                                        "delete remote file info faild : {}",
                                                        e
                                                    ),
                                                );
                                                ui.close_menu();
                                                return;
                                            }
                                        };

                                        self.remote_disk_info = FtpApp::refresh_remote_path(
                                            &self.remote_path,
                                            &self.sender,
                                        );
                                    } else {
                                        let fullpath = join_path(vec![
                                            self.local_path.clone(),
                                            filename.clone(),
                                        ])
                                        .unwrap()[0]
                                            .clone();

                                        match delete_local_file(&fullpath) {
                                            Ok(p) => p,
                                            Err(e) => {
                                                msgbox::error(
                                                    &self.title.to_string(),
                                                    &format!("delete local file faild : {}", e),
                                                );
                                                ui.close_menu();
                                                return;
                                            }
                                        };

                                        self.local_disk_info =
                                            FtpApp::refresh_local_path(&self.local_path);
                                    }

                                    ui.close_menu();
                                }
                            }
                        };

                        let row_height = 20.0;
                        body.row(row_height, |mut row| {
                            row.col(|ui| {
                                if i.typ == "FOLDER" {
                                    ui.add(egui::Image::new(
                                        self.folder_image.texture_id(ctx),
                                        egui::Vec2::new(20.0, 20.0),
                                    ));
                                } else if i.typ == "FILE" {
                                    ui.add(egui::Image::new(
                                        self.file_image.texture_id(ctx),
                                        egui::Vec2::new(20.0, 20.0),
                                    ));
                                } else if i.typ == "SSD"
                                    || i.typ == "HDD"
                                    || i.typ == "Unknown Drive"
                                {
                                    ui.add(egui::Image::new(
                                        self.drive_image.texture_id(ctx),
                                        egui::Vec2::new(20.0, 20.0),
                                    ));
                                } else {
                                    ui.add(egui::Image::new(
                                        self.file_image.texture_id(ctx),
                                        egui::Vec2::new(20.0, 20.0),
                                    ));
                                }
                            })
                            .context_menu(|ui| {
                                menu(ui);
                            });

                            row.col(|ui| {
                                ui.label(i.name.clone());
                            })
                            .context_menu(|ui| {
                                menu(ui);
                            });

                            row.col(|ui| {
                                ui.label(i.typ.clone());
                            })
                            .context_menu(|ui| {
                                menu(ui);
                            });

                            row.col(|ui| {
                                ui.label(transfer_size(i.size as f64));
                            })
                            .context_menu(|ui| {
                                menu(ui);
                            });

                            row.col(|ui| {
                                ui.label(i.last_modified.clone());
                            })
                            .context_menu(|ui| {
                                menu(ui);
                            });
                        });
                    }
                });
        });
    }

    fn transfer_table(&mut self, id: &str, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.push_id(id, |ui| {
            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Size::initial(50.0).at_least(50.0))
                .column(Size::initial(220.0).at_least(50.0))
                .column(Size::initial(220.0).at_least(50.0))
                .column(Size::initial(80.0).at_least(50.0))
                .column(Size::initial(100.0).at_least(50.0))
                .column(Size::initial(100.0).at_least(50.0))
                .column(Size::initial(150.0).at_least(50.0))
                .column(Size::initial(55.0).at_least(50.0))
                .resizable(true)
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("");
                    });
                    header.col(|ui| {
                        ui.heading("Remote Path");
                    });
                    header.col(|ui| {
                        ui.heading("Local Path");
                    });
                    header.col(|ui| {
                        ui.heading("Type");
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
                    let transfer_lock = G_TRANSFER.read().unwrap();
                    let transfer = HashMap::clone(&transfer_lock);
                    drop(transfer_lock);

                    for (_, i) in transfer {
                        let row_height = 20.0;
                        body.row(row_height, |mut row| {
                            row.col(|ui| {
                                ui.add(egui::Image::new(
                                    self.file_image.texture_id(ctx),
                                    egui::Vec2::new(20.0, 20.0),
                                ));
                            });

                            row.col(|ui| {
                                ui.label(&i.remote_path);
                            });
                            row.col(|ui| {
                                ui.label(&i.local_path);
                            });
                            row.col(|ui| {
                                ui.label(&i.typ);
                            });
                            row.col(|ui| {
                                ui.label(transfer_size(i.size));
                            });

                            row.col(|ui| {
                                ui.label(transfer_speed(i.speed));
                            });

                            row.col(|ui| {
                                ui.label(format!("{} s", i.remaind_time as i64));
                            });

                            row.col(|ui| {
                                if ui.button("Cancel").clicked() {
                                    let mut transfer = G_TRANSFER.write().unwrap();
                                    if transfer.contains_key(&i.local_path) {
                                        transfer.remove(&i.local_path);
                                    }
                                };
                            });
                        });
                    }
                });
        });
    }
    fn refresh_local_path(local_path: &String) -> Vec<FileInfo> {
        if local_path == FtpApp::ROOT_FLAG {
            return get_local_disk_info().unwrap();
        }

        match get_local_folder_info(local_path) {
            Ok(p) => p,
            Err(e) => {
                msgbox::error(
                    &"heroinn FTP".to_string(),
                    &format!("get folder info faild : {}", e),
                );
                vec![]
            }
        }
    }
    fn refresh_remote_path(remote_path: &String, sender: &Sender<FTPPacket>) -> Vec<FileInfo> {
        if remote_path == FtpApp::ROOT_FLAG {
            match get_remote_disk_info(sender) {
                Ok(p) => p,
                Err(e) => {
                    msgbox::error(
                        &"heroinn FTP".to_string(),
                        &format!("get disk info error : {}", e),
                    );
                    vec![]
                }
            };
        }

        match get_remote_folder_info(sender, remote_path) {
            Ok(p) => p,
            Err(e) => {
                msgbox::error(
                    &"heroinn FTP".to_string(),
                    &format!("get remote folder info faild : {}", e),
                );
                vec![]
            }
        }
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

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        return;
    }

    let mut s = TcpConnection::connect(&format!("127.0.0.1:{}", args[1])).unwrap();
    let mut s2 = s.clone();

    let title = args[2].clone();

    std::thread::Builder::new()
        .name("ftp receiver worker".to_string())
        .spawn(move || loop {
            let data = match s.recv() {
                Ok(p) => p,
                Err(_) => {
                    std::process::exit(0);
                }
            };
            let packet = FTPPacket::parse(&data).unwrap();

            match packet.id() {
                FTPId::RPC => {
                    let msg = RpcMessage::parse(&packet.data).unwrap();
                    log::debug!("ftp recv msg from core : {}", msg.id);
                    G_RPCCLIENT.write(&msg);
                }
                FTPId::Close => {
                    std::process::exit(0);
                }
                FTPId::Get => {}
                FTPId::Put => {}
                FTPId::Unknown => {}
            }
        })
        .unwrap();

    let (sender, receiver) = channel::<FTPPacket>();

    std::thread::Builder::new()
        .name("ftp sender worker".to_string())
        .spawn(move || loop {
            let msg = match receiver.recv() {
                Ok(p) => p,
                Err(_) => {
                    std::process::exit(0);
                }
            };
            log::debug!("ftp send msg to core : {}", msg.id);
            s2.send(&mut msg.serialize().unwrap()).unwrap();
        })
        .unwrap();

    let mut options = eframe::NativeOptions::default();
    options.initial_window_size = Some(egui::Vec2::new(1070.0, 500.0));
    eframe::run_native(
        &format!("Heroinn FTP - {}", title),
        options,
        Box::new(|_cc| Box::new(FtpApp::new(sender))),
    );
}
