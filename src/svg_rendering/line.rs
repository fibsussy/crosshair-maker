use std::fmt::Write;

use crate::types::Piece;
use super::{apply_color, split_dim};

pub fn draw_line(svg: &mut String, cx: i32, cy: i32, piece: &Piece) {
    let Piece::Line {
        origin,
        vector,
        thickness,
        color,
        odd_anchor,
        anti_aliasing,
        ..
    } = piece
    else {
        return;
    };

    let (ox, oy) = *origin;
    let (vx, vy) = *vector;
    let (_ax, ay) = odd_anchor.offset();
    let t = *thickness;

    let x1 = cx as f64 + ox as f64;
    let y1 = cy as f64 - oy as f64;
    let x2 = cx as f64 + ox as f64 + vx as f64;
    let y2 = cy as f64 - oy as f64 - vy as f64;

    let dx = x2 - x1;
    let dy = y2 - y1;
    let line_len = (dx * dx + dy * dy).sqrt();

    if line_len < 0.001 {
        return;
    }

    let (t_neg, t_pos) = split_dim(t, ay);

    // Perpendicular unit vector (points "left" of the line direction)
    let ux = -dy / line_len;
    let uy = dx / line_len;

    // Extend t_neg in the negative perpendicular direction, t_pos in the positive
    let corners: [(f64, f64); 4] = [
        (x1 - ux * t_neg, y1 - uy * t_neg),
        (x1 + ux * t_pos, y1 + uy * t_pos),
        (x2 + ux * t_pos, y2 + uy * t_pos),
        (x2 - ux * t_neg, y2 - uy * t_neg),
    ];

    let mut path = String::from("M");
    for (i, (px, py)) in corners.iter().enumerate() {
        if i > 0 {
            path.push('L');
        }
        path.push_str(&format!("{px:.1},{py:.1}"));
    }
    path.push('Z');

    let fill = apply_color(color);
    if *anti_aliasing {
        write!(svg, r#"<path d="{path}" {fill} shape-rendering="geometricPrecision"/>"#).unwrap();
    } else {
        write!(svg, r#"<path d="{path}" {fill} shape-rendering="crispEdges"/>"#).unwrap();
    }
}