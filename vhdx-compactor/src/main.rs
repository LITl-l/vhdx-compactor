// On Windows release builds, mark the executable as a GUI-subsystem binary so
// no console window pops when the user double-clicks it from Explorer. Debug
// builds keep the console subsystem so log output stays visible.
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod app;
mod pipeline;
mod platform;
mod ui;

#[cfg(not(windows))]
mod stub;
#[cfg(windows)]
mod windows;

use eframe::egui;

fn main() -> eframe::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 520.0])
            .with_min_inner_size([520.0, 400.0])
            .with_title("WSL2 VHDX Compactor"),
        ..Default::default()
    };

    eframe::run_native(
        "WSL2 VHDX Compactor",
        options,
        Box::new(|_cc| Ok(Box::new(app::App::new()))),
    )
}
