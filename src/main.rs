#![allow(clippy::too_many_lines, clippy::many_single_char_names)]

mod types;
mod svg_rendering;
mod project_io;
mod preview;
mod ui_pieces;
mod ui_properties;

use std::path::PathBuf;

use eframe::egui;

use types::{AppConfig, CrosshairProject, Piece};
use preview::PreviewState;

struct CrosshairApp {
    pieces: Vec<Piece>,
    selected_indices: Vec<usize>,
    status_message: String,
    project_name: String,
    current_file_path: Option<PathBuf>,
    config: AppConfig,
    preview: PreviewState,
    show_new_dialog: bool,
    show_save_as_dialog: bool,
    show_delete_confirm: bool,
    new_project_name: String,
    piece_thumbnails: preview::PieceThumbnailCache,
    recent_thumbnails: preview::RecentThumbnailCache,
}

impl CrosshairApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Set monospace as the default font for the entire app.
        let mut style = (*cc.egui_ctx.style()).clone();
        for text_style in [
            egui::TextStyle::Body,
            egui::TextStyle::Monospace,
            egui::TextStyle::Button,
            egui::TextStyle::Heading,
            egui::TextStyle::Small,
        ] {
            let size = style.text_styles[&text_style].size;
            style.text_styles.insert(
                text_style,
                egui::FontId::new(size, egui::FontFamily::Monospace),
            );
        }
        cc.egui_ctx.set_style(style);
        let config = project_io::load_config();
        Self {
            pieces: types::default_pieces(),
            selected_indices: Vec::new(),
            status_message: String::new(),
            project_name: "Untitled".to_string(),
            current_file_path: None,
            config,
            preview: PreviewState::new(),
            show_new_dialog: false,
            show_save_as_dialog: false,
            show_delete_confirm: false,
            new_project_name: String::new(),
            piece_thumbnails: preview::PieceThumbnailCache::new(),
            recent_thumbnails: preview::RecentThumbnailCache::new(),
        }
    }

    fn update_piece_thumbnails(&mut self, ctx: &egui::Context) {
        let frame = self.preview.animation_frame();
        self.piece_thumbnails.update(ctx, &self.pieces, frame);
    }

    fn load_recent_thumbnails(&mut self, ctx: &egui::Context) {
        // Keep the currently-open project's thumbnail in sync with live edits
        if let Some(ref path) = self.current_file_path {
            self.recent_thumbnails.set_live_pieces(path, &self.pieces);
        }
        let frame = self.preview.animation_frame();
        self.recent_thumbnails.update(ctx, &self.config.recent_crosshairs, frame);
    }

    fn invalidate_recent_thumbnail(&mut self, path: &PathBuf) {
        self.recent_thumbnails.invalidate(path);
    }

    fn save_with_exports(&self, path: &std::path::Path) {
        preview::save_exports(path, &self.pieces);
        self.update_current_if_matches(path);
    }

    fn update_current_if_matches(&self, path: &std::path::Path) {
        if self.config.current_crosshair.as_ref().map(|p| p.as_path()) == Some(path) {
            project_io::save_current_exports(&self.pieces);
        }
    }

    fn set_as_current(&mut self, path: PathBuf) {
        project_io::save_current_exports(&self.pieces);
        self.config.current_crosshair = Some(path);
        project_io::save_config(&self.config);
    }

    fn export_svg(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("crosshair.svg")
            .save_file()
        {
            let svg = self.preview.generate_svg_full(&self.pieces);
            if let Err(e) = std::fs::write(&path, &svg) {
                self.status_message = format!("Error: {e}");
            } else {
                self.status_message =
                    format!("Saved SVG to {}", path.display());
            }
        }
    }

    fn export_png(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("crosshair.png")
            .save_file()
        {
            let svg = self.preview.generate_svg_full(&self.pieces);
            let tree = resvg::usvg::Tree::from_str(
                &svg,
                &resvg::usvg::Options::default(),
            );
            match tree {
                Ok(tree) => {
                    let size = tree.size();
                    let mut pixmap =
                        resvg::tiny_skia::Pixmap::new(
                            size.width() as u32,
                            size.height() as u32,
                        )
                        .unwrap();
                    resvg::render(
                        &tree,
                        resvg::tiny_skia::Transform::default(),
                        &mut pixmap.as_mut(),
                    );
                    if let Err(e) = image::save_buffer(
                        &path,
                        pixmap.data(),
                        pixmap.width(),
                        pixmap.height(),
                        image::ColorType::Rgba8,
                    ) {
                        self.status_message = format!("Error: {e}");
                    } else {
                        self.status_message =
                            format!("Saved PNG to {}", path.display());
                    }
                }
                Err(e) => {
                    self.status_message = format!("SVG parse error: {e}");
                }
            }
        }
    }

    fn export_apng(&mut self) {
        use std::fs::File;
        use std::io::BufWriter;
        use crate::svg_rendering::{generate_svg, infer_extent};

        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("crosshair.apng")
            .save_file()
        {
            let (extent_x, extent_y) = infer_extent(&self.pieces);
            let w = (extent_x * 2).max(1) as u32;
            let h = (extent_y * 2).max(1) as u32;
            let num_frames: u32 = 30;

            let has_animation = self.pieces.iter().any(|p| {
                matches!(
                    p.color_type(),
                    crate::types::ColorType::Rainbow { .. } | crate::types::ColorType::GradientCycle { .. }
                )
            });

            if !has_animation {
                self.status_message = "No animated colors to export as APNG".to_string();
                return;
            }

            let mut frames: Vec<Vec<u8>> = Vec::new();
            for i in 0..num_frames {
                let frame = i as f64 / num_frames as f64;
                let colored_pieces: Vec<crate::types::Piece> = self.pieces
                    .iter()
                    .map(|p| {
                        if !matches!(p.color_type(), crate::types::ColorType::Solid) {
                            let mut new_piece = p.clone();
                            let animated_color = p.get_animated_color(frame);
                            new_piece.set_color_override(&animated_color);
                            new_piece
                        } else {
                            p.clone()
                        }
                    })
                    .collect();

                let svg = generate_svg(w, h, &colored_pieces);
                if let Ok(tree) = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()) {
                    if let Some(mut pixmap) = resvg::tiny_skia::Pixmap::new(w, h) {
                        resvg::render(
                            &tree,
                            resvg::tiny_skia::Transform::default(),
                            &mut pixmap.as_mut(),
                        );
                        frames.push(pixmap.data().to_vec());
                    }
                }
            }

            if frames.is_empty() {
                self.status_message = "Failed to generate APNG frames".to_string();
                return;
            }

            if let Ok(file) = File::create(&path) {
                let mut buf_writer = BufWriter::new(file);
                let result = {
                    let mut encoder = png::Encoder::new(&mut buf_writer, w, h);
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    encoder.set_animated(num_frames, 0).unwrap();
                    encoder.set_frame_delay(1, 50).unwrap();

                    match encoder.write_header() {
                        Ok(mut png_writer) => {
                            for frame_data in &frames {
                                let _ = png_writer.write_image_data(frame_data);
                            }
                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                };
                match result {
                    Ok(()) => self.status_message = format!("Saved APNG to {}", path.display()),
                    Err(e) => self.status_message = format!("APNG encode error: {e}"),
                }
            } else {
                self.status_message = "Failed to create APNG file".to_string();
            }
        }
    }

    fn save_project(&mut self) {
        if let Some(ref path) = self.current_file_path.clone() {
            let project = CrosshairProject {
                name: self.project_name.clone(),
                pieces: self.pieces.clone(),
                odd_anchor: None,
            };
            if project_io::save_project(&project, &mut self.config, Some(path.clone())).is_some() {
                self.save_with_exports(path);
                self.invalidate_recent_thumbnail(path);
                self.status_message = format!("Saved to {}", path.display());
            } else {
                self.status_message = "Failed to save project".to_string();
            }
        } else {
            self.show_save_as_dialog = true;
        }
    }

    fn save_project_as(&mut self) {
        self.show_save_as_dialog = true;
    }

    fn do_save_as(&mut self, name: &str) {
        if name.is_empty() {
            return;
        }
        let project = CrosshairProject {
            name: name.to_string(),
            pieces: self.pieces.clone(),
            odd_anchor: None,
        };
        self.project_name = name.to_string();
        if let Some(path) = project_io::save_project(&project, &mut self.config, None) {
            self.save_with_exports(&path);
            self.invalidate_recent_thumbnail(&path);
            self.current_file_path = Some(path.clone());
            self.status_message = format!("Saved as '{}' to {}", name, path.display());
        } else {
            self.status_message = "Failed to save project".to_string();
        }
        self.show_save_as_dialog = false;
    }

    fn new_project(&mut self, name: &str) {
        self.pieces = types::default_pieces();
        self.project_name = name.to_string();
        self.current_file_path = None;
        self.selected_indices.clear();
        self.preview = PreviewState::new();
        self.piece_thumbnails = preview::PieceThumbnailCache::new();
        self.show_new_dialog = false;
        self.status_message = format!("New project '{}'", name);
    }

    fn open_project(&mut self, path: PathBuf) {
        if let Some(project) = project_io::load_project(&path) {
            let name = project.name.clone();
            self.pieces = project.pieces;
            self.project_name = name.clone();
            self.current_file_path = Some(path.clone());
            self.selected_indices.clear();
            self.preview = PreviewState::new();
            self.piece_thumbnails = preview::PieceThumbnailCache::new();
            project_io::add_to_recent(&mut self.config, path.clone());
            self.status_message = format!("Opened '{}'", name);
        } else {
            self.status_message = format!("Failed to open {}", path.display());
        }
    }
}

