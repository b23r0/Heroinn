#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{
    collections::HashMap,
    sync::{
        mpsc::{channel, Sender},
        Arc, RwLock,
    },
};

use eframe::{egui, App};
use egui_extras::{Column, Size, StripBuilder};
use heroinn_util::{
    ftp::{
        method::{join_path, transfer_size, transfer_speed},
        FTPId, FTPPacket, FileInfo,
    },
    protocol::tcp::TcpConnection,
    rpc::{RpcClient, RpcMessage},
};
use std::sync::LazyLock;

mod controller;
mod msgbox;
use controller::*;

static G_RPCCLIENT: LazyLock<Arc<RpcClient>> = LazyLock::new(|| Arc::new(RpcClient::new()));
static G_TRANSFER: LazyLock<Arc<RwLock<HashMap<String, TransferInfo>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

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

/// Decode an embedded `.ico` resource into a GPU-resident egui texture.
fn load_icon(ctx: &egui::Context, name: &str, bytes: &[u8]) -> egui::TextureHandle {
    let img = image::load_from_memory(bytes)
        .expect("failed to decode embedded icon")
        .to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    ctx.load_texture(name, color, egui::TextureOptions::LINEAR)
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
    drive_image: Option<egui::TextureHandle>,
    folder_image: Option<egui::TextureHandle>,
    file_image: Option<egui::TextureHandle>,
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
            drive_image: None,
            folder_image: None,
            file_image: None,
            title: String::from("Heroinn FTP"),
            local_folder_strace: vec![],
            remote_folder_strace: vec![],
        }
    }
}

impl App for FtpApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if !self.initilized {
            ctx.set_theme(egui::Theme::Dark);
            self.initilized = true;
        }

        if self.drive_image.is_none() {
            self.drive_image = Some(load_icon(&ctx, "drive.ico", include_bytes!("res/drive.ico")));
        }
        if self.folder_image.is_none() {
            self.folder_image = Some(load_icon(
                &ctx,
                "folder.ico",
                include_bytes!("res/folder.ico"),
            ));
        }
        if self.file_image.is_none() {
            self.file_image = Some(load_icon(&ctx, "file.ico", include_bytes!("res/file.ico")));
        }

        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.switch, SwitchDock::List, "List");
                ui.selectable_value(&mut self.switch, SwitchDock::Transfer, "Transfer");

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
                SwitchDock::List => {
                    StripBuilder::new(ui)
                        .size(Size::remainder())
                        .vertical(|mut strip| {
                            strip.strip(|builder| {
                                builder.sizes(Size::remainder(), 2).horizontal(|mut strip| {
                                    strip.cell(|ui| {
                                        self.render_file_table("1", ui, FSType::Local);
                                    });
                                    strip.cell(|ui| {
                                        self.render_file_table("2", ui, FSType::Remote);
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
                                self.transfer_table("3", ui);
                            });
                        });
                }
            }
        });

        ctx.request_repaint();
    }
}

