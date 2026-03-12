mod app;
mod files;
mod fonts;
mod history;
mod keymap;
mod ui;

use std::path::PathBuf;

fn main() {
    let folder = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("cannot read cwd"));

    if !folder.is_dir() {
        eprintln!("Error: '{}' is not a directory", folder.display());
        std::process::exit(1);
    }

    let app = match app::App::new(folder) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Categorizer")
            .with_inner_size([900.0, 600.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Categorizer",
        options,
        Box::new(|cc| {
            fonts::configure_fonts(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
    .expect("failed to run eframe");
}
