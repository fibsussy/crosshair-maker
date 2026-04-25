use eframe::egui;
use std::fs::File;
use std::io::BufWriter;

use crate::svg_rendering::{generate_svg, infer_bounds, infer_extent};
use crate::types::Piece;

/// Available background images for the live dynamic effects preview.
#[derive(Clone, PartialEq)]
pub enum PreviewBackground {
    None,
    CSGO,
    TheFinals,
}

impl PreviewBackground {
    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::CSGO => "CSGO",
            Self::TheFinals => "The Finals",
        }
    }
}

pub struct PreviewState {
    preview_image: Option<egui::ColorImage>,
    preview_texture: Option<egui::TextureHandle>,
    needs_preview_update: bool,
    max_preview_size: u32,
    override_max: bool,
    /// Animation start time (monotonic).
    animation_start: std::time::Instant,
    /// Selected background for live preview.
    pub selected_bg: PreviewBackground,
    /// Decoded background image (cached).
    bg_image: Option<(PreviewBackground, image::RgbaImage)>,
    /// Composite texture (background + crosshair + effects).
    composite_texture: Option<egui::TextureHandle>,
    /// Zoom level (1.0 = default).
    zoom_level: f32,
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
            selected_bg: PreviewBackground::None,
            bg_image: None,
            composite_texture: None,
            zoom_level: 1.0,
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

