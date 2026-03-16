use egui::{Context, RichText, Color32};
use crate::app::App;
use crate::keymap::{self, BindTarget, FolderBindings};

pub fn render(app: &mut App, ctx: &Context) {
    handle_keyboard(app, ctx);

    // Toolbar
    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            let open_label = format!("[{}] 📂 Open folder…", app.keymap.open_folder.label);
            if ui.button(open_label).clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    app.open_folder(path);
                }
            }
            let kb_label = format!("[{}] ⚙ Keybindings", app.keymap.toggle_keybindings.label);
            if ui.button(kb_label).clicked() {
                app.show_keymap_editor = !app.show_keymap_editor;
                app.listening_bind = None;
            }
            let hist_label = format!("[{}] 📜 History", app.keymap.toggle_history.label);
            if ui.button(hist_label).clicked() {
                app.show_history = !app.show_history;
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

    // History panel
    if app.show_history {
        render_history_panel(app, ctx);
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
                for dir in &subdirs {
                    let dir_name = dir.file_name().unwrap_or_default().to_string_lossy();
                    let label = if let Some(bind) = app.folder_bindings.get(&dir_name) {
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
            let border = 2.0;
            let padded = egui::vec2(available.x - border * 2.0, available.y - border * 2.0);
            let scale = (padded.x / size.x).min(padded.y / size.y).min(1.0);
            let display_size = size * scale;
            ui.centered_and_justified(|ui| {
                let (rect, _) = ui.allocate_exact_size(
                    display_size + egui::vec2(border * 2.0, border * 2.0),
                    egui::Sense::hover(),
                );
                let border_rect = egui::Rect::from_center_size(rect.center(), display_size + egui::vec2(border * 2.0, border * 2.0));
                ui.painter().rect_stroke(border_rect, 0.0, egui::Stroke::new(border, Color32::YELLOW));
                let img_rect = egui::Rect::from_center_size(rect.center(), display_size);
                ui.painter().image(texture.id(), img_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
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
                    if let Some(conflict) = app.keymap.has_conflict(key, mods, Some(&target), &app.folder_bindings) {
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
                            BindTarget::Category(name) => {
                                app.folder_bindings.0.insert(name.clone(), bind);
                                let _ = app.folder_bindings.save(&app.folder);
                            }
                            BindTarget::Skip => app.keymap.skip = bind,
                            BindTarget::Undo => app.keymap.undo = bind,
                            BindTarget::NewFolder => app.keymap.new_folder = bind,
                            BindTarget::OpenFolder => app.keymap.open_folder = bind,
                            BindTarget::ToggleKeybindings => app.keymap.toggle_keybindings = bind,
                            BindTarget::ToggleHistory => app.keymap.toggle_history = bind,
                        }
                        if !matches!(&target, BindTarget::Category(_)) {
                            if let Err(e) = app.keymap.save() {
                                app.status_message = Some(e);
                            }
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
        // Category keys — iterate subdirs, look up by folder name
        for dir in &app.subdirs.clone() {
            let dir_name = dir.file_name().unwrap_or_default().to_string_lossy();
            if let Some(bind) = app.folder_bindings.get(&dir_name) {
                if i.key_pressed(bind.key) && mods_match(i.modifiers, bind.modifiers) {
                    if let Err(e) = app.move_current(dir) {
                        app.status_message = Some(e);
                    }
                    return;
                }
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
            return;
        }

        // Open folder
        if i.key_pressed(app.keymap.open_folder.key)
            && mods_match(i.modifiers, app.keymap.open_folder.modifiers)
        {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                app.open_folder(path);
            }
            return;
        }

        // Toggle keybindings
        if i.key_pressed(app.keymap.toggle_keybindings.key)
            && mods_match(i.modifiers, app.keymap.toggle_keybindings.modifiers)
        {
            app.show_keymap_editor = !app.show_keymap_editor;
            app.listening_bind = None;
            return;
        }

        // Toggle history
        if i.key_pressed(app.keymap.toggle_history.key)
            && mods_match(i.modifiers, app.keymap.toggle_history.modifiers)
        {
            app.show_history = !app.show_history;
        }
    });
}

fn mods_match(actual: egui::Modifiers, expected: egui::Modifiers) -> bool {
    actual.ctrl == expected.ctrl && actual.shift == expected.shift && actual.alt == expected.alt
}

fn render_new_folder_popup(app: &mut App, ctx: &Context) {
    let mut open = true;
    let mut should_create = false;

    // Check Enter key directly from ctx — fixes unreliable lost_focus pattern
    ctx.input(|i| {
        if i.key_pressed(egui::Key::Enter) {
            should_create = true;
        }
    });

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
                if ui.button("Create").clicked() {
                    should_create = true;
                }
                if ui.button("Cancel").clicked() {
                    app.show_new_folder_popup = false;
                    app.new_folder_name.clear();
                    app.new_folder_error = None;
                }
            });
        });

    if should_create {
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

fn render_history_panel(app: &mut App, ctx: &Context) {
    const THUMB_MAX: f32 = 256.0;
    const LABEL_HEIGHT: f32 = 30.0;

    // Load thumbnails at max resolution; we scale at render time
    while app.history_thumbs.len() < app.move_log.len() {
        let idx = app.history_thumbs.len();
        let op = &app.move_log[idx];
        let thumb = if crate::files::is_image(&op.to) {
            load_thumbnail(&op.to, ctx, THUMB_MAX).ok()
        } else {
            None
        };
        app.history_thumbs.push(thumb);
    }

    egui::TopBottomPanel::bottom("history_panel")
        .resizable(true)
        .min_height(80.0)
        .max_height(400.0)
        .default_height(140.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("📜 History").strong().size(12.0));
                ui.label(
                    RichText::new(format!("({})", app.move_log.len()))
                        .weak()
                        .size(11.0),
                );
            });
            let avail_h = ui.available_height();
            let img_h = (avail_h - LABEL_HEIGHT).max(32.0);

            if app.move_log.is_empty() {
                ui.colored_label(Color32::GRAY, "No moves yet.");
                // Fill remaining space so panel doesn't auto-shrink
                let rem = ui.available_size();
                ui.allocate_space(rem);
            } else {
                let scroll_h = avail_h;
                egui::ScrollArea::horizontal()
                    .id_source("history_hscroll")
                    .stick_to_right(true)
                    .min_scrolled_height(scroll_h)
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.set_min_height(scroll_h);
                            for (i, op) in app.move_log.iter().enumerate() {
                                let filename = op.from.file_name().unwrap_or_default().to_string_lossy();
                                let dest_dir = op.to.parent()
                                    .and_then(|p| p.file_name())
                                    .unwrap_or_default()
                                    .to_string_lossy();

                                ui.vertical(|ui| {
                                    // Thumbnail or icon — sized to fit panel height
                                    if let Some(Some(tex)) = app.history_thumbs.get(i) {
                                        let size = tex.size_vec2();
                                        let scale = (img_h / size.y).min(img_h / size.x).min(1.0);
                                        let display = size * scale;
                                        ui.set_width(display.x.max(50.0));
                                        ui.image((tex.id(), display));
                                    } else {
                                        ui.set_width(img_h.max(50.0));
                                        let icon_size = (img_h * 0.5).min(48.0);
                                        let (icon, _) = file_icon_and_size(&op.to);
                                        ui.centered_and_justified(|ui| {
                                            ui.label(RichText::new(icon).size(icon_size));
                                        });
                                    }
                                    // Folder label
                                    ui.label(
                                        RichText::new(truncate_str(&dest_dir, 12))
                                            .color(Color32::LIGHT_BLUE)
                                            .size(10.0),
                                    );
                                    // Filename
                                    ui.label(
                                        RichText::new(truncate_str(&filename, 12))
                                            .weak()
                                            .size(9.0),
                                    );
                                });
                                ui.add_space(4.0);
                            }
                        });
                    });
            }
        });
}

