use egui::{Context, RichText, Color32};
use crate::app::App;

pub fn render(app: &mut App, ctx: &Context) {
    handle_keyboard(app, ctx);

    // Toolbar
    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if ui.button("📂 Open folder…").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    app.open_folder(path);
                }
            }
            ui.separator();
            ui.label(format!("{}", app.folder.display()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let total = app.remaining() + app.history.len();
                let done = app.history.len();
                ui.label(format!("file {}/{}", done + 1, total.max(done + 1)));
            });
        });
    });

    // Right sidebar
    egui::SidePanel::right("sidebar")
        .min_width(180.0)
        .max_width(280.0)
        .show(ctx, |ui| {
            render_sidebar(app, ui);
        });

    // Central image/preview panel
    egui::CentralPanel::default().show(ctx, |ui| {
        render_preview(app, ui, ctx);
    });
}

fn render_sidebar(app: &mut App, ui: &mut egui::Ui) {
    // Filename + size
    if let Some(file) = app.current_file() {
        let name = file.file_name().unwrap_or_default().to_string_lossy();
        ui.heading(RichText::new(name.as_ref()).size(13.0));
        if let Ok(meta) = std::fs::metadata(file) {
            ui.label(format_size(meta.len()));
        }
    } else {
        ui.label("No files");
    }

    ui.separator();

    if app.subdirs.is_empty() {
        ui.colored_label(
            Color32::YELLOW,
            "⚠ Create at least one\nsubdirectory to categorize.",
        );
    } else {
        // Category buttons
        egui::ScrollArea::vertical()
            .id_source("cats")
            .max_height(ui.available_height() - 80.0)
            .show(ui, |ui: &mut egui::Ui| {
                let subdirs = app.subdirs.clone();
                for (i, dir) in subdirs.iter().enumerate() {
                    let label = if i < 9 {
                        format!("[{}] 📁 {}", i + 1, dir.file_name().unwrap_or_default().to_string_lossy())
                    } else {
                        format!("    📁 {}", dir.file_name().unwrap_or_default().to_string_lossy())
                    };
                    if ui.button(label).clicked() && app.current_file().is_some() {
                        if let Err(e) = app.move_current(&dir.clone()) {
                            app.status_message = Some(e);
                        }
                    }
                }
            });
    }

    ui.separator();

    // Status message
    if let Some(msg) = &app.status_message.clone() {
        ui.colored_label(Color32::RED, msg);
        ui.separator();
    }

    // Skip + Undo buttons
    ui.horizontal(|ui| {
        if ui.button("[S] ⏭ Skip").clicked() {
            app.skip_current();
        }
        let undo_btn = egui::Button::new("[Ctrl+Z] ↩ Undo");
        if ui
            .add_enabled(!app.history.is_empty(), undo_btn)
            .clicked()
        {
            if let Err(e) = app.undo() {
                app.status_message = Some(e);
            }
        }
    });
}

fn render_preview(app: &mut App, ui: &mut egui::Ui, ctx: &Context) {
    if app.files.is_empty() && app.skipped.is_empty() {
        // Done screen
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.heading("✅ Done!");
                ui.label(format!("{} files categorized", app.history.len()));
                ui.add_space(16.0);
                if ui.button("📂 Open another folder…").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        app.open_folder(path);
                    }
                }
            });
        });
        return;
    }

    let Some(file) = app.current_file().cloned() else {
        ui.centered_and_justified(|ui| {
            ui.label("No files to categorize.");
        });
        return;
    };

    load_file_view(app, &file, ctx);

    match &app.file_view {
        crate::app::FileView::Loading => {
            ui.centered_and_justified(|ui| { ui.spinner(); });
        }
        crate::app::FileView::Image(texture) => {
            let size = texture.size_vec2();
            let available = ui.available_size();
            let scale = (available.x / size.x).min(available.y / size.y).min(1.0);
            let display_size = size * scale;
            ui.centered_and_justified(|ui| {
                ui.image((texture.id(), display_size));
            });
        }
        crate::app::FileView::Text(text) => {
            let text = text.clone();
            egui::ScrollArea::both().show(ui, |ui| {
                ui.add(egui::TextEdit::multiline(&mut text.as_str()).desired_rows(30).code_editor());
            });
        }
        crate::app::FileView::Other { icon, size } => {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(*icon).size(64.0));
                    ui.label(file.file_name().unwrap_or_default().to_string_lossy().as_ref());
                    ui.label(format_size(*size));
                });
            });
        }
    }
}

