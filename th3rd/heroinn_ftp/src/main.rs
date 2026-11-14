#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{
    collections::HashMap,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
        mpsc::{Sender, TryRecvError, channel},
    },
};

use eframe::{App, egui};
use egui_extras::{Column, Size, StripBuilder};
use heroinn_util::{
    ftp::{
        FTPPacket, FileInfo,
        method::{join_path, transfer_size, transfer_speed},
    },
    protocol::tcp::TcpConnection,
    rpc::RpcClient,
};
use std::sync::LazyLock;

mod connection;
mod controller;
mod msgbox;
use connection::FtpWorkers;
use controller::*;

static G_RPCCLIENT: LazyLock<Arc<RpcClient>> = LazyLock::new(|| Arc::new(RpcClient::new()));
static G_TRANSFER: LazyLock<Arc<RwLock<HashMap<String, TransferInfo>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

#[derive(Clone, Copy, Debug, PartialEq)]
enum SwitchDock {
    List,
    Transfer,
}

#[derive(Clone, Copy, PartialEq)]
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

fn can_enter(info: &FileInfo) -> bool {
    info.typ == "FOLDER" || info.typ == "SSD" || info.typ == "HDD" || info.typ.ends_with(" Drive")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RemoteRefreshKind {
    Disk,
    Folder,
}

fn remote_refresh_kind(path: &str) -> RemoteRefreshKind {
    if path == FtpApp::ROOT_FLAG {
        RemoteRefreshKind::Disk
    } else {
        RemoteRefreshKind::Folder
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

fn file_row_icon(ui: &mut egui::Ui, texture: &egui::TextureHandle) -> egui::Response {
    ui.add(
        egui::Image::from_texture(egui::load::SizedTexture::new(
            texture.id(),
            egui::Vec2::new(20.0, 20.0),
        ))
        .sense(egui::Sense::click()),
    )
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
    stopped: Arc<AtomicBool>,
    initilized: bool,
    title: String,
    switch: SwitchDock,
    local_path: String,
    remote_path: String,
    local_disk_info: Vec<FileInfo>,
    remote_disk_info: Vec<FileInfo>,
    remote_loading: bool,
    remote_error: Option<String>,
    remote_load: Option<std::sync::mpsc::Receiver<Result<Vec<FileInfo>, String>>>,
    local_error: Option<String>,
    sender: Sender<FTPPacket>,
    drive_image: Option<egui::TextureHandle>,
    folder_image: Option<egui::TextureHandle>,
    file_image: Option<egui::TextureHandle>,
    local_folder_strace: Vec<String>,
    remote_folder_strace: Vec<String>,
}

impl FtpApp {
    const ROOT_FLAG: &'static str = "[DISK]";

    pub fn new(sender: Sender<FTPPacket>, stopped: Arc<AtomicBool>) -> Self {
        let (local_disk_info, local_error) = match get_local_disk_info() {
            Ok(entries) => (entries, None),
            Err(error) => (
                Vec::new(),
                Some(format!("Local disk discovery failed: {error}")),
            ),
        };

        let (load_sender, load_receiver) = channel();
        let remote_sender = sender.clone();
        let (remote_loading, remote_error, remote_load) = match std::thread::Builder::new()
            .name("ftp initial remote discovery".to_string())
            .spawn(move || {
                let result =
                    get_remote_disk_info(&remote_sender).map_err(|error| error.to_string());
                let _ = load_sender.send(result);
            }) {
            Ok(_) => (true, None, Some(load_receiver)),
            Err(error) => (
                false,
                Some(format!("Remote discovery failed to start: {error}")),
                None,
            ),
        };

        Self {
            stopped,
            initilized: false,
            switch: SwitchDock::List,
            local_path: String::from(FtpApp::ROOT_FLAG),
            remote_path: String::from(FtpApp::ROOT_FLAG),
            remote_disk_info: Vec::new(),
            remote_loading,
            remote_error,
            remote_load,
            local_error,
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

    fn poll_remote_load(&mut self) {
        let result = self
            .remote_load
            .as_ref()
            .map(|receiver| receiver.try_recv());
        match result {
            Some(Ok(result)) => {
                self.remote_load = None;
                self.remote_loading = false;
                Self::apply_remote_load_result(
                    &mut self.remote_disk_info,
                    &mut self.remote_error,
                    result,
                );
            }
            Some(Err(TryRecvError::Disconnected)) => {
                self.remote_load = None;
                self.remote_loading = false;
                self.remote_error =
                    Some("Remote discovery worker stopped without a result".to_string());
            }
            Some(Err(TryRecvError::Empty)) | None => {}
        }
    }

    fn apply_remote_load_result(
        entries: &mut Vec<FileInfo>,
        error: &mut Option<String>,
        result: Result<Vec<FileInfo>, String>,
    ) {
        match result {
            Ok(loaded) => {
                *entries = loaded;
                *error = None;
            }
            Err(message) => {
                *entries = Vec::new();
                *error = Some(format!("Remote files unavailable: {message}"));
            }
        }
    }
}

impl App for FtpApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if close_disconnected_viewport(&ctx, &self.stopped) {
            return;
        }
        self.poll_remote_load();

        if !self.initilized {
            ctx.set_theme(egui::Theme::Dark);
            self.initilized = true;
        }

        if self.drive_image.is_none() {
            self.drive_image = Some(load_icon(
                &ctx,
                "drive.ico",
                include_bytes!("res/drive.ico"),
            ));
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
                                builder.sizes(Size::remainder(), 2).clip(true).horizontal(
                                    |mut strip| {
                                        strip.cell(|ui| {
                                            self.render_file_table("1", ui, FSType::Local);
                                        });
                                        strip.cell(|ui| {
                                            self.render_file_table("2", ui, FSType::Remote);
                                        });
                                    },
                                );
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

fn close_disconnected_viewport(ctx: &egui::Context, stopped: &AtomicBool) -> bool {
    if stopped.load(Ordering::Acquire) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        true
    } else {
        false
    }
}

impl FtpApp {
    fn render_file_table(&mut self, id: &str, ui: &mut egui::Ui, typ: FSType) {
        ui.push_id(id, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading(format!("{:?}", typ));
            });
            if typ == FSType::Remote {
                if self.remote_loading {
                    ui.label("Loading remote files...");
                }
                if let Some(error) = &self.remote_error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
            } else if let Some(error) = &self.local_error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
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

    fn enter_entry(&mut self, typ: FSType, info: &FileInfo) -> Result<(), String> {
        if !can_enter(info) {
            return Ok(());
        }

        if typ == FSType::Local {
            let fullpath = join_path(vec![self.local_path.clone(), info.name.clone()])
                .map_err(|e| format!("join local path failed: {}", e))?
                .into_iter()
                .next()
                .ok_or_else(|| "join local path returned no path".to_string())?;
            let entries = get_local_folder_info(&fullpath)
                .map_err(|e| format!("get folder info failed: {}", e))?;

            self.local_folder_strace.push(self.local_path.clone());
            self.local_path = fullpath;
            self.local_disk_info = entries;
        } else {
            let fullpath = get_remote_join_path(&self.sender, &self.remote_path, &info.name)
                .map_err(|e| format!("join remote path failed: {}", e))?;
            let entries = get_remote_folder_info(&self.sender, &fullpath)
                .map_err(|e| format!("get remote folder info failed: {}", e))?;

            self.remote_folder_strace.push(self.remote_path.clone());
            self.remote_path = fullpath;
            self.remote_disk_info = entries;
        }

        Ok(())
    }

    fn file_table(&mut self, id: &str, ui: &mut egui::Ui, typ: FSType) {
        ui.push_id(id, |ui| {
            ui.set_clip_rect(ui.max_rect());
            let folder_image = self.folder_image.clone();
            let drive_image = self.drive_image.clone();
            let file_image = self.file_image.clone();

            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .sense(egui::Sense::click())
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
                            if can_enter(&i) && ui.button("Open").clicked() {
                                if let Err(e) = self.enter_entry(typ, &i) {
                                    msgbox::error(&self.title.to_string(), &e);
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
                                        match upload_file(&self.sender, &local_path, &remote_path) {
                                            Ok(_) => {
                                                self.switch = SwitchDock::Transfer;
                                            }
                                            Err(e) => {
                                                msgbox::error(
                                                    &self.title.to_string(),
                                                    &format!("download remote file failed : {}", e),
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
                        let mut open_requested = false;
                        body.row(row_height, |mut row| {
                            row.col(|ui| {
                                let icon = if i.typ == "FOLDER" {
                                    &folder_image
                                } else if i.typ == "SSD"
                                    || i.typ == "HDD"
                                    || i.typ == "Unknown Drive"
                                {
                                    &drive_image
                                } else {
                                    &file_image
                                };
                                if let Some(tex) = icon {
                                    file_row_icon(ui, tex).context_menu(|ui| {
                                        menu(ui);
                                    });
                                }
                            })
                            .1
                            .context_menu(|ui| {
                                menu(ui);
                            });

                            row.col(|ui| {
                                ui.label(i.name.clone());
                            })
                            .1
                            .context_menu(|ui| {
                                menu(ui);
                            });

                            row.col(|ui| {
                                ui.label(i.typ.clone());
                            })
                            .1
                            .context_menu(|ui| {
                                menu(ui);
                            });

                            row.col(|ui| {
                                ui.label(transfer_size(i.size as f64));
                            })
                            .1
                            .context_menu(|ui| {
                                menu(ui);
                            });

                            row.col(|ui| {
                                ui.label(i.last_modified.clone());
                            })
                            .1
                            .context_menu(|ui| {
                                menu(ui);
                            });

                            if can_enter(&i) && row.response().double_clicked() {
                                open_requested = true;
                            }
                        });

                        if open_requested && let Err(e) = self.enter_entry(typ, &i) {
                            msgbox::error(&self.title.to_string(), &e);
                        }
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
                                    ui.add(egui::Image::from_texture(
                                        egui::load::SizedTexture::new(
                                            tex.id(),
                                            egui::Vec2::new(20.0, 20.0),
                                        ),
                                    ));
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
            return match get_local_disk_info() {
                Ok(entries) => entries,
                Err(error) => {
                    msgbox::error(
                        &"heroinn FTP".to_string(),
                        &format!("get disk info error : {}", error),
                    );
                    vec![]
                }
            };
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
    fn refresh_remote_path(remote_path: &str, sender: &Sender<FTPPacket>) -> Vec<FileInfo> {
        match remote_refresh_kind(remote_path) {
            RemoteRefreshKind::Disk => match get_remote_disk_info(sender) {
                Ok(p) => p,
                Err(e) => {
                    msgbox::error(
                        &"heroinn FTP".to_string(),
                        &format!("get disk info error : {}", e),
                    );
                    vec![]
                }
            },
            RemoteRefreshKind::Folder => match get_remote_folder_info(sender, remote_path) {
                Ok(p) => p,
                Err(e) => {
                    msgbox::error(
                        &"heroinn FTP".to_string(),
                        &format!("get remote folder info failed : {}", e),
                    );
                    vec![]
                }
            },
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

    let connection = TcpConnection::connect_local_plain(&format!("127.0.0.1:{}", args[1])).unwrap();
    let (mut workers, sender) = FtpWorkers::start(connection).unwrap();
    let stopped = workers.stopped();

    let title = args[2].clone();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1070.0, 500.0]),
        ..Default::default()
    };
    let result = eframe::run_native(
        &format!("Heroinn FTP - {}", title),
        options,
        Box::new(|_cc| Ok(Box::new(FtpApp::new(sender, stopped)))),
    );
    workers.shutdown();
    result.unwrap();
}

#[cfg(test)]
mod tests {
    use super::{FtpApp, RemoteRefreshKind, can_enter, file_row_icon, remote_refresh_kind};
    use eframe::egui;
    use heroinn_util::ftp::FileInfo;

    #[test]
    fn disconnected_io_requests_normal_viewport_close_without_destroying_the_context() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let context = egui::Context::default();
        let stopped = AtomicBool::new(false);
        let mut active = context.run_ui(egui::RawInput::default(), |ui| {
            assert!(!super::close_disconnected_viewport(ui.ctx(), &stopped));
        });
        active.textures_delta.clear();
        assert!(active.viewport_output.values().all(|viewport| {
            !viewport
                .commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::Close))
        }));
        stopped.store(true, Ordering::Release);
        let mut closing = context.run_ui(egui::RawInput::default(), |ui| {
            assert!(super::close_disconnected_viewport(ui.ctx(), &stopped));
        });
        closing.textures_delta.clear();
        assert!(closing.viewport_output.values().any(|viewport| {
            viewport
                .commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::Close))
        }));
        let mut subsequent = context.run_ui(egui::RawInput::default(), |_| {});
        subsequent.textures_delta.clear();
        assert!(
            subsequent
                .viewport_output
                .contains_key(&egui::ViewportId::ROOT)
        );
    }

    fn file_info(typ: &str) -> FileInfo {
        FileInfo {
            name: String::from("entry"),
            typ: String::from(typ),
            ..Default::default()
        }
    }

    #[test]
    fn folders_and_drives_can_be_entered() {
        for typ in ["FOLDER", "SSD", "HDD", "Unknown Drive"] {
            assert!(can_enter(&file_info(typ)), "{typ} should be enterable");
        }
    }

    #[test]
    fn regular_files_cannot_be_entered() {
        assert!(!can_enter(&file_info("FILE")));
        assert!(!can_enter(&file_info("SYMLINK")));
    }

    #[test]
    fn root_refresh_is_a_disk_request_and_not_a_folder_fallback() {
        assert_eq!(
            remote_refresh_kind(FtpApp::ROOT_FLAG),
            RemoteRefreshKind::Disk
        );
        assert_eq!(remote_refresh_kind("/home/user"), RemoteRefreshKind::Folder);
    }

    #[test]
    fn remote_load_state_distinguishes_loading_success_and_failure() {
        let mut entries = Vec::new();
        let mut error = None;
        FtpApp::apply_remote_load_result(&mut entries, &mut error, Ok(vec![file_info("FOLDER")]));
        assert_eq!(entries.len(), 1);
        assert_eq!(error, None);

        FtpApp::apply_remote_load_result(
            &mut entries,
            &mut error,
            Err("connection timed out".to_string()),
        );
        assert!(entries.is_empty());
        assert_eq!(
            error.as_deref(),
            Some("Remote files unavailable: connection timed out")
        );
    }

    #[test]
    fn file_icon_opens_the_context_menu_on_secondary_click() {
        let context = egui::Context::default();
        let texture = context.load_texture(
            "file-row-icon",
            egui::ColorImage::filled([20, 20], egui::Color32::WHITE),
            egui::TextureOptions::LINEAR,
        );
        let screen = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::Vec2::splat(100.0));
        let mut icon_center = egui::pos2(10.0, 10.0);
        let mut menu_opened = false;

        let run_frame =
            |events: Vec<egui::Event>, icon_center: &mut egui::Pos2, menu_opened: &mut bool| {
                let input = egui::RawInput {
                    screen_rect: Some(screen),
                    events,
                    ..Default::default()
                };
                let mut output = context.run_ui(input, |ui| {
                    let response = file_row_icon(ui, &texture);
                    *icon_center = response.rect.center();
                    assert!(response.sense.senses_click());
                    response.context_menu(|ui| {
                        *menu_opened = true;
                        ui.label("menu");
                    });
                });
                output.textures_delta.clear();
            };

        run_frame(Vec::new(), &mut icon_center, &mut menu_opened);
        run_frame(
            vec![egui::Event::PointerMoved(icon_center)],
            &mut icon_center,
            &mut menu_opened,
        );
        run_frame(
            vec![egui::Event::PointerButton {
                pos: icon_center,
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            }],
            &mut icon_center,
            &mut menu_opened,
        );
        run_frame(
            vec![egui::Event::PointerButton {
                pos: icon_center,
                button: egui::PointerButton::Secondary,
                pressed: false,
                modifiers: egui::Modifiers::default(),
            }],
            &mut icon_center,
            &mut menu_opened,
        );
        run_frame(Vec::new(), &mut icon_center, &mut menu_opened);

        assert!(
            menu_opened,
            "secondary-clicking the file icon must open the context menu"
        );
    }
}