    /// Get or load the selected background image.
    fn ensure_bg_image(&mut self) -> Option<&image::RgbaImage> {
        if self.selected_bg == PreviewBackground::None {
            self.bg_image = None;
            return None;
        }
        // Check if cached bg matches selection
        if let Some((ref cached_bg, _)) = self.bg_image {
            if *cached_bg == self.selected_bg {
                return self.bg_image.as_ref().map(|(_, img)| img);
            }
        }
        // Load the background
        let bytes: Option<&[u8]> = match self.selected_bg {
            PreviewBackground::CSGO => {
                Some(include_bytes!("../assets/preview_backgrounds/csgo.png"))
            }
            PreviewBackground::TheFinals => {
                Some(include_bytes!("../assets/preview_backgrounds/thefinals.png"))
            }
            PreviewBackground::None => None,
        };
        if let Some(data) = bytes {
            if let Ok(img) = image::load_from_memory(data) {
                self.bg_image = Some((self.selected_bg.clone(), img.to_rgba8()));
                return self.bg_image.as_ref().map(|(_, img)| img);
            }
        }
        self.bg_image = None;
        None
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
pub fn save_exports(json_path: &std::path::Path, pieces: &[Piece], effects: &crate::types::DynamicEffects) {
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

    // Export dynamic mask + config
    save_dynamic_export(json_path, pieces, effects, extent_x, extent_y);
}

fn is_dynamic(piece: &Piece) -> bool {
    matches!(piece.color_type(), crate::types::ColorType::Dynamic { .. })
}

/// Save dynamic mask + config file.
///
/// Exports:
/// - `{stem}.dynamic.png` — grayscale binary mask (white where Dynamic pieces are)
/// - `{stem}.dynamic.cfg` — effect chain configuration (one line per enabled effect)
///
/// Cleans up old per-mode mask files from the previous architecture.
fn save_dynamic_export(
    json_path: &std::path::Path,
    pieces: &[Piece],
    effects: &crate::types::DynamicEffects,
    extent_x: i32,
    extent_y: i32,
) {
    let stem = json_path.file_stem().unwrap_or_default().to_string_lossy();
    let mask_path = json_path.with_file_name(format!("{stem}.dynamic.png"));
    let cfg_path = json_path.with_file_name(format!("{stem}.dynamic.cfg"));

    let has_dyn = pieces.iter().any(|p| is_dynamic(p));

    if !has_dyn || !effects.has_any_enabled() {
        // Clean up dynamic files
        let _ = std::fs::remove_file(&mask_path);
        let _ = std::fs::remove_file(&cfg_path);
        // Clean up legacy per-mode mask files
        for tag in crate::types::ALL_LEGACY_MODE_TAGS {
            let _ = std::fs::remove_file(json_path.with_file_name(format!("{stem}.mask.{tag}.png")));
        }
        let _ = std::fs::remove_file(json_path.with_file_name(format!("{stem}.mask.png")));
        let _ = std::fs::remove_file(json_path.with_file_name(format!("{stem}.mask.apng")));
        return;
    }

    let w = extent_x as u32 * 2;
    let h = extent_y as u32 * 2;

    // Build binary mask: Dynamic pieces = white, everything else = hidden
    let mask_pieces: Vec<Piece> = pieces.iter().map(|p| {
        let mut mp = p.clone();
        if is_dynamic(p) {
            mp.set_color_override("#ffffffff"); // white = apply effects
        } else {
            mp.set_visible(false);
        }
        mp
    }).collect();

    let svg = generate_svg(extent_x.cast_unsigned(), extent_y.cast_unsigned(), &mask_pieces);
    if let Some(rgba_data) = rasterize_svg(&svg, w, h) {
        // Convert RGBA to grayscale (take alpha channel as the mask value)
        let gray: Vec<u8> = rgba_data.chunks(4).map(|px| px[3]).collect();
        let _ = image::save_buffer(&mask_path, &gray, w, h, image::ColorType::L8);
    }

    // Write config file
    let cfg = effects.to_cfg_string();
    let _ = std::fs::write(&cfg_path, &cfg);

    // Clean up legacy per-mode mask files
    for tag in crate::types::ALL_LEGACY_MODE_TAGS {
        let _ = std::fs::remove_file(json_path.with_file_name(format!("{stem}.mask.{tag}.png")));
    }
    let _ = std::fs::remove_file(json_path.with_file_name(format!("{stem}.mask.png")));
    let _ = std::fs::remove_file(json_path.with_file_name(format!("{stem}.mask.apng")));
}

/// Helper: rasterize an SVG string to RGBA pixel data.
fn rasterize_svg(svg: &str, w: u32, h: u32) -> Option<Vec<u8>> {
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
    resvg::render(&tree, resvg::tiny_skia::Transform::default(), &mut pixmap.as_mut());
    Some(pixmap.data().to_vec())
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
    apply_color_override_inner(pieces, frame, false)
}

/// Like `apply_color_override` but for export: ContrastInvert pieces
/// are rendered as transparent so they don't appear in the main image.
fn apply_color_override_for_export(pieces: &[Piece], frame: f64) -> Vec<Piece> {
    apply_color_override_inner(pieces, frame, true)
}

fn apply_color_override_inner(pieces: &[Piece], frame: f64, export_mode: bool) -> Vec<Piece> {
    pieces
        .iter()
        .map(|p| {
            if export_mode && is_dynamic(p) {
                // In export mode, Dynamic → transparent (handled by mask)
                let mut new_piece = p.clone();
                new_piece.set_color_override("#00000000");
                new_piece
            } else if needs_color_override(p) {
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
    // Apply non-Solid overrides (e.g. Eraser → transparent, ContrastInvert → transparent)
    let effective = apply_color_override_for_export(pieces, 0.0);
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

/// Target FPS for APNG export. Frame values use real-time seconds
/// so the exported animation matches the live preview speed.
const EXPORT_FPS: u32 = 30;
/// Maximum frames per APNG to keep file sizes reasonable.
const EXPORT_MAX_FRAMES: u32 = 300;

fn save_animated_export(json_path: &std::path::Path, pieces: &[Piece], extent_x: i32, extent_y: i32) {
    let w = extent_x as u32 * 2;
    let h = extent_y as u32 * 2;

    // Compute cycle duration so the APNG covers exactly one full loop
    let cycle_duration = crate::types::max_animation_cycle(pieces);
    let num_frames: u32 = ((cycle_duration * EXPORT_FPS as f64).round() as u32)
        .max(2)
        .min(EXPORT_MAX_FRAMES);
    let frame_delay_secs = cycle_duration / num_frames as f64;

    // APNG frame delay as integer ratio: delay_num / delay_den seconds.
    // Use millisecond precision: delay_num = round(delay_ms), delay_den = 1000.
    let delay_den: u16 = 1000;
    let delay_num: u16 = ((frame_delay_secs * 1000.0).round() as u16).max(1);

    // Save SVG (static)
    let svg = generate_svg(extent_x.cast_unsigned(), extent_y.cast_unsigned(), pieces);
    let _ = std::fs::write(json_path.with_extension("svg"), &svg);

    // Generate frames with real-time second values
    let mut frames: Vec<Vec<u8>> = Vec::new();
    for i in 0..num_frames {
        let frame_time = i as f64 * frame_delay_secs;
        let colored_pieces = apply_color_override_for_export(pieces, frame_time);
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
        let actual_frames = frames.len() as u32;
        let apng_path = json_path.with_extension("apng");
        if let Ok(file) = File::create(&apng_path) {
            let mut buf_writer = BufWriter::new(file);
            let mut encoder = png::Encoder::new(&mut buf_writer, w, h);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_animated(actual_frames, 0).unwrap();
            encoder.set_frame_delay(delay_num, delay_den).unwrap();

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

pub fn render_preview_panel(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    preview: &mut PreviewState,
    pieces: &[Piece],
    effects: &crate::types::DynamicEffects,
) {
    let (extent_x, extent_y) = infer_extent(pieces);
    let width = extent_x.cast_unsigned() * 2;
    let height = extent_y.cast_unsigned() * 2;

    let max_size = preview.max_preview_size() * 2;
    let is_capped = !preview.override_max() && (width > max_size || height > max_size);

    // Top controls row
    ui.horizontal(|ui| {
        ui.label(format!("Size: {width}x{height}"));
        if is_capped {
            let mut ov = preview.override_max();
            let cb = ui.checkbox(&mut ov, egui::RichText::new("Override max size?").color(egui::Color32::RED));
            if cb.changed() {
                preview.set_override_max(ov);
            }
        }
        ui.separator();
        ui.label("Background:");
        let bg_label = preview.selected_bg.label();
        egui::ComboBox::from_id_salt("preview_bg")
            .selected_text(bg_label)
            .show_ui(ui, |ui| {
                if ui.selectable_label(preview.selected_bg == PreviewBackground::None, "None").clicked() {
                    preview.selected_bg = PreviewBackground::None;
                    preview.mark_dirty();
                }
                if ui.selectable_label(preview.selected_bg == PreviewBackground::CSGO, "CSGO").clicked() {
                    preview.selected_bg = PreviewBackground::CSGO;
                    preview.mark_dirty();
                }
                if ui.selectable_label(preview.selected_bg == PreviewBackground::TheFinals, "The Finals").clicked() {
                    preview.selected_bg = PreviewBackground::TheFinals;
                    preview.mark_dirty();
                }
            });
    });

    let has_bg = preview.selected_bg != PreviewBackground::None;
    let has_dynamic = pieces.iter().any(|p| is_dynamic(p));

    // Handle scroll-wheel zoom (only when hovering over the preview area)
    let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
    if scroll_delta != 0.0 {
        let factor = 1.0 + scroll_delta * 0.005;
        preview.zoom_level = (preview.zoom_level * factor).max(0.01);
    }

    // Show zoom level + reset button
    ui.horizontal(|ui| {
        ui.label(format!("Zoom: {:.0}%", preview.zoom_level * 100.0));
        if ui.small_button("Reset").clicked() {
            preview.zoom_level = 1.0;
        }
    });

    let available_size = ui.available_size();
    let zoom = preview.zoom_level;

    if has_bg {
        // Live compositing mode: background + crosshair + dynamic effects
        ctx.request_repaint(); // Always repaint for mouse tracking

        // Get the crosshair texture for overlay
        let _crosshair_tex = preview.texture(ctx, pieces);

        // Allocate the preview rect
        let (rect, response) = ui.allocate_exact_size(available_size, egui::Sense::hover());
        let mouse_in_rect = response.hovered();
        let mouse_pos = response.hover_pos();

        // Hide cursor when hovering
        if mouse_in_rect {
            ui.ctx().set_cursor_icon(egui::CursorIcon::None);
        }

        // Get background image dimensions
        let bg_img = preview.ensure_bg_image().cloned();
        if let Some(bg) = bg_img {
            let bg_w = bg.width();
            let bg_h = bg.height();

            // Compute composite: background centered and zoomed, crosshair at mouse or center
            let composite_w = rect.width() as u32;
            let composite_h = rect.height() as u32;
            if composite_w > 0 && composite_h > 0 {
                // Zoomed background dimensions
                let zbg_w = (bg_w as f32 * zoom) as i32;
                let zbg_h = (bg_h as f32 * zoom) as i32;
                let bg_ox = (composite_w as i32 - zbg_w) / 2;
                let bg_oy = (composite_h as i32 - zbg_h) / 2;

                // Crosshair position (center of crosshair at mouse or at center of rect)
                // Crosshair is NOT scaled — always native pixel size
                let ch_w = width;
                let ch_h = height;
                let (ch_cx, ch_cy) = if let Some(mpos) = mouse_pos {
                    let local = mpos - rect.min;
                    (local.x as i32, local.y as i32)
                } else {
                    (composite_w as i32 / 2, composite_h as i32 / 2)
                };
                let ch_ox = ch_cx - ch_w as i32 / 2;
                let ch_oy = ch_cy - ch_h as i32 / 2;

                let frame = preview.animation_frame();
                let ext_x = extent_x.cast_unsigned().min(preview.max_preview_size());
                let ext_y = extent_y.cast_unsigned().min(preview.max_preview_size());
                let raster_w = ch_w.min(max_size);
                let raster_h = ch_h.min(max_size);

                // Rasterize crosshair for export (dynamic → transparent, keeps piece order)
                let crosshair_rgba = {
                    let eff = apply_color_override_for_export(pieces, frame);
                    let svg = generate_svg(ext_x, ext_y, &eff);
                    rasterize_svg(&svg, raster_w, raster_h)
                };

                // Rasterize binary mask (dynamic pieces = white, rest hidden)
                let mask_rgba = if has_dynamic && effects.has_any_enabled() {
                    let mask_pieces: Vec<Piece> = pieces.iter().map(|p| {
                        let mut mp = p.clone();
                        if is_dynamic(p) {
                            mp.set_color_override("#ffffffff");
                        } else {
                            mp.set_visible(false);
                        }
                        mp
                    }).collect();
                    let svg = generate_svg(ext_x, ext_y, &mask_pieces);
                    rasterize_svg(&svg, raster_w, raster_h)
                } else {
                    None
                };

                let actual_ch_w = raster_w;
                let actual_ch_h = raster_h;

                // Build composite: background (zoomed) → dynamic effects → crosshair overlay (native)
                let mut pixels = vec![0u8; (composite_w * composite_h * 4) as usize];
                for y in 0..composite_h {
                    for x in 0..composite_w {
                        let pi = ((y * composite_w + x) * 4) as usize;

                        // 1. Background pixel (sample from zoomed coordinates)
                        let bx_f = (x as f32 - bg_ox as f32) / zoom;
                        let by_f = (y as f32 - bg_oy as f32) / zoom;
                        let bx = bx_f as i32;
                        let by = by_f as i32;
                        let (mut pr, mut pg, mut pb) = if bx_f >= 0.0 && by_f >= 0.0 && bx >= 0 && by >= 0 && bx < bg_w as i32 && by < bg_h as i32 {
                            let px = bg.get_pixel(bx as u32, by as u32);
                            (px[0], px[1], px[2])
                        } else {
                            (20u8, 20u8, 20u8)
                        };

                        // Check if this pixel falls within the crosshair bounds
                        let cx = x as i32 - ch_ox;
                        let cy = y as i32 - ch_oy;
                        if cx >= 0 && cy >= 0 && cx < actual_ch_w as i32 && cy < actual_ch_h as i32 {
                            let ci = ((cy as u32 * actual_ch_w + cx as u32) * 4) as usize;

                            // 2. Dynamic effects (where mask is white, transform bg pixel)
                            if let Some(ref mask) = mask_rgba {
                                if ci + 3 < mask.len() && mask[ci + 3] > 127 {
                                    let (er, eg, eb) = effects.apply_to_pixel(
                                        pr as f32 / 255.0,
                                        pg as f32 / 255.0,
                                        pb as f32 / 255.0,
                                    );
                                    pr = (er * 255.0).clamp(0.0, 255.0) as u8;
                                    pg = (eg * 255.0).clamp(0.0, 255.0) as u8;
                                    pb = (eb * 255.0).clamp(0.0, 255.0) as u8;
                                }
                            }

                            // 3. Crosshair overlay (alpha-blend)
                            if let Some(ref ch) = crosshair_rgba {
                                if ci + 3 < ch.len() {
                                    let ca = ch[ci + 3] as f32 / 255.0;
                                    if ca > 0.001 {
                                        let sr = ch[ci] as f32 / 255.0;
                                        let sg = ch[ci + 1] as f32 / 255.0;
                                        let sb = ch[ci + 2] as f32 / 255.0;
                                        pr = ((sr + pr as f32 / 255.0 * (1.0 - ca)) * 255.0).clamp(0.0, 255.0) as u8;
                                        pg = ((sg + pg as f32 / 255.0 * (1.0 - ca)) * 255.0).clamp(0.0, 255.0) as u8;
                                        pb = ((sb + pb as f32 / 255.0 * (1.0 - ca)) * 255.0).clamp(0.0, 255.0) as u8;
                                    }
                                }
                            }
                        }

                        pixels[pi] = pr;
                        pixels[pi + 1] = pg;
                        pixels[pi + 2] = pb;
                        pixels[pi + 3] = 255;
                    }
                }

                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [composite_w as usize, composite_h as usize],
                    &pixels,
                );

                // Upload composite texture
                let tex = match &mut preview.composite_texture {
                    Some(tex) if tex.size() == image.size => {
                        tex.set_partial([0, 0], image, egui::TextureOptions::NEAREST);
                        tex.clone()
                    }
                    _ => {
                        let tex = ctx.load_texture("preview_composite", image, egui::TextureOptions::NEAREST);
                        preview.composite_texture = Some(tex.clone());
                        tex
                    }
                };

                ui.painter().image(
                    tex.id(),
                    rect,
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        } else {
            // Background selected but failed to load
            ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, egui::Color32::from_gray(20));
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "Background failed to load", egui::FontId::default(), egui::Color32::GRAY);
        }
    } else {
        // No background — scale crosshair with zoom
        if let Some(texture) = preview.texture(ctx, pieces) {
            let tex_size = texture.size();
            let aspect = tex_size[0] as f32 / tex_size[1] as f32;
            let avail_w = available_size.x;
            let avail_h = available_size.y;
            // Fit to available space, then apply zoom
            let (base_w, base_h) = if avail_w / avail_h > aspect {
                (avail_h * aspect, avail_h)
            } else {
                (avail_w, avail_w / aspect)
            };
            let img_w = base_w * zoom;
            let img_h = base_h * zoom;

            // Use a scroll area so zoomed-in crosshair can be panned
            egui::ScrollArea::both()
                .max_width(avail_w)
                .max_height(avail_h)
                .show(ui, |ui| {
                    let img_size = egui::Vec2::new(img_w, img_h);
                    let (rect, _response) = ui.allocate_exact_size(img_size, egui::Sense::hover());
                    ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, egui::Color32::from_gray(20));
                    ui.painter().image(
                        texture.id(), rect,
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                });
        } else {
            let (rect, _) = ui.allocate_exact_size(available_size, egui::Sense::hover());
            ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, egui::Color32::from_gray(20));
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "No preview available", egui::FontId::default(), egui::Color32::GRAY);
        }
    }
}
