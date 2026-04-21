use eframe::egui;
use std::fs::File;
use std::io::BufWriter;

use crate::svg_rendering::{generate_svg, infer_bounds, infer_extent};
use crate::types::Piece;

pub struct PreviewState {
    preview_image: Option<egui::ColorImage>,
    preview_texture: Option<egui::TextureHandle>,
    needs_preview_update: bool,
    max_preview_size: u32,
    override_max: bool,
    /// Animation start time (monotonic).
    animation_start: std::time::Instant,
}

impl PreviewState {
    pub fn new() -> Self {
        Self {
            preview_image: None,
            preview_texture: None,
            needs_preview_update: true,
            max_preview_size: 1920,
            override_max: false,
            animation_start: std::time::Instant::now(),
        }
    }

    pub fn mark_dirty(&mut self) {
        self.needs_preview_update = true;
    }

    /// Current animation frame value (0.0..inf, wraps via modular arithmetic in color functions).
    pub fn animation_frame(&self) -> f64 {
        self.animation_start.elapsed().as_secs_f64()
    }

    pub fn generate_svg(&self, pieces: &[Piece]) -> String {
        let (extent_x, extent_y) = infer_extent(pieces);
        let (extent_x, extent_y) = if self.override_max {
            (extent_x.cast_unsigned(), extent_y.cast_unsigned())
        } else {
            let max = self.max_preview_size;
            (
                extent_x.cast_unsigned().min(max),
                extent_y.cast_unsigned().min(max),
            )
        };
        generate_svg(extent_x, extent_y, pieces)
    }

    pub fn generate_svg_full(&self, pieces: &[Piece]) -> String {
        let (extent_x, extent_y) = infer_extent(pieces);
        generate_svg(extent_x.cast_unsigned(), extent_y.cast_unsigned(), pieces)
    }

    pub fn update(&mut self, ctx: &egui::Context, pieces: &[Piece]) {
        let has_animation = pieces.iter().any(|p| has_animated_color(p));
        let has_overrides = pieces.iter().any(|p| needs_color_override(p));

        if !self.needs_preview_update && !has_animation {
            return;
        }
        if egui::DragAndDrop::has_any_payload(ctx) {
            return;
        }
        self.needs_preview_update = false;

        // Apply color overrides for non-Solid types (Eraser, Rainbow, Gradient)
        let effective_pieces: Vec<Piece>;
        let pieces_ref = if has_overrides {
            if has_animation {
                ctx.request_repaint();
            }
            let frame = self.animation_frame();
            effective_pieces = apply_color_override(pieces, frame);
            &effective_pieces
        } else {
            pieces
        };

        let svg = self.generate_svg(pieces_ref);
        let tree = match resvg::usvg::Tree::from_str(
            &svg,
            &resvg::usvg::Options::default(),
        ) {
            Ok(t) => t,
            Err(_) => return,
        };

        let size = tree.size();
        let w = size.width() as u32;
        let h = size.height() as u32;

        if w == 0 || h == 0 {
            return;
        }

        let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::default(),
            &mut pixmap.as_mut(),
        );

        self.preview_image = Some(egui::ColorImage::from_rgba_unmultiplied(
            [w as usize, h as usize],
            &pixmap.data(),
        ));
    }

    pub fn texture(&mut self, ctx: &egui::Context, pieces: &[Piece]) -> Option<egui::TextureHandle> {
        self.update(ctx, pieces);

        if let Some(image) = &self.preview_image {
            let needs_new = match &self.preview_texture {
                None => true,
                Some(tex) => tex.size() != image.size,
            };
            if needs_new {
                self.preview_texture = Some(ctx.load_texture(
                    "crosshair_preview",
                    image.clone(),
                    egui::TextureOptions::NEAREST,
                ));
            } else if let Some(tex) = &mut self.preview_texture {
                tex.set_partial([0, 0], image.clone(), egui::TextureOptions::NEAREST);
            }
        }

        self.preview_texture.clone()
    }

    pub fn max_preview_size(&self) -> u32 {
        self.max_preview_size
    }

    pub fn override_max(&self) -> bool {
        self.override_max
    }

    pub fn set_override_max(&mut self, val: bool) {
        self.override_max = val;
        self.mark_dirty();
    }
}

// ── thumbnail helpers ───────────────────────────────────────────────

