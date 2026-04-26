fn main() {
    let bg_dir = std::path::Path::new("assets/preview_backgrounds");
    let mut entries = Vec::new();
    
    if let Ok(dir) = std::fs::read_dir(bg_dir) {
        for entry in dir.flatten() {
            if let Some(ext) = entry.path().extension() {
                if ext.to_string_lossy().to_lowercase() == "png" {
                    if let Some(stem) = entry.path().file_stem() {
                        let name = stem.to_string_lossy().to_string();
                        entries.push(name);
                    }
                }
            }
        }
    }
    entries.sort();
    
    println!("cargo:rerun-if-changed=assets/preview_backgrounds/*");
    
    let dest_path = std::path::Path::new("src").join("bg_embedded.rs");
    
    let mut code = String::new();
    code.push_str("pub const BACKGROUND_NAMES: &[&str] = &[\n");
    for name in &entries {
        code.push_str(&format!("    \"{}\",\n", name));
    }
    code.push_str("];\n\n");
    code.push_str("pub fn get_background(name: &str) -> Option<&'static [u8]> {\n");
    code.push_str("    match name {\n");
    for name in &entries {
        code.push_str(&format!(
            "        \"{}\" => Some(include_bytes!(\"../assets/preview_backgrounds/{}.png\")),\n",
            name, name
        ));
    }
    code.push_str("        _ => None,\n");
    code.push_str("    }\n");
    code.push_str("}\n");
    
    std::fs::write(&dest_path, code).unwrap();
}