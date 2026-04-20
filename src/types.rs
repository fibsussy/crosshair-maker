use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
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
        visible: bool,
        #[serde(default)]
        odd_anchor: OddAnchor,
    },
    Line {
        origin: (i32, i32),
        vector: (i32, i32),
        thickness: i32,
        color: String,
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
            visible: true,
            odd_anchor: OddAnchor::default(),
            lock_gap: true,
        },
        HappyFace {
            origin: (-910, -400),
            size: 3,
            color: "#00ff7dff".to_string(),
            visible: true,
            odd_anchor: OddAnchor::default(),
        },
    ]
}