/// Zero out origins so the piece renders centered for thumbnails.
fn zero_origin(piece: &mut Piece) {
    match piece {
        Piece::Cross { origin, .. }
        | Piece::Dot { origin, .. }
        | Piece::Line { origin, .. }
        | Piece::Rectangle { origin, .. }
        | Piece::HappyFace { origin, .. } => *origin = (0, 0),
        Piece::RectPattern { origin, obj, .. } | Piece::CircPattern { origin, obj, .. } => {
            *origin = (0, 0);
            zero_origin(obj);
        }
    }
}

/// Render a single piece, tightly cropped to its content.
/// Origins are zeroed so the piece renders centered, then the output
/// is cropped to the minimum bounding box (no wasted transparent space).
fn render_piece_image(piece: &Piece, frame: f64) -> Option<egui::ColorImage> {
    if !piece.is_visible() {
        return None;
    }
    let mut centered = piece.clone();
    // Apply color override for non-Solid types
    if needs_color_override(&centered) {
        let color = centered.get_animated_color(frame);
        centered.set_color_override(&color);
    }
    zero_origin(&mut centered);
    let pieces = [centered];

    // Get tight content bounds (relative to SVG center)
    let (bmin_x, bmin_y, bmax_x, bmax_y) = infer_bounds(&pieces);
    if bmax_x <= bmin_x || bmax_y <= bmin_y {
        return None;
    }

    // Generate SVG at symmetric extent (guaranteed to contain everything)
    let (ex, ey) = infer_extent(&pieces);
    if ex <= 0 || ey <= 0 {
        return None;
    }
    let svg = generate_svg(ex.cast_unsigned(), ey.cast_unsigned(), &pieces);
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).ok()?;

    // SVG center in pixel coords
    let cx = ex as f64;
    let cy = ey as f64;

    // Content region in SVG pixel coords (with 1px padding)
    let left = (cx + bmin_x).floor() as i32 - 1;
    let top = (cy + bmin_y).floor() as i32 - 1;
    let right = (cx + bmax_x).ceil() as i32 + 1;
    let bottom = (cy + bmax_y).ceil() as i32 + 1;

    let crop_w = (right - left).max(1) as u32;
    let crop_h = (bottom - top).max(1) as u32;

    // Render with translation so the content region starts at (0,0)
    let mut pixmap = resvg::tiny_skia::Pixmap::new(crop_w, crop_h)?;
    let transform = resvg::tiny_skia::Transform::from_translate(
        -(left as f32),
        -(top as f32),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    Some(egui::ColorImage::from_rgba_unmultiplied(
        [crop_w as usize, crop_h as usize],
        pixmap.data(),
    ))
}

// ── piece thumbnail cache ───────────────────────────────────────────

/// Caches per-piece thumbnails, only re-rendering when a piece changes.
pub struct PieceThumbnailCache {
    entries: Vec<Option<(String, egui::TextureHandle)>>,
}

impl PieceThumbnailCache {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Compare each piece's serialized form to the cache; only re-render
    /// the ones that actually changed (or animated pieces every frame).
    pub fn update(&mut self, ctx: &egui::Context, pieces: &[Piece], frame: f64) {
        // Grow / shrink to match piece count
        self.entries.resize_with(pieces.len(), || None);
        self.entries.truncate(pieces.len());

        for (i, piece) in pieces.iter().enumerate() {
            let animated = needs_color_override(piece);
            let key = serde_json::to_string(piece).unwrap_or_default();
            let needs_update = animated || match &self.entries[i] {
                Some((cached_key, _)) => *cached_key != key,
                None => true,
            };
            if needs_update {
                self.entries[i] = render_piece_image(piece, frame).map(|img| {
                    let tex = ctx.load_texture(
                        format!("piece_thumb_{i}"),
                        img,
                        egui::TextureOptions::NEAREST,
                    );
                    (key, tex)
                });
            }
        }
    }

    pub fn get(&self, index: usize) -> Option<&egui::TextureHandle> {
        self.entries.get(index)?.as_ref().map(|(_, tex)| tex)
    }
}

// ── recent crosshair thumbnail cache ────────────────────────────────

/// Render the full crosshair (all pieces composited) as a small thumbnail.
fn render_crosshair_thumbnail(pieces: &[Piece], frame: f64) -> Option<egui::ColorImage> {
    if pieces.is_empty() {
        return None;
    }
    let effective = apply_color_override(pieces, frame);
    let (ex, ey) = infer_extent(&effective);
    if ex <= 0 || ey <= 0 {
        return None;
    }
    let svg = generate_svg(ex.cast_unsigned(), ey.cast_unsigned(), &effective);
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).ok()?;
    let size = tree.size();
    let w = size.width() as u32;
    let h = size.height() as u32;
    if w == 0 || h == 0 {
        return None;
    }
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
    resvg::render(&tree, resvg::tiny_skia::Transform::default(), &mut pixmap.as_mut());
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [w as usize, h as usize],
        pixmap.data(),
    ))
}

