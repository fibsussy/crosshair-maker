use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

fn default_one() -> f64 { 1.0 }

/// Legacy enum kept only for deserializing old project files.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum GradientTransition {
    Loop,
    Bounce,
    SmoothLoop,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum LoopMode {
    /// Reverse direction at each end (ping-pong).
    Bounce,
    /// Cycle through colors in one direction, wrapping around.
    Cycle,
}

impl Default for LoopMode {
    fn default() -> Self {
        LoopMode::Bounce
    }
}

impl std::fmt::Display for LoopMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoopMode::Bounce => write!(f, "Bounce"),
            LoopMode::Cycle => write!(f, "Cycle"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum InterpolationMode {
    /// Smoothly interpolate between adjacent color stops.
    Smooth,
    /// Snap instantly to each color stop (no blending).
    Instant,
}

impl Default for InterpolationMode {
    fn default() -> Self {
        InterpolationMode::Smooth
    }
}

impl std::fmt::Display for InterpolationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterpolationMode::Smooth => write!(f, "Smooth"),
            InterpolationMode::Instant => write!(f, "Instant Cuts"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ColorType {
    Solid,
    Eraser,
    Rainbow {
        #[serde(default = "default_one")]
        saturation: f64,
        #[serde(default = "default_one")]
        lightness: f64,
        #[serde(default = "default_one")]
        alpha: f64,
        #[serde(default = "default_one")]
        speed: f64,
        #[serde(default)]
        reverse: bool,
    },
    GradientCycle {
        #[serde(default)]
        colors: Vec<String>,
        #[serde(default = "default_one")]
        speed: f64,
        #[serde(default)]
        loop_mode: LoopMode,
        #[serde(default)]
        interpolation: InterpolationMode,
        /// Legacy field — migrated into `loop_mode`/`interpolation` on load.
        #[serde(default)]
        transition: Option<GradientTransition>,
        /// Legacy field — migrated into `colors` on load.
        #[serde(default)]
        color2: Option<String>,
    },
}

impl Default for ColorType {
    fn default() -> Self {
        ColorType::Solid
    }
}

impl ColorType {
    /// Migrate legacy fields on load.
    pub fn migrate(&mut self) {
        if let ColorType::GradientCycle { colors, color2, transition, loop_mode, interpolation, .. } = self {
            // Migrate legacy color2 into colors vec
            if let Some(c2) = color2.take() {
                if colors.is_empty() {
                    colors.push(c2);
                }
            }
            // Migrate legacy transition enum into loop_mode + interpolation
            if let Some(t) = transition.take() {
                match t {
                    GradientTransition::Bounce => {
                        *loop_mode = LoopMode::Bounce;
                        *interpolation = InterpolationMode::Smooth;
                    }
                    GradientTransition::Loop | GradientTransition::SmoothLoop => {
                        *loop_mode = LoopMode::Cycle;
                        *interpolation = InterpolationMode::Smooth;
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum OddAnchor {
    #[default]
    TopRight,
    TopLeft,
    BottomLeft,
    BottomRight,
    Center,
}

impl OddAnchor {
    pub fn offset(self) -> (f64, f64) {
        match self {
            OddAnchor::TopRight => (0.0, -1.0),
            OddAnchor::TopLeft => (-1.0, -1.0),
            OddAnchor::BottomLeft => (-1.0, 0.0),
            OddAnchor::BottomRight => (0.0, 0.0),
            OddAnchor::Center => (-0.5, -0.5),
        }
    }
}

impl std::fmt::Display for OddAnchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OddAnchor::TopRight => write!(f, "TopRight"),
            OddAnchor::TopLeft => write!(f, "TopLeft"),
            OddAnchor::BottomLeft => write!(f, "BottomLeft"),
            OddAnchor::BottomRight => write!(f, "BottomRight"),
            OddAnchor::Center => write!(f, "Center"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Piece {
    Cross {
        origin: (i32, i32),
        h_gap: i32,
        v_gap: i32,
        length: i32,
        thickness: i32,
        color: String,
        #[serde(default)]
        color_type: ColorType,
        visible: bool,
        #[serde(default)]
        odd_anchor: OddAnchor,
        #[serde(default = "default_true")]
        lock_gap: bool,
    },
    Dot {
        origin: (i32, i32),
        size: u32,
        color: String,
        #[serde(default)]
        color_type: ColorType,
        visible: bool,
        #[serde(default)]
        odd_anchor: OddAnchor,
    },
    Line {
        origin: (i32, i32),
        vector: (i32, i32),
        thickness: i32,
        color: String,
        #[serde(default)]
        color_type: ColorType,
        visible: bool,
        #[serde(default)]
        odd_anchor: OddAnchor,
    },
    Rectangle {
        origin: (i32, i32),
        width: u32,
        height: u32,
        rotation: f64,
        color: String,
        #[serde(default)]
        color_type: ColorType,
        visible: bool,
        #[serde(default)]
        odd_anchor: OddAnchor,
    },
    RectPattern {
        origin: (i32, i32),
        x_distance: i32,
        x_quantity: u32,
        y_distance: i32,
        y_quantity: u32,
        obj: Box<Self>,
        visible: bool,
    },
    CircPattern {
        origin: (i32, i32),
        radius: i32,
        quantity: u32,
        start_deg: f64,
        obj: Box<Self>,
        visible: bool,
    },
    HappyFace {
        origin: (i32, i32),
        size: u32,
        color: String,
        #[serde(default)]
        color_type: ColorType,
        visible: bool,
        #[serde(default)]
        odd_anchor: OddAnchor,
    },
}

impl Piece {
    pub fn type_name(&self) -> &'static str {
        match self {
            Piece::Cross { .. } => "Cross",
            Piece::Dot { .. } => "Dot",
            Piece::Line { .. } => "Line",
            Piece::Rectangle { .. } => "Rectangle",
            Piece::RectPattern { .. } => "RectPattern",
            Piece::CircPattern { .. } => "CircPattern",
            Piece::HappyFace { .. } => "HappyFace",
        }
    }

    pub fn is_visible(&self) -> bool {
        match self {
            Piece::Cross { visible, .. }
            | Piece::Dot { visible, .. }
            | Piece::Line { visible, .. }
            | Piece::Rectangle { visible, .. }
            | Piece::RectPattern { visible, .. }
            | Piece::CircPattern { visible, .. }
            | Piece::HappyFace { visible, .. } => *visible,
        }
    }

    pub fn set_visible(&mut self, val: bool) {
        match self {
            Piece::Cross { visible, .. }
            | Piece::Dot { visible, .. }
            | Piece::Line { visible, .. }
            | Piece::Rectangle { visible, .. }
            | Piece::RectPattern { visible, .. }
            | Piece::CircPattern { visible, .. }
            | Piece::HappyFace { visible, .. } => *visible = val,
        }
    }

    pub fn odd_anchor(&self) -> OddAnchor {
        match self {
            Piece::Cross { odd_anchor, .. }
            | Piece::Dot { odd_anchor, .. }
            | Piece::Line { odd_anchor, .. }
            | Piece::Rectangle { odd_anchor, .. }
            | Piece::HappyFace { odd_anchor, .. } => *odd_anchor,
            // Patterns don't have their own anchor; inner obj does
            Piece::RectPattern { .. } | Piece::CircPattern { .. } => OddAnchor::default(),
        }
    }

    pub fn color_type(&self) -> ColorType {
        match self {
            Piece::Cross { color_type, .. }
            | Piece::Dot { color_type, .. }
            | Piece::Line { color_type, .. }
            | Piece::Rectangle { color_type, .. }
            | Piece::HappyFace { color_type, .. } => color_type.clone(),
            Piece::RectPattern { obj, .. } | Piece::CircPattern { obj, .. } => obj.color_type(),
        }
    }

    pub fn get_animated_color(&self, frame: f64) -> String {
        let ct = self.color_type();
        match ct {
            ColorType::Solid => self.base_color(),
            ColorType::Eraser => "#00000000".to_string(),
            ColorType::Rainbow { saturation, lightness, alpha, speed, reverse } => {
                let dir = if reverse { -1.0 } else { 1.0 };
                let hue = (frame * speed * dir * 360.0).rem_euclid(360.0);
                let (r, g, b) = hsv_to_rgb(hue, saturation, lightness);
                let a = (alpha.clamp(0.0, 1.0) * 255.0) as u8;
                format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
            }
            ColorType::GradientCycle { colors, speed, loop_mode, interpolation, .. } => {
                if colors.len() < 2 {
                    return colors.first().cloned().unwrap_or_else(|| "#ffffffff".to_string());
                }
                let n = colors.len();
                let (seg, local_t) = match loop_mode {
                    LoopMode::Bounce => {
                        // Ping-pong: 0→1→2→...→n-1→...→1→0
                        let segments = n - 1;
                        let total_len = segments as f64 * 2.0;
                        let t = (frame * speed) % total_len;
                        let t = if t > segments as f64 { total_len - t } else { t };
                        let seg = (t.floor() as usize).min(segments - 1);
                        (seg, t - seg as f64)
                    }
                    LoopMode::Cycle => {
                        // Cycle: 0→1→...→n-1→0 (wraps around)
                        let t = (frame * speed) % n as f64;
                        let seg = t.floor() as usize % n;
                        (seg, t - t.floor())
                    }
                };
                let next = match loop_mode {
                    LoopMode::Bounce => (seg + 1).min(n - 1),
                    LoopMode::Cycle => (seg + 1) % n,
                };
                let all_colors: Vec<&str> = colors.iter().map(|s| s.as_str()).collect();
                let c1 = parse_hex_color_rgba(all_colors[seg]);
                match interpolation {
                    InterpolationMode::Smooth => {
                        let c2 = parse_hex_color_rgba(all_colors[next]);
                        let r = ((c1.0 as f64 * (1.0 - local_t)) + (c2.0 as f64 * local_t)) as u8;
                        let g = ((c1.1 as f64 * (1.0 - local_t)) + (c2.1 as f64 * local_t)) as u8;
                        let b = ((c1.2 as f64 * (1.0 - local_t)) + (c2.2 as f64 * local_t)) as u8;
                        let a = ((c1.3 as f64 * (1.0 - local_t)) + (c2.3 as f64 * local_t)) as u8;
                        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
                    }
                    InterpolationMode::Instant => {
                        // Snap to current color stop, no blending
                        format!("#{:02x}{:02x}{:02x}{:02x}", c1.0, c1.1, c1.2, c1.3)
                    }
                }
            }
        }
    }

    fn base_color(&self) -> String {
        match self {
            Piece::Cross { color, .. }
            | Piece::Dot { color, .. }
            | Piece::Line { color, .. }
            | Piece::Rectangle { color, .. }
            | Piece::HappyFace { color, .. } => color.clone(),
            Piece::RectPattern { obj, .. } | Piece::CircPattern { obj, .. } => obj.base_color(),
        }
    }



    pub fn set_color_override(&mut self, color: &str) {
        match self {
            Piece::Cross { color: c, .. }
            | Piece::Dot { color: c, .. }
            | Piece::Line { color: c, .. }
            | Piece::Rectangle { color: c, .. }
            | Piece::HappyFace { color: c, .. } => *c = color.to_string(),
            Piece::RectPattern { obj, .. } | Piece::CircPattern { obj, .. } => obj.set_color_override(color),
        }
    }
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as i32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8)
}

/// Compute the duration (in seconds) of one full animation cycle for a piece.
/// Returns `None` if the piece has no animation.
pub fn animation_cycle_duration(piece: &Piece) -> Option<f64> {
    match piece.color_type() {
        ColorType::Rainbow { speed, .. } => Some(1.0 / speed),
        ColorType::GradientCycle { colors, speed, loop_mode, .. } => {
            let n = colors.len();
            if n < 2 { return None; }
            let cycle = match loop_mode {
                LoopMode::Bounce => (n - 1) as f64 * 2.0 / speed,
                LoopMode::Cycle => n as f64 / speed,
            };
            Some(cycle)
        }
        _ => None,
    }
}

/// Compute the maximum animation cycle duration across all pieces.
/// Returns 1.0 as a fallback if no animated pieces exist.
pub fn max_animation_cycle(pieces: &[Piece]) -> f64 {
    pieces.iter()
        .filter_map(|p| animation_cycle_duration(p))
        .fold(1.0_f64, f64::max)
}

fn parse_hex_color_rgba(hex: &str) -> (u8, u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    let a = if hex.len() >= 8 {
        u8::from_str_radix(&hex[6..8], 16).unwrap_or(255)
    } else {
        255
    };
    (r, g, b, a)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CrosshairProject {
    pub name: String,
    pub pieces: Vec<Piece>,
    /// Legacy field kept for backwards-compatible deserialization.
    #[serde(default)]
    pub odd_anchor: Option<OddAnchor>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub recent_crosshairs: Vec<std::path::PathBuf>,
    #[serde(default)]
    pub current_crosshair: Option<std::path::PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            recent_crosshairs: Vec::new(),
            current_crosshair: None,
        }
    }
}

pub fn default_pieces() -> Vec<Piece> {
    use Piece::{Cross, Dot, HappyFace, RectPattern};
    let default_color_type = ColorType::default();
    vec![
        RectPattern {
            origin: (0, -10),
            x_distance: 10,
            x_quantity: 1,
            y_distance: -10,
            y_quantity: 3,
            obj: Box::new(Dot {
                origin: (0, 0),
                size: 1,
                color: "#ff5050ff".to_string(),
                color_type: default_color_type.clone(),
                visible: true,
                odd_anchor: OddAnchor::default(),
            }),
            visible: true,
        },
        Cross {
            origin: (0, 0),
            h_gap: 2,
            v_gap: 2,
            length: 2,
            thickness: 2,
            color: "#00ff7dff".to_string(),
            color_type: default_color_type.clone(),
            visible: true,
            odd_anchor: OddAnchor::default(),
            lock_gap: true,
        },
        HappyFace {
            origin: (-50, -10),
            size: 3,
            color: "#00ff7dff".to_string(),
            color_type: default_color_type,
            visible: true,
            odd_anchor: OddAnchor::default(),
        },
    ]
}
