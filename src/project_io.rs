use std::path::PathBuf;

use crate::types::{AppConfig, CrosshairProject};

pub fn config_path() -> PathBuf {
    let mut path = dirs_config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("crosshair-maker");
    path.push("config.json");
    path
}

fn dirs_config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(dir));
        }
        if let Ok(home) = std::env::var("HOME") {
            return Some(PathBuf::from(home).join(".config"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            return Some(PathBuf::from(home).join("Library/Application Support"));
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return Some(PathBuf::from(appdata));
        }
    }
    None
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    if let Ok(data) = std::fs::read_to_string(&path) {
        if let Ok(config) = serde_json::from_str(&data) {
            return config;
        }
    }
    AppConfig::default()
}

pub fn save_config(config: &AppConfig) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(data) = serde_json::to_string_pretty(config) {
        let _ = std::fs::write(&path, data);
    }
}

pub fn add_to_recent(config: &mut AppConfig, path: PathBuf) {
    config.recent_crosshairs.retain(|p| p != &path);
    config.recent_crosshairs.insert(0, path);
    if config.recent_crosshairs.len() > 20 {
        config.recent_crosshairs.truncate(20);
    }
    save_config(config);
}

pub fn project_dir() -> PathBuf {
    let mut path = dirs_config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("crosshair-maker");
    path.push("projects");
    let _ = std::fs::create_dir_all(&path);
    path
}

/// Export "current" crosshair in the projects directory.
/// Saves as current.png (static) or current.apng (animated), deleting the other.
pub fn save_current_exports(pieces: &[crate::types::Piece], effects: &crate::types::DynamicEffects) {
    let dir = project_dir();
    let current_path = dir.join("current.json"); // virtual path for extension swapping
    crate::preview::save_exports(&current_path, pieces, effects);
}

pub fn save_project(project: &CrosshairProject, config: &mut AppConfig, path: Option<PathBuf>) -> Option<PathBuf> {
    let path = path.unwrap_or_else(|| {
        let mut p = project_dir();
        p.push(format!("{}.json", sanitize_filename(&project.name)));
        p
    });
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(data) = serde_json::to_string_pretty(project) {
        if std::fs::write(&path, data).is_ok() {
            add_to_recent(config, path.clone());
            return Some(path);
        }
    }
    None
}

pub fn load_project(path: &PathBuf) -> Option<CrosshairProject> {
    let data = std::fs::read_to_string(path).ok()?;
    let mut project: CrosshairProject = serde_json::from_str(&data).ok()?;
    // Migrate legacy color types
    for piece in &mut project.pieces {
        migrate_color_types(piece);
    }
    Some(project)
}

fn migrate_color_types(piece: &mut crate::types::Piece) {
    use crate::types::Piece;
    match piece {
        Piece::Cross { color_type, .. }
        | Piece::Dot { color_type, .. }
        | Piece::Line { color_type, .. }
        | Piece::Rectangle { color_type, .. }
        | Piece::HappyFace { color_type, .. } => {
            color_type.migrate();
        }
        Piece::RectPattern { obj, .. } | Piece::CircPattern { obj, .. } => {
            migrate_color_types(obj);
        }
    }
}

pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == ' ')
        .collect::<String>()
        .trim()
        .to_string()
}
