use egui::{Context, RichText, Color32};
use crate::app::App;
use crate::keymap::{self, BindTarget};

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
            if ui.button("⚙ Keybindings").clicked() {
                app.show_keymap_editor = !app.show_keymap_editor;
                app.listening_bind = None;
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

    // Keymap editor overlay
    if app.show_keymap_editor {
        render_keymap_editor(app, ctx);
    }

    // New folder popup
    if app.show_new_folder_popup {
        render_new_folder_popup(app, ctx);
    }

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
                    let dir_name = dir.file_name().unwrap_or_default().to_string_lossy();
                    let label = if let Some(bind) = app.keymap.category_keys.get(i) {
                        format!("[{}] 📁 {}", bind.label, dir_name)
                    } else {
                        format!("    📁 {}", dir_name)
                    };
                    if ui.button(label).clicked() && app.current_file().is_some() {
                        if let Err(e) = app.move_current(&dir.clone()) {
                            app.status_message = Some(e);
                        }
                    }
                }
            });
    }

    // New folder button
    let nf_label = format!("[{}] 📁+ New folder", app.keymap.new_folder.label);
    if ui.button(nf_label).clicked() {
        app.show_new_folder_popup = true;
        app.new_folder_name.clear();
        app.new_folder_error = None;
    }

    ui.separator();

    // Status message
    if let Some(msg) = &app.status_message.clone() {
        ui.colored_label(Color32::RED, msg);
        ui.separator();
    }

    // Skip + Undo buttons
    ui.horizontal(|ui| {
        let skip_label = format!("[{}] ⏭ Skip", app.keymap.skip.label);
        if ui.button(skip_label).clicked() {
            app.skip_current();
        }
        let undo_label = format!("[{}] ↩ Undo", app.keymap.undo.label);
        let undo_btn = egui::Button::new(undo_label);
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
    // While new folder popup is open, don't process normal shortcuts
    if app.show_new_folder_popup {
        return;
    }

    // In listening mode, consume keypresses for rebinding
    if app.listening_bind.is_some() {
        let mut captured = None;
        ctx.input(|i| {
            // Escape cancels listening
            if i.key_pressed(egui::Key::Escape) {
                captured = Some(None); // Signal cancel
                return;
            }
            for event in &i.events {
                if let egui::Event::Key {
                    key, pressed: true, modifiers, ..
                } = event
                {
                    captured = Some(Some((*key, *modifiers)));
                    break;
                }
            }
        });
        if let Some(result) = captured {
            match result {
                None => {
                    // Escape pressed — cancel
                    app.listening_bind = None;
                }
                Some((key, modifiers)) => {
                    let mods = egui::Modifiers {
                        alt: modifiers.alt,
                        ctrl: modifiers.ctrl,
                        shift: modifiers.shift,
                        mac_cmd: false,
                        command: false,
                    };
                    let target = app.listening_bind.clone().unwrap();
                    if let Some(conflict) = app.keymap.has_conflict(key, mods, Some(&target), app.subdirs.len()) {
                        app.status_message =
                            Some(format!("Key already bound to {conflict}"));
                    } else {
                        let label = keymap::format_key_label(key, mods);
                        let bind = keymap::KeyBind {
                            key,
                            modifiers: mods,
                            label,
                        };
                        match &target {
                            BindTarget::Category(idx) => {
                                if let Some(slot) = app.keymap.category_keys.get_mut(*idx) {
                                    *slot = bind;
                                }
                            }
                            BindTarget::Skip => app.keymap.skip = bind,
                            BindTarget::Undo => app.keymap.undo = bind,
                            BindTarget::NewFolder => app.keymap.new_folder = bind,
                        }
                        if let Err(e) = app.keymap.save() {
                            app.status_message = Some(e);
                        }
                    }
                    app.listening_bind = None;
                }
            }
        }
        return; // Don't process normal shortcuts while listening
    }

    // Normal keyboard handling — driven by keymap
    ctx.input(|i| {
        // Category keys
        let num_cats = app.subdirs.len().min(app.keymap.category_keys.len());
        for idx in 0..num_cats {
            let bind = &app.keymap.category_keys[idx];
            if i.key_pressed(bind.key) && mods_match(i.modifiers, bind.modifiers) {
                if let Some(dir) = app.subdirs.get(idx).cloned() {
                    if let Err(e) = app.move_current(&dir) {
                        app.status_message = Some(e);
                    }
                }
                return;
            }
        }

        // Skip
        if i.key_pressed(app.keymap.skip.key)
            && mods_match(i.modifiers, app.keymap.skip.modifiers)
        {
            app.skip_current();
            return;
        }

        // Undo
        if i.key_pressed(app.keymap.undo.key)
            && mods_match(i.modifiers, app.keymap.undo.modifiers)
        {
            if let Err(e) = app.undo() {
                app.status_message = Some(e);
            }
            return;
        }

        // New folder
        if i.key_pressed(app.keymap.new_folder.key)
            && mods_match(i.modifiers, app.keymap.new_folder.modifiers)
        {
            app.show_new_folder_popup = true;
            app.new_folder_name.clear();
            app.new_folder_error = None;
        }
    });
}

fn mods_match(actual: egui::Modifiers, expected: egui::Modifiers) -> bool {
    actual.ctrl == expected.ctrl && actual.shift == expected.shift && actual.alt == expected.alt
}

fn render_new_folder_popup(app: &mut App, ctx: &Context) {
    let mut open = true;
    egui::Window::new("New folder")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(250.0)
        .show(ctx, |ui| {
            ui.label("Folder name:");
            let response = ui.text_edit_singleline(&mut app.new_folder_name);
            // Auto-focus on first frame
            response.request_focus();

            if let Some(err) = &app.new_folder_error {
                ui.colored_label(Color32::RED, err.as_str());
            }

            ui.horizontal(|ui| {
                if ui.button("Create").clicked()
                    || (response.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                {
                    match app.create_subfolder(&app.new_folder_name.clone()) {
                        Ok(()) => {
                            app.show_new_folder_popup = false;
                            app.new_folder_name.clear();
                            app.new_folder_error = None;
                        }
                        Err(e) => {
                            app.new_folder_error = Some(e);
                        }
                    }
                }
                if ui.button("Cancel").clicked() {
                    app.show_new_folder_popup = false;
                    app.new_folder_name.clear();
                    app.new_folder_error = None;
                }
            });
        });
    if !open {
        app.show_new_folder_popup = false;
        app.new_folder_name.clear();
        app.new_folder_error = None;
    }
    // Close on Escape
    ctx.input(|i| {
        if i.key_pressed(egui::Key::Escape) {
            app.show_new_folder_popup = false;
            app.new_folder_name.clear();
            app.new_folder_error = None;
        }
    });
}

fn render_keymap_editor(app: &mut App, ctx: &Context) {
    let mut open = app.show_keymap_editor;
    egui::Window::new("Keybindings")
        .open(&mut open)
        .resizable(true)
        .default_width(350.0)
        .show(ctx, |ui| {
            ui.label("Click a binding to remap it. Press Escape to cancel.");
            ui.separator();

            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    // Category bindings
                    let num_cats = app.subdirs.len().min(app.keymap.category_keys.len());
                    for i in 0..num_cats {
                        let dir_name = app
                            .subdirs
                            .get(i)
                            .and_then(|d| d.file_name().map(|n| n.to_string_lossy().to_string()))
                            .unwrap_or_else(|| format!("Category {}", i + 1));
                        let is_listening = app.listening_bind == Some(BindTarget::Category(i));
                        let btn_text = if is_listening {
                            "Press a key...".to_string()
                        } else {
                            app.keymap.category_keys[i].label.clone()
                        };
                        ui.horizontal(|ui| {
                            let btn = egui::Button::new(
                                RichText::new(&btn_text).monospace(),
                            )
                            .min_size(egui::vec2(90.0, 0.0));
                            let btn = if is_listening {
                                btn.fill(Color32::from_rgb(60, 60, 120))
                            } else {
                                btn
                            };
                            if ui.add(btn).clicked() {
                                app.listening_bind = Some(BindTarget::Category(i));
                                app.status_message = None;
                            }
                            ui.label(format!("📁 {dir_name}"));
                        });
                    }

                    ui.separator();

                    // Skip binding
                    let is_listening_skip = app.listening_bind == Some(BindTarget::Skip);
                    let skip_text = if is_listening_skip {
                        "Press a key...".to_string()
                    } else {
                        app.keymap.skip.label.clone()
                    };
                    ui.horizontal(|ui| {
                        let btn = egui::Button::new(
                            RichText::new(&skip_text).monospace(),
                        )
                        .min_size(egui::vec2(90.0, 0.0));
                        let btn = if is_listening_skip {
                            btn.fill(Color32::from_rgb(60, 60, 120))
                        } else {
                            btn
                        };
                        if ui.add(btn).clicked() {
                            app.listening_bind = Some(BindTarget::Skip);
                            app.status_message = None;
                        }
                        ui.label("⏭ Skip");
                    });

                    // Undo binding
                    let is_listening_undo = app.listening_bind == Some(BindTarget::Undo);
                    let undo_text = if is_listening_undo {
                        "Press a key...".to_string()
                    } else {
                        app.keymap.undo.label.clone()
                    };
                    ui.horizontal(|ui| {
                        let btn = egui::Button::new(
                            RichText::new(&undo_text).monospace(),
                        )
                        .min_size(egui::vec2(90.0, 0.0));
                        let btn = if is_listening_undo {
                            btn.fill(Color32::from_rgb(60, 60, 120))
                        } else {
                            btn
                        };
                        if ui.add(btn).clicked() {
                            app.listening_bind = Some(BindTarget::Undo);
                            app.status_message = None;
                        }
                        ui.label("↩ Undo");
                    });

                    // New folder binding
                    let is_listening_nf = app.listening_bind == Some(BindTarget::NewFolder);
                    let nf_text = if is_listening_nf {
                        "Press a key...".to_string()
                    } else {
                        app.keymap.new_folder.label.clone()
                    };
                    ui.horizontal(|ui| {
                        let btn = egui::Button::new(
                            RichText::new(&nf_text).monospace(),
                        )
                        .min_size(egui::vec2(90.0, 0.0));
                        let btn = if is_listening_nf {
                            btn.fill(Color32::from_rgb(60, 60, 120))
                        } else {
                            btn
                        };
                        if ui.add(btn).clicked() {
                            app.listening_bind = Some(BindTarget::NewFolder);
                            app.status_message = None;
                        }
                        ui.label("📁+ New folder");
                    });
                });

            ui.separator();
            if ui.button("Reset to defaults").clicked() {
                app.keymap = keymap::Keymap::default();
                app.listening_bind = None;
                if let Err(e) = app.keymap.save() {
                    app.status_message = Some(e);
                }
            }
        });
    app.show_keymap_editor = open;
    if !open {
        app.listening_bind = None;
    }
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