fn load_file_view(app: &mut App, file: &std::path::Path, ctx: &Context) {
    // Already cached for this file?
    if let Some((cached_path, _)) = &app.texture_cache {
        if cached_path == file {
            if let Some((_, tex)) = &app.texture_cache {
                app.file_view = crate::app::FileView::Image(tex.clone());
            }
            return;
        }
    }

    if crate::files::is_image(file) {
        match load_image_texture(file, ctx) {
            Ok(texture) => {
                app.texture_cache = Some((file.to_path_buf(), texture.clone()));
                app.file_view = crate::app::FileView::Image(texture);
            }
            Err(_) => {
                let size = std::fs::metadata(file).map(|m| m.len()).unwrap_or(0);
                app.file_view = crate::app::FileView::Other { icon: "🖼", size };
            }
        }
    } else if crate::files::is_text(file) {
        match std::fs::read_to_string(file) {
            Ok(text) => app.file_view = crate::app::FileView::Text(text),
            Err(_) => {
                let size = std::fs::metadata(file).map(|m| m.len()).unwrap_or(0);
                app.file_view = crate::app::FileView::Other { icon: "📄", size };
            }
        }
    } else {
        let (icon, size) = file_icon_and_size(file);
        app.file_view = crate::app::FileView::Other { icon, size };
    }
}

fn load_image_texture(path: &std::path::Path, ctx: &Context) -> Result<egui::TextureHandle, image::ImageError> {
    let img = image::open(path)?.to_rgba8();
    let (w, h) = img.dimensions();
    let pixels = img.into_raw();
    let color_image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &pixels);
    Ok(ctx.load_texture(
        path.to_string_lossy().as_ref(),
        color_image,
        egui::TextureOptions::default(),
    ))
}

fn file_icon_and_size(path: &std::path::Path) -> (&'static str, u64) {
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let icon = match path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref() {
        Some("pdf") => "📕",
        Some("zip") | Some("tar") | Some("gz") | Some("7z") | Some("rar") => "🗜",
        Some("mp4") | Some("mkv") | Some("avi") | Some("mov") | Some("webm") => "🎬",
        Some("mp3") | Some("flac") | Some("wav") | Some("ogg") => "🎵",
        Some("doc") | Some("docx") => "📝",
        Some("xls") | Some("xlsx") => "📊",
        Some("ppt") | Some("pptx") => "📋",
        Some("rs") | Some("py") | Some("js") | Some("ts") | Some("go") | Some("c") | Some("cpp") => "💻",
        _ => "📄",
    };
    (icon, size)
}

fn handle_keyboard(app: &mut App, ctx: &Context) {
    ctx.input(|i| {
        // Number keys 1–9: move to Nth category
        for n in 1usize..=9 {
            let key = match n {
                1 => egui::Key::Num1,
                2 => egui::Key::Num2,
                3 => egui::Key::Num3,
                4 => egui::Key::Num4,
                5 => egui::Key::Num5,
                6 => egui::Key::Num6,
                7 => egui::Key::Num7,
                8 => egui::Key::Num8,
                9 => egui::Key::Num9,
                _ => unreachable!(),
            };
            if i.key_pressed(key) {
                if let Some(dir) = app.subdirs.get(n - 1).cloned() {
                    if let Err(e) = app.move_current(&dir) {
                        app.status_message = Some(e);
                    }
                }
            }
        }

        // S: skip
        if i.key_pressed(egui::Key::S) {
            app.skip_current();
        }

        // Ctrl+Z: undo
        if i.key_pressed(egui::Key::Z) && i.modifiers.ctrl {
            if let Err(e) = app.undo() {
                app.status_message = Some(e);
            }
        }
    });
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
