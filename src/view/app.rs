use super::Message;
use crate::model::{Connection, Model};
use egui::Layout;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

const DEFAULT_ADDRESS: &str = "14030100";

pub struct App {
    model: Arc<Mutex<Model>>,
    selected_port: String,
    valid_device_address: String,
    device_address: String,
    controller: mpsc::Sender<Message>,
}

impl App {
    pub fn new(model: Arc<Mutex<Model>>, controller: mpsc::Sender<Message>) -> Self {
        Self {
            model,
            controller,
            selected_port: String::new(),
            valid_device_address: String::from(DEFAULT_ADDRESS),
            device_address: String::from(DEFAULT_ADDRESS),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let model = { self.model.lock().unwrap().clone() };

        egui::TopBottomPanel::top(0).show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 8.;

            ui.with_layout(Layout::top_down(egui::Align::Min), |ui| {
                ui.heading("HSW Collaudo Bio");

                ui.with_layout(Layout::left_to_right(), |ui| {
                    egui::ComboBox::from_id_source(1)
                        .width(128.)
                        .selected_text(self.selected_port.as_str())
                        .show_ui(ui, |ui| {
                            for port in &model.ports {
                                ui.selectable_value(
                                    &mut self.selected_port,
                                    port.clone(),
                                    port.clone(),
                                );
                            }
                        });
                    if ui.button("Connetti").clicked() {
                        self.controller
                            .send(Message::ConnectToPort(self.selected_port.clone()))
                            .ok();
                    }
                    ui.with_layout(Layout::right_to_left(), |ui| {
                        ui.label(match &model.connection {
                            Connection::Connected(port) => format!("Connesso a {}", port),
                            Connection::Disconnected => "Disconnesso".into(),
                        });
                    });
                });
            });
        });

        egui::TopBottomPanel::bottom(1).default_height(128.).show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .stick_to_bottom()
                .max_height(128.)
                .show(ui, |ui| {
                    for m in &model.messages {
                        ui.add_sized([ui.available_width(), 8.], egui::Label::new(m));
                    }
                });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if model.is_connected() {
                if ui.text_edit_singleline(&mut self.device_address).changed() {
                    self.manage_address_input();
                }

                ui.add_enabled_ui(self.is_address_valid(), |ui| {
                    if ui.button("Collauda").clicked() {
                        self.controller
                            .send(Message::Test(
                                u32::from_str_radix(self.device_address.as_str(), 16).unwrap(),
                            ))
                            .ok();
                    }
                });
            }
        });
    }
}

impl App {
    fn is_address_valid(self: &Self) -> bool {
        u32::from_str_radix(self.device_address.as_str(), 16).is_ok()
    }

    fn manage_address_input(self: &mut Self) {
        if self.device_address.len() > 0 {
            match u32::from_str_radix(self.device_address.as_str(), 16) {
                Ok(_) => self.valid_device_address = self.device_address.clone(),
                Err(_) => self.device_address = self.valid_device_address.clone(),
            }
        }
    }
}
