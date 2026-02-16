mod analyzer;
mod app;
mod categories;
mod cleaner;
mod disk_info;
mod monitor;
mod shredder;
mod utils;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("TidyMac")
            .with_inner_size([600.0, 800.0])
            .with_min_inner_size([600.0, 800.0])
            .with_max_inner_size([600.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "TidyMac",
        options,
        Box::new(|cc| Ok(Box::new(app::TidyMacApp::new(cc)))),
    )
}
