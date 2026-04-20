use eframe::egui;

use crate::svg_rendering::{generate_svg, infer_bounds, infer_extent};
use crate::types::Piece;

pub struct PreviewState {
    preview_image: Option<egui::ColorImage>,
    preview_texture: Option<egui::TextureHandle>,
    needs_preview_update: bool,
    max_preview_size: u32,
    override_max: bool,
}

impl PreviewState {
    pub fn new() -> Self {
        Self {
            preview_image: None,
            preview_texture: None,
            needs_preview_update: true,
            max_preview_size: 1920,
            override_max: false,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.needs_preview_update = true;
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
        if !self.needs_preview_update {
            return;
        }
        if egui::DragAndDrop::has_any_payload(ctx) {
            return;
        }
        self.needs_preview_update = false;

        let svg = self.generate_svg(pieces);
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
fn render_piece_image(piece: &Piece) -> Option<egui::ColorImage> {
    if !piece.is_visible() {
        return None;
    }
    let mut centered = piece.clone();
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
    /// the ones that actually changed.
    pub fn update(&mut self, ctx: &egui::Context, pieces: &[Piece]) {
        // Grow / shrink to match piece count
        self.entries.resize_with(pieces.len(), || None);
        self.entries.truncate(pieces.len());

        for (i, piece) in pieces.iter().enumerate() {
            let key = serde_json::to_string(piece).unwrap_or_default();
            let needs_update = match &self.entries[i] {
                Some((cached_key, _)) => *cached_key != key,
                None => true,
            };
            if needs_update {
                self.entries[i] = render_piece_image(piece).map(|img| {
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

/// Load a crosshair PNG from disk (sibling to a .json project file) as a thumbnail.
pub fn load_crosshair_thumbnail(json_path: &std::path::Path, max_size: u32) -> Option<egui::ColorImage> {
    let png_path = json_path.with_extension("png");
    let data = std::fs::read(&png_path).ok()?;
    let img = image::load_from_memory_with_format(&data, image::ImageFormat::Png).ok()?;
    let img = img.thumbnail(max_size, max_size);
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [w as usize, h as usize],
        rgba.as_raw(),
    ))
}

/// Generate SVG string and render it to a Pixmap. Returns None if rendering fails.
pub fn render_png(pieces: &[Piece]) -> Option<(String, resvg::tiny_skia::Pixmap)> {
    let (extent_x, extent_y) = infer_extent(pieces);
    if extent_x <= 0 || extent_y <= 0 {
        return None;
    }
    let svg = generate_svg(extent_x.cast_unsigned(), extent_y.cast_unsigned(), pieces);
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).ok()?;
    let size = tree.size();
    let w = size.width() as u32;
    let h = size.height() as u32;
    if w == 0 || h == 0 {
        return None;
    }
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );
    Some((svg, pixmap))
}

/// Save SVG and PNG exports alongside a project JSON path.
pub fn save_exports(json_path: &std::path::Path, pieces: &[Piece]) {
    let (extent_x, extent_y) = infer_extent(pieces);
    if extent_x <= 0 || extent_y <= 0 {
        return;
    }
    let svg = generate_svg(extent_x.cast_unsigned(), extent_y.cast_unsigned(), pieces);

    // Save SVG
    let _ = std::fs::write(json_path.with_extension("svg"), &svg);

    // Save PNG
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
