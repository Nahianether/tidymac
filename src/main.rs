mod app;
mod categories;
mod cleaner;
mod utils;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("TidyMac")
            .with_inner_size([720.0, 620.0])
            .with_min_inner_size([500.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "TidyMac",
        options,
        Box::new(|cc| Ok(Box::new(app::TidyMacApp::new(cc)))),
    )
}
