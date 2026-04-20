use eframe::egui;

use crate::types::{OddAnchor, Piece};

fn slider_i32(ui: &mut egui::Ui, val: &mut i32, range: std::ops::RangeInclusive<i32>, label: &str) -> bool {
    ui.add(egui::Slider::new(val, range).text(label).step_by(1.0)).changed()
}

fn slider_u32(ui: &mut egui::Ui, val: &mut u32, range: std::ops::RangeInclusive<u32>, label: &str) -> bool {
    ui.add(egui::Slider::new(val, range).text(label).step_by(1.0)).changed()
}

fn slider_f64(ui: &mut egui::Ui, val: &mut f64, range: std::ops::RangeInclusive<f64>, label: &str) -> bool {
    ui.add(egui::Slider::new(val, range).text(label).step_by(1.0)).changed()
}

fn edit_origin(ui: &mut egui::Ui, origin: &mut (i32, i32)) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Origin");
        changed |= ui.add(egui::DragValue::new(&mut origin.0).speed(1.0)).changed();
        ui.label("Y");
        changed |= ui.add(egui::DragValue::new(&mut origin.1).speed(1.0)).changed();
    });
    changed
}

fn edit_odd_anchor(ui: &mut egui::Ui, anchor: &mut OddAnchor) -> bool {
    let mut idx = match *anchor {
        OddAnchor::TopRight => 0,
        OddAnchor::TopLeft => 1,
        OddAnchor::BottomLeft => 2,
        OddAnchor::BottomRight => 3,
        OddAnchor::Center => 4,
    };
    let prev = idx;
    egui::ComboBox::from_label("Odd Anchor")
        .selected_text(format!("{anchor}"))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut idx, 0, "TopRight");
            ui.selectable_value(&mut idx, 1, "TopLeft");
            ui.selectable_value(&mut idx, 2, "BottomLeft");
            ui.selectable_value(&mut idx, 3, "BottomRight");
            ui.selectable_value(&mut idx, 4, "Center");
        });
    if idx != prev {
        *anchor = match idx {
            0 => OddAnchor::TopRight,
            1 => OddAnchor::TopLeft,
            2 => OddAnchor::BottomLeft,
            3 => OddAnchor::BottomRight,
            _ => OddAnchor::Center,
        };
        true
    } else {
        false
    }
}

pub fn edit_piece(ui: &mut egui::Ui, piece: &mut Piece) -> bool {
    let mut changed = false;
    match piece {
        Piece::Cross {
            origin,
            h_gap,
            v_gap,
            length,
            thickness,
            color,
            odd_anchor,
            lock_gap,
            ..
        } => {
            ui.label("Cross");
            changed |= edit_origin(ui, origin);
            if ui.checkbox(lock_gap, "Lock Axis").changed() {
                if *lock_gap {
                    *v_gap = *h_gap;
                }
                changed = true;
            }
            if *lock_gap {
                if slider_i32(ui, h_gap, -100..=100, "Gap") {
                    *v_gap = *h_gap;
                    changed = true;
                }
            } else {
                changed |= slider_i32(ui, h_gap, -100..=100, "H Gap");
                changed |= slider_i32(ui, v_gap, -100..=100, "V Gap");
            }
            changed |= slider_i32(ui, length, -200..=200, "Length");
            changed |= slider_i32(ui, thickness, 1..=50, "Thickness");
            changed |= edit_color(ui, color);
            changed |= edit_odd_anchor(ui, odd_anchor);
        }
        Piece::Dot {
            origin,
            size,
            color,
            odd_anchor,
            ..
        } => {
            ui.label("Dot");
            changed |= edit_origin(ui, origin);
            changed |= slider_u32(ui, size, 1..=100, "Size");
            changed |= edit_color(ui, color);
            changed |= edit_odd_anchor(ui, odd_anchor);
        }
        Piece::Line {
            origin,
            vector,
            thickness,
            color,
            odd_anchor,
            ..
        } => {
            ui.label("Line");
            changed |= edit_origin(ui, origin);
            ui.horizontal(|ui| {
                ui.label("Vector");
                changed |= ui.add(egui::DragValue::new(&mut vector.0).speed(1.0)).changed();
                ui.label("Y");
                changed |= ui.add(egui::DragValue::new(&mut vector.1).speed(1.0)).changed();
            });
            changed |= slider_i32(ui, thickness, 1..=50, "Thickness");
            changed |= edit_color(ui, color);
            changed |= edit_odd_anchor(ui, odd_anchor);
        }
        Piece::Rectangle {
            origin,
            width,
            height,
            rotation,
            color,
            odd_anchor,
            ..
        } => {
            ui.label("Rectangle");
            changed |= edit_origin(ui, origin);
            changed |= slider_u32(ui, width, 1..=500, "Width");
            changed |= slider_u32(ui, height, 1..=500, "Height");
            changed |= slider_f64(ui, rotation, -360.0..=360.0, "Rotation");
            changed |= edit_color(ui, color);
            changed |= edit_odd_anchor(ui, odd_anchor);
        }
        Piece::RectPattern {
            origin,
            x_distance,
            x_quantity,
            y_distance,
            y_quantity,
            obj,
            ..
        } => {
            ui.label("RectPattern");
            changed |= edit_origin(ui, origin);
            changed |= slider_i32(ui, x_distance, -200..=200, "X Distance");
            changed |= slider_u32(ui, x_quantity, 1..=50, "X Quantity");
            changed |= slider_i32(ui, y_distance, -200..=200, "Y Distance");
            changed |= slider_u32(ui, y_quantity, 1..=50, "Y Quantity");
            ui.separator();
            ui.label("Inner piece:");
            changed |= edit_piece_type_selector(ui, obj);
            changed |= edit_piece(ui, obj);
        }
        Piece::CircPattern {
            origin,
            radius,
            quantity,
            start_deg,
            obj,
            ..
        } => {
            ui.label("CircPattern");
            changed |= edit_origin(ui, origin);
            changed |= slider_i32(ui, radius, 1..=500, "Radius");
            changed |= slider_u32(ui, quantity, 1..=50, "Quantity");
            changed |= slider_f64(ui, start_deg, -360.0..=360.0, "Start Degrees");
            ui.separator();
            ui.label("Inner piece:");
            changed |= edit_piece_type_selector(ui, obj);
            changed |= edit_piece(ui, obj);
        }
        Piece::HappyFace {
            origin,
            size,
            color,
            odd_anchor,
            ..
        } => {
            ui.label("HappyFace");
            changed |= edit_origin(ui, origin);
            changed |= slider_u32(ui, size, 1..=100, "Size");
            changed |= edit_color(ui, color);
            changed |= edit_odd_anchor(ui, odd_anchor);
        }
    }
    changed
}

