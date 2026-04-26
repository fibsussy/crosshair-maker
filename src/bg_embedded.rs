pub const BACKGROUND_NAMES: &[&str] = &[
    "CSGO",
    "Kovaaks Target",
    "The Finals",
];

pub fn get_background(name: &str) -> Option<&'static [u8]> {
    match name {
        "CSGO" => Some(include_bytes!("../assets/preview_backgrounds/CSGO.png")),
        "Kovaaks Target" => Some(include_bytes!("../assets/preview_backgrounds/Kovaaks Target.png")),
        "The Finals" => Some(include_bytes!("../assets/preview_backgrounds/The Finals.png")),
        _ => None,
    }
}