/// Caches recent crosshair thumbnails, loading pieces from project JSON
/// and re-rendering animated ones each frame.
pub struct RecentThumbnailCache {
    entries: std::collections::HashMap<std::path::PathBuf, RecentEntry>,
}

struct RecentEntry {
    pieces: Vec<Piece>,
    has_animation: bool,
    texture: Option<egui::TextureHandle>,
    key: String,
}

impl RecentThumbnailCache {
    pub fn new() -> Self {
        Self { entries: std::collections::HashMap::new() }
    }

    pub fn update(&mut self, ctx: &egui::Context, paths: &[std::path::PathBuf], frame: f64) {
        // Remove entries for paths no longer in the list
        self.entries.retain(|k, _| paths.contains(k));

        for path in paths {
            let entry = self.entries.entry(path.clone()).or_insert_with(|| {
                let pieces = crate::project_io::load_project(path)
                    .map(|p| p.pieces)
                    .unwrap_or_default();
                let has_animation = pieces.iter().any(|p| has_animated_color(p));
                let key = serde_json::to_string(&pieces).unwrap_or_default();
                RecentEntry { pieces, has_animation, texture: None, key }
            });

            let needs_update = entry.has_animation || entry.texture.is_none();
            if needs_update {
                if let Some(img) = render_crosshair_thumbnail(&entry.pieces, frame) {
                    entry.texture = Some(ctx.load_texture(
                        format!("recent_thumb_{}", path.display()),
                        img,
                        egui::TextureOptions::NEAREST,
                    ));
                }
            }
        }
    }

    /// Override the cached pieces for a path with live data (e.g. the currently-open project).
    /// This ensures the thumbnail reflects unsaved edits.
    pub fn set_live_pieces(&mut self, path: &std::path::Path, pieces: &[Piece]) {
        let has_animation = pieces.iter().any(|p| has_animated_color(p));
        let key = serde_json::to_string(pieces).unwrap_or_default();
        let entry = self.entries.entry(path.to_path_buf()).or_insert_with(|| {
            RecentEntry { pieces: Vec::new(), has_animation: false, texture: None, key: String::new() }
        });
        if entry.key != key {
            entry.pieces = pieces.to_vec();
            entry.has_animation = has_animation;
            entry.key = key;
            entry.texture = None; // Force re-render
        }
    }

    pub fn get(&self, path: &std::path::Path) -> Option<&egui::TextureHandle> {
        self.entries.get(path)?.texture.as_ref()
    }

    pub fn invalidate(&mut self, path: &std::path::PathBuf) {
        self.entries.remove(path);
    }
}

/// Save SVG and PNG/APNG exports alongside a project JSON path.
pub fn save_exports(json_path: &std::path::Path, pieces: &[Piece]) {
    let (extent_x, extent_y) = infer_extent(pieces);
    if extent_x <= 0 || extent_y <= 0 {
        return;
    }

    // Check if any piece has animated colors
    let has_animation = pieces.iter().any(|p| {
        matches!(
            p.color_type(),
            crate::types::ColorType::Rainbow { .. } | crate::types::ColorType::GradientCycle { .. }
        )
    });

    if has_animation {
        save_animated_export(json_path, pieces, extent_x, extent_y);
    } else {
        save_static_export(json_path, pieces, extent_x, extent_y);
    }
}

fn has_animated_color(piece: &Piece) -> bool {
    matches!(
        piece.color_type(),
        crate::types::ColorType::Rainbow { .. } | crate::types::ColorType::GradientCycle { .. }
    )
}

fn needs_color_override(piece: &Piece) -> bool {
    !matches!(piece.color_type(), crate::types::ColorType::Solid)
}

fn apply_color_override(pieces: &[Piece], frame: f64) -> Vec<Piece> {
    pieces
        .iter()
        .map(|p| {
            if needs_color_override(p) {
                let mut new_piece = p.clone();
                let animated_color = p.get_animated_color(frame);
                new_piece.set_color_override(&animated_color);
                new_piece
            } else {
                p.clone()
            }
        })
        .collect()
}