fn edit_piece_type_selector(ui: &mut egui::Ui, obj: &mut Box<Piece>) -> bool {
    let mut changed = false;
    let current = obj.type_name();
    let types = ["Dot", "Line", "Rectangle", "Cross", "HappyFace"];
    egui::ComboBox::from_label("Inner type")
        .selected_text(current)
        .show_ui(ui, |ui| {
            for t in &types {
                if ui.selectable_label(current == *t, *t).clicked() && current != *t {
                    *obj = Box::new(default_piece_of_type(t));
                    changed = true;
                }
            }
        });
    changed
}

fn default_piece_of_type(name: &str) -> Piece {
    match name {
        "Dot" => Piece::Dot {
            origin: (0, 0),
            size: 2,
            color: "#ff5050ff".to_string(),
            visible: true,
            odd_anchor: OddAnchor::default(),
        },
        "Line" => Piece::Line {
            origin: (0, 0),
            vector: (10, 0),
            thickness: 2,
            color: "#ffffffff".to_string(),
            visible: true,
            odd_anchor: OddAnchor::default(),
        },
        "Rectangle" => Piece::Rectangle {
            origin: (0, 0),
            width: 10,
            height: 10,
            rotation: 0.0,
            color: "#ffffffff".to_string(),
            visible: true,
            odd_anchor: OddAnchor::default(),
        },
        "Cross" => Piece::Cross {
            origin: (0, 0),
            h_gap: 2,
            v_gap: 2,
            length: 4,
            thickness: 2,
            color: "#00ff7dff".to_string(),
            visible: true,
            odd_anchor: OddAnchor::default(),
            lock_gap: true,
        },
        "HappyFace" => Piece::HappyFace {
            origin: (0, 0),
            size: 3,
            color: "#00ff7dff".to_string(),
            visible: true,
            odd_anchor: OddAnchor::default(),
        },
        _ => Piece::Dot {
            origin: (0, 0),
            size: 2,
            color: "#ffffffff".to_string(),
            visible: true,
            odd_anchor: OddAnchor::default(),
        },
    }
}

pub fn edit_color(ui: &mut egui::Ui, color: &mut String) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Color");
        changed |= ui.text_edit_singleline(color).changed();

        let mut c = parse_color(color);
        if ui.color_edit_button_srgba(&mut c).changed() {
            let [r, g, b, a] = c.to_srgba_unmultiplied();
            *color = format!("#{r:02x}{g:02x}{b:02x}{a:02x}");
            changed = true;
        }
    });
    changed
}

pub fn parse_color(color: &str) -> egui::Color32 {
    if let Some(hex) = color.strip_prefix('#') {
        if hex.len() == 8 {
            if let Ok(r) = u8::from_str_radix(&hex[0..2], 16) {
                if let Ok(g) = u8::from_str_radix(&hex[2..4], 16) {
                    if let Ok(b) = u8::from_str_radix(&hex[4..6], 16) {
                        if let Ok(a) = u8::from_str_radix(&hex[6..8], 16) {
                            return egui::Color32::from_rgba_unmultiplied(r, g, b, a);
                        }
                    }
                }
            }
        } else if hex.len() == 6 {
            if let Ok(r) = u8::from_str_radix(&hex[0..2], 16) {
                if let Ok(g) = u8::from_str_radix(&hex[2..4], 16) {
                    if let Ok(b) = u8::from_str_radix(&hex[4..6], 16) {
                        return egui::Color32::from_rgb(r, g, b);
                    }
                }
            }
        }
    }
    egui::Color32::WHITE
}