impl FtpApp {
    fn render_file_table(&mut self, id: &str, ui: &mut egui::Ui, typ: FSType) {
        egui::CentralPanel::default()
            .show(ui, |ui| {

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
                                                    msgbox::error(&"heroinn FTP".to_string(), &format!("get folder info failed : {}" ,e));
                                                    ui.close();
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
                                            
                                            let fullpath = match get_remote_join_path(&self.sender ,&parent_path, "."){
                                                Ok(p) => p,
                                                Err(e) => {
                                                    msgbox::error(&self.title.to_string(), &format!("join remote path failed : {}" ,e));
                                                    ui.close();
                                                    return;
                                                },
                                            };
                                            log::debug!("remote full path : {}" , fullpath);
        
                                            self.remote_disk_info = match get_remote_folder_info(&self.sender , &fullpath){
                                                Ok(p) => p,
                                                Err(e) => {
                                                    msgbox::error(&self.title.to_string(), &format!("get remote folder info failed : {}" ,e));
                                                    ui.close();
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
                            self.file_table(id, ui, typ);
                        });
                    });
                });
            });
        });
    }

    fn file_table(&mut self, id: &str, ui: &mut egui::Ui, typ: FSType) {
        ui.push_id(id, |ui| {
            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::initial(50.0).at_least(50.0))
                .column(Column::initial(110.0).at_least(50.0))
                .column(Column::initial(50.0).at_least(50.0))
                .column(Column::initial(90.0).at_least(50.0))
                .column(Column::initial(165.0).at_least(50.0))
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
                                                    &format!("get folder info failed : {}", e),
                                                );
                                                ui.close();
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
                                                &format!("join remote path failed : {}", e),
                                            );
                                            ui.close();
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
                                                        "get remote folder info failed : {}",
                                                        e
                                                    ),
                                                );
                                                ui.close();
                                                return;
                                            }
                                        };

                                    self.remote_folder_strace.push(self.remote_path.clone());
                                    self.remote_path = fullpath;
                                }
                                ui.close();
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
                                                        &format!("join remote path failed : {}", e),
                                                    );
                                                    ui.close();
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
                                                            "download remote file failed : {}",
                                                            e
                                                        ),
                                                    );
                                                    ui.close();
                                                    return;
                                                }
                                            };
                                        } else {
                                            msgbox::error(
                                                &self.title,
                                                &"not allow download to root path".to_string(),
                                            );
                                        }

                                        ui.close();
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
                                                    &format!("join remote path failed : {}", e),
                                                );
                                                ui.close();
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
                                                        "download remote file failed : {}",
                                                        e
                                                    ),
                                                );
                                                ui.close();
                                                return;
                                            }
                                        };
                                    } else {
                                        msgbox::error(
                                            &self.title,
                                            &"not allow download to root path".to_string(),
                                        );
                                    }

                                    ui.close();
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
                                                    &format!("join remote path failed : {}", e),
                                                );
                                                ui.close();
                                                return;
                                            }
                                        };

                                        match delete_remote_file(&self.sender, &fullpath) {
                                            Ok(p) => p,
                                            Err(e) => {
                                                msgbox::error(
                                                    &self.title.to_string(),
                                                    &format!(
                                                        "delete remote file info failed : {}",
                                                        e
                                                    ),
                                                );
                                                ui.close();
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
                                                    &format!("delete local file failed : {}", e),
                                                );
                                                ui.close();
                                                return;
                                            }
                                        };

                                        self.local_disk_info =
                                            FtpApp::refresh_local_path(&self.local_path);
                                    }

                                    ui.close();
                                }
                            }
                        };

                        let row_height = 20.0;
                        body.row(row_height, |mut row| {
                            row.col(|ui| {
                                let icon = if i.typ == "FOLDER" {
                                    &self.folder_image
                                } else if i.typ == "SSD"
                                    || i.typ == "HDD"
                                    || i.typ == "Unknown Drive"
                                {
                                    &self.drive_image
                                } else {
                                    &self.file_image
                                };
                                if let Some(tex) = icon {
                                    ui.add(egui::Image::from_texture(egui::load::SizedTexture::new(
                                        tex.id(),
                                        egui::Vec2::new(20.0, 20.0),
                                    )));
                                }
                            })
                            .1.context_menu(|ui| {
                                menu(ui);
                            });

                            row.col(|ui| {
                                ui.label(i.name.clone());
                            })
                            .1.context_menu(|ui| {
                                menu(ui);
                            });

                            row.col(|ui| {
                                ui.label(i.typ.clone());
                            })
                            .1.context_menu(|ui| {
                                menu(ui);
                            });

                            row.col(|ui| {
                                ui.label(transfer_size(i.size as f64));
                            })
                            .1.context_menu(|ui| {
                                menu(ui);
                            });

                            row.col(|ui| {
                                ui.label(i.last_modified.clone());
                            })
                            .1.context_menu(|ui| {
                                menu(ui);
                            });
                        });
                    }
                });
        });
    }

    fn transfer_table(&mut self, id: &str, ui: &mut egui::Ui) {
        ui.push_id(id, |ui| {
            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::initial(50.0).at_least(50.0))
                .column(Column::initial(220.0).at_least(50.0))
                .column(Column::initial(220.0).at_least(50.0))
                .column(Column::initial(80.0).at_least(50.0))
                .column(Column::initial(100.0).at_least(50.0))
                .column(Column::initial(100.0).at_least(50.0))
                .column(Column::initial(150.0).at_least(50.0))
                .column(Column::initial(55.0).at_least(50.0))
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
                                if let Some(tex) = &self.file_image {
                                    ui.add(egui::Image::from_texture(egui::load::SizedTexture::new(
                                        tex.id(),
                                        egui::Vec2::new(20.0, 20.0),
                                    )));
                                }
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
                    &format!("get folder info failed : {}", e),
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
                    &format!("get remote folder info failed : {}", e),
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
                Ok(FTPId::RPC) => {
                    let msg = RpcMessage::parse(&packet.data).unwrap();
                    log::debug!("ftp recv msg from core : {}", msg.id);
                    G_RPCCLIENT.write(&msg);
                }
                Ok(FTPId::Close) => {
                    std::process::exit(0);
                }
                Ok(FTPId::Get | FTPId::Put) => {}
                Err(e) => log::warn!("unknown ftp packet id : {}", e),
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
            s2.send(&msg.serialize().unwrap()).unwrap();
        })
        .unwrap();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1070.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        &format!("Heroinn FTP - {}", title),
        options,
        Box::new(|_cc| Ok(Box::new(FtpApp::new(sender)))),
    )
    .unwrap();
}