fn load_thumbnail(path: &std::path::Path, ctx: &Context, max_dim: f32) -> Result<egui::TextureHandle, image::ImageError> {
    let img = image::open(path)?;
    let thumb = img.thumbnail(max_dim as u32, max_dim as u32).to_rgba8();
    let (w, h) = thumb.dimensions();
    let pixels = thumb.into_raw();
    let color_image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &pixels);
    Ok(ctx.load_texture(
        format!("thumb_{}", path.display()),
        color_image,
        egui::TextureOptions::default(),
    ))
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
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
                    // --- Folders (local) section ---
                    ui.heading(RichText::new("Folders (local)").size(13.0));
                    ui.label(RichText::new("Per-folder bindings stored alongside your files").weak().size(11.0));
                    ui.add_space(4.0);

                    let subdirs = app.subdirs.clone();
                    for dir in &subdirs {
                        let dir_name = dir
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let is_listening = app.listening_bind == Some(BindTarget::Category(dir_name.clone()));
                        let btn_text = if is_listening {
                            "Press a key...".to_string()
                        } else if let Some(bind) = app.folder_bindings.get(&dir_name) {
                            bind.label.clone()
                        } else {
                            "—".to_string()
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
                                app.listening_bind = Some(BindTarget::Category(dir_name.clone()));
                                app.status_message = None;
                            }
                            ui.label(format!("📁 {dir_name}"));
                        });
                    }

                    if subdirs.is_empty() {
                        ui.colored_label(Color32::GRAY, "No subfolders yet");
                    }

                    ui.add_space(8.0);
                    ui.separator();

                    // --- Actions (global) section ---
                    ui.heading(RichText::new("Actions (global)").size(13.0));
                    ui.label(RichText::new("Stored in ~/.config/categorizer/keymap.json").weak().size(11.0));
                    ui.add_space(4.0);

                    render_global_bind_row(ui, app, BindTarget::Skip, "⏭ Skip", &app.keymap.skip.label.clone());
                    render_global_bind_row(ui, app, BindTarget::Undo, "↩ Undo", &app.keymap.undo.label.clone());
                    render_global_bind_row(ui, app, BindTarget::NewFolder, "📁+ New folder", &app.keymap.new_folder.label.clone());
                    render_global_bind_row(ui, app, BindTarget::OpenFolder, "📂 Open folder", &app.keymap.open_folder.label.clone());
                    render_global_bind_row(ui, app, BindTarget::ToggleKeybindings, "⚙ Keybindings", &app.keymap.toggle_keybindings.label.clone());
                    render_global_bind_row(ui, app, BindTarget::ToggleHistory, "📜 History", &app.keymap.toggle_history.label.clone());
                });

            ui.separator();
            if ui.button("Reset to defaults").clicked() {
                app.keymap = keymap::Keymap::default();
                app.listening_bind = None;
                // Delete local folder bindings and re-auto-assign
                FolderBindings::delete(&app.folder);
                let mut fb = FolderBindings::default();
                fb.ensure_bound(&app.subdirs, &app.keymap);
                let _ = fb.save(&app.folder);
                app.folder_bindings = fb;
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

fn render_global_bind_row(ui: &mut egui::Ui, app: &mut App, target: BindTarget, action_label: &str, current_label: &str) {
    let is_listening = app.listening_bind == Some(target.clone());
    let btn_text = if is_listening {
        "Press a key...".to_string()
    } else {
        current_label.to_string()
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
            app.listening_bind = Some(target);
            app.status_message = None;
        }
        ui.label(action_label);
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
