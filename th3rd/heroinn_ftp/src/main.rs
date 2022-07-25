use eframe::{egui, App};
use egui_extras::{Size, StripBuilder};
use log::LevelFilter;
use simple_logger::SimpleLogger;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
enum SwitchDock {
    List,
    Transfer
}

struct FtpApp{
    switch : SwitchDock,
    cur_path : String,
    remote_path : String,
}

impl Default for FtpApp{
    fn default() -> Self {
        Self { 
            switch : SwitchDock::List,
            cur_path : String::from("C:\\"),
            remote_path : String::from("C:\\")
        }
    }
}

impl App for FtpApp{
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        egui::CentralPanel::default().show(ctx, |ui| {

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
                                    self.render_table("1", ctx, ui , "Local FS" , vec![1,2,3]);
                                });
                                strip.cell(|ui| {
                                    self.render_table("2", ctx, ui , "Remote FS" , vec![1,2,3]);
                                });
                            });
                        });
                    });
                },
                SwitchDock::Transfer => {

                },
            }
        });


    }
}

impl FtpApp{

    fn render_table(&mut self , id : &str , ctx :&egui::Context , ui : &mut egui::Ui , title : &str , files : Vec<u32>){
            egui::CentralPanel::default()
            .show_inside(ui, |ui| {

            ui.vertical_centered(|ui| {
                ui.heading(title);
            });
            ui.separator();
                
            StripBuilder::new(ui)
            .size(Size::exact(18.0))
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
                            if title == "Local FS"{
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
            .column(Size::initial(50.0).at_least(50.0))
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

                for _ in files {
                    let row_height = 20.0;
                    body.row(row_height, |mut row| {
                        
                        row.col(|ui| {
                            //ui.add(
                            //    egui::Image::new(self.listener_image.texture_id(ctx), egui::Vec2::new(30.0, 30.0))
                            //);
                        });

                        row.col(|ui| {
                            ui.label("test.txt");
                        });
                        row.col(|ui| {
                            ui.label("File");
                        });

                        row.col(|ui| {
                            ui.label("10 kb");
                        });

                        row.col(|ui| {
                            ui.label("2022-02-02 10:32:00");
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
}

fn main() {
    SimpleLogger::new().with_utc_timestamps().with_utc_timestamps().with_colors(true).init().unwrap();
	::log::set_max_level(LevelFilter::Info);
    
    let mut options = eframe::NativeOptions::default();
    options.initial_window_size = Some(egui::Vec2::new(1060.0,500.0));
    eframe::run_native(
        "Heroinn",
        options,
        Box::new(|_cc| Box::new(FtpApp::default())),
    );
}
