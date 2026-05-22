#![cfg_attr(not(windows), allow(dead_code, unused_imports))]

#[cfg(not(windows))]
fn main() {
    eprintln!("sd_tester is Windows-only. Build and run on Windows 10/11.");
}

#[cfg(windows)]
mod app;
#[cfg(windows)]
mod capacity;
#[cfg(windows)]
mod engine;
#[cfg(windows)]
mod forensics;
#[cfg(windows)]
mod io;
#[cfg(windows)]
mod raw;
#[cfg(windows)]
mod state;
#[cfg(windows)]
mod targets;
#[cfg(windows)]
mod ui;
#[cfg(windows)]
mod win32;
#[cfg(windows)]
mod workloads;

#[cfg(windows)]
fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1250.0, 820.0]),
        ..Default::default()
    };

    eframe::run_native(
        "SD/USB Native Tester",
        native_options,
        Box::new(|_cc| Ok(Box::new(app::SdTesterApp::default()))),
    )
}