impl eframe::App for CrosshairApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── keyboard shortcuts ──────────────────────────────────
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            self.save_project();
        }

        // ── thumbnail generation ────────────────────────────────
        self.update_piece_thumbnails(ctx);
        self.load_recent_thumbnails(ctx);

        let mut open_path: Option<PathBuf> = None;
        let mut do_save = false;
        let mut do_save_as = false;
        let mut do_export_svg = false;
        let mut do_export_png = false;
        let mut do_export_apng = false;
        let mut do_set_current: Option<PathBuf> = None;

        egui::SidePanel::left("pieces_panel")
            .resizable(true)
            .default_width(320.0)
            .show(ctx, |ui| {
                let mut mark_dirty = false;
                let current_ch = self.config.current_crosshair.clone();
                ui_pieces::render_pieces_panel(
                    ui,
                    ctx,
                    &mut self.pieces,
                    &mut self.selected_indices,
                    &mut self.status_message,
                    &self.project_name,
                    &self.current_file_path,
                    &current_ch,
                    &mut self.config,
                    &mut self.show_new_dialog,
                    &mut self.show_save_as_dialog,
                    &mut self.new_project_name,
                    &self.piece_thumbnails,
                    &self.recent_thumbnails,
                    |path| open_path = Some(path),
                    || do_save = true,
                    || do_save_as = true,
                    || do_export_svg = true,
                    || do_export_png = true,
                    || do_export_apng = true,
                    || self.show_delete_confirm = true,
                    |path| do_set_current = Some(path),
                );
                if !self.selected_indices.is_empty() {
                    mark_dirty = true;
                }
                if mark_dirty {
                    self.preview.mark_dirty();
                }
            });

        if let Some(path) = open_path {
            self.open_project(path);
        }
        if do_save {
            self.save_project();
        }
        if do_save_as {
            self.save_project_as();
        }
        if do_export_svg {
            self.export_svg();
        }
        if do_export_png {
            self.export_png();
        }
        if do_export_apng {
            self.export_apng();
        }
        if let Some(path) = do_set_current {
            self.open_project(path.clone());
            self.set_as_current(path);
            self.save_project();
        }

        egui::SidePanel::right("properties_panel")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("Properties");
                ui.separator();

                if self.selected_indices.len() == 1 {
                    let idx = self.selected_indices[0];
                    if let Some(piece) = self.pieces.get_mut(idx) {
                        if ui_properties::edit_piece(ui, piece) {
                            self.preview.mark_dirty();
                        }
                    }
                } else if self.selected_indices.len() > 1 {
                    ui.label(format!("{} pieces selected", self.selected_indices.len()));
                    ui.separator();
                    ui.label("Hidden Properties (Multi-Select)");
                    ui.small("Edit properties for each selected piece individually.");
                } else {
                    ui.label("Select a piece to edit");
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Preview");
            ui.separator();

            preview::render_preview_panel(ui, ctx, &mut self.preview, &self.pieces);
        });

        if self.show_new_dialog {
            let mut create = false;
            let mut cancel = false;
            egui::Window::new("New Project")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Project name:");
                    ui.text_edit_singleline(&mut self.new_project_name);
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            create = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
            if cancel {
                self.show_new_dialog = false;
            } else if create && !self.new_project_name.is_empty() {
                let name = self.new_project_name.clone();
                if name.eq_ignore_ascii_case("current") {
                    self.status_message = "Cannot use name 'current' — reserved".to_string();
                } else {
                    self.new_project(&name);
                }
            }
        }

        if self.show_save_as_dialog {
            let mut save = false;
            let mut cancel = false;
            egui::Window::new("Save As")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Project name:");
                    ui.text_edit_singleline(&mut self.new_project_name);
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            save = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
            if cancel {
                self.show_save_as_dialog = false;
            } else if save && !self.new_project_name.is_empty() {
                let name = self.new_project_name.clone();
                if name.eq_ignore_ascii_case("current") {
                    self.status_message = "Cannot use name 'current' — reserved".to_string();
                } else {
                    self.do_save_as(&name);
                }
            }
        }

        if self.show_delete_confirm {
            let mut do_delete = false;
            let mut cancel_delete = false;
            egui::Window::new("Confirm Delete")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Are you sure you want to remove the selected piece?");
                    ui.horizontal(|ui| {
                        if ui.button("Delete").clicked() {
                            do_delete = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel_delete = true;
                        }
                    });
                });
            if cancel_delete {
                self.show_delete_confirm = false;
            } else if do_delete {
                let mut indices: Vec<usize> = self.selected_indices.clone();
                indices.sort_unstable();
                indices.reverse();
                for idx in indices {
                    if idx < self.pieces.len() {
                        self.pieces.remove(idx);
                    }
                }
                self.selected_indices.clear();
                self.preview.mark_dirty();
                self.show_delete_confirm = false;
            }
        }
    }
}

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Crosshair Maker"),
        ..Default::default()
    };

    eframe::run_native(
        "Crosshair Maker",
        native_options,
        Box::new(|cc| Ok(Box::new(CrosshairApp::new(cc)))),
    )
}
