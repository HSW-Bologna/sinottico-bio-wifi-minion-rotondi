#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod controller;
mod model;
mod view;

use simplelog::*;
use std::sync::{Arc, Mutex};
use view::app::App;

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        //WriteLogger::new(LevelFilter::Info, Config::default(), File::create("my_rust_binary.log").unwrap()),
    ])
    .unwrap();

    let model = Arc::new(Mutex::new(model::Model::default()));

    let controller_model = Arc::clone(&model);
    let view_model = Arc::clone(&model);

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(480., 320.)),
        min_window_size: Some(egui::vec2(480., 320.)),
        ..eframe::NativeOptions::default()
    };

    eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc| {
            let controller = controller::Controller::new(controller_model, cc.egui_ctx.clone());
            let tx = controller.get_command_channel();
            controller.start();
            Box::new(App::new(view_model, tx))
        }),
    );
}