fn save_static_export(json_path: &std::path::Path, pieces: &[Piece], extent_x: i32, extent_y: i32) {
    // Apply non-Solid overrides (e.g. Eraser → transparent)
    let effective = apply_color_override(pieces, 0.0);
    let svg = generate_svg(extent_x.cast_unsigned(), extent_y.cast_unsigned(), &effective);
    let _ = std::fs::write(json_path.with_extension("svg"), &svg);

    if let Ok(tree) = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()) {
        let size = tree.size();
        let w = size.width() as u32;
        let h = size.height() as u32;
        if w > 0 && h > 0 {
            if let Some(mut pixmap) = resvg::tiny_skia::Pixmap::new(w, h) {
                resvg::render(
                    &tree,
                    resvg::tiny_skia::Transform::default(),
                    &mut pixmap.as_mut(),
                );
                let _ = image::save_buffer(
                    json_path.with_extension("png"),
                    pixmap.data(),
                    w,
                    h,
                    image::ColorType::Rgba8,
                );
            }
        }
    }
    // Delete the apng if it exists (mutually exclusive)
    let _ = std::fs::remove_file(json_path.with_extension("apng"));
}

fn save_animated_export(json_path: &std::path::Path, pieces: &[Piece], extent_x: i32, extent_y: i32) {
    let num_frames: u32 = 30;
    let w = extent_x as u32 * 2;
    let h = extent_y as u32 * 2;

    // Save SVG (static)
    let svg = generate_svg(extent_x.cast_unsigned(), extent_y.cast_unsigned(), pieces);
    let _ = std::fs::write(json_path.with_extension("svg"), &svg);

    // Generate frames
    let mut frames: Vec<Vec<u8>> = Vec::new();
    for i in 0..num_frames {
        let frame = i as f64 / num_frames as f64;
        let colored_pieces = apply_color_override(pieces, frame);
        let svg = generate_svg(extent_x.cast_unsigned(), extent_y.cast_unsigned(), &colored_pieces);

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

    // Save as APNG
    if !frames.is_empty() {
        let apng_path = json_path.with_extension("apng");
        if let Ok(file) = File::create(&apng_path) {
            let mut buf_writer = BufWriter::new(file);
            let mut encoder = png::Encoder::new(&mut buf_writer, w, h);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_animated(num_frames, 0).unwrap();
            encoder.set_frame_delay(1, 50).unwrap();

            if let Ok(mut png_writer) = encoder.write_header() {
                for frame_data in &frames {
                    let _ = png_writer.write_image_data(frame_data);
                }
            };
        }
    }

    // Delete the png if it exists (mutually exclusive)
    let _ = std::fs::remove_file(json_path.with_extension("png"));
}

// ── preview panel ───────────────────────────────────────────────────

pub fn render_preview_panel(ui: &mut egui::Ui, ctx: &egui::Context, preview: &mut PreviewState, pieces: &[Piece]) {
    let (extent_x, extent_y) = infer_extent(pieces);
    let width = extent_x.cast_unsigned() * 2;
    let height = extent_y.cast_unsigned() * 2;

    let max_size = preview.max_preview_size() * 2;
    let is_capped = !preview.override_max() && (width > max_size || height > max_size);

    ui.horizontal(|ui| {
        ui.label(format!("Size: {width}x{height}"));
        if is_capped {
            let mut ov = preview.override_max();
            let cb = ui.checkbox(&mut ov, egui::RichText::new("Override max size?").color(egui::Color32::RED));
            if cb.changed() {
                preview.set_override_max(ov);
            }
        }
    });

    let available_size = ui.available_size();

    if let Some(texture) = preview.texture(ctx, pieces) {
        let tex_size = texture.size();
        let aspect = tex_size[0] as f32 / tex_size[1] as f32;
        let avail_w = available_size.x;
        let avail_h = available_size.y;
        let (img_w, img_h) = if avail_w / avail_h > aspect {
            (avail_h * aspect, avail_h)
        } else {
            (avail_w, avail_w / aspect)
        };
        let img_size = egui::Vec2::new(img_w, img_h);
        let (rect, _response) = ui.allocate_exact_size(img_size, egui::Sense::hover());
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::ZERO,
            egui::Color32::from_gray(20),
        );
        ui.painter().image(
            texture.id(),
            rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    } else {
        let (rect, _) = ui.allocate_exact_size(available_size, egui::Sense::hover());
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::ZERO,
            egui::Color32::from_gray(20),
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No preview available",
            egui::FontId::default(),
            egui::Color32::GRAY,
        );
    }
}
