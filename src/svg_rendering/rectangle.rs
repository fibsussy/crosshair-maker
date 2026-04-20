use std::fmt::Write;

use crate::types::Piece;
use super::{apply_color, split_dim, write_rect};

pub fn draw_rectangle(svg: &mut String, cx: i32, cy: i32, piece: &Piece) {
    let Piece::Rectangle {
        origin,
        width,
        height,
        rotation,
        color,
        odd_anchor,
        ..
    } = piece
    else {
        return;
    };

    let (ox, oy) = *origin;
    let (ax, ay) = odd_anchor.offset();
    let w = width.cast_signed();
    let h = height.cast_signed();
    let rx = cx as f64 + ox as f64;
    let ry = cy as f64 - oy as f64;

    let (w_neg, _w_pos) = split_dim(w, ax);
    let (h_neg, _h_pos) = split_dim(h, ay);

    if *rotation == 0.0 {
        write_rect(svg, rx - w_neg, ry - h_neg, w as f64, h as f64, color);
    } else {
        let rot = rotation.to_radians();
        let (cos_r, sin_r) = (rot.cos(), rot.sin());
        let wf = w as f64;
        let hf = h as f64;
        let corners: [(f64, f64); 4] = [
            (-w_neg, -h_neg),
            (wf - w_neg, -h_neg),
            (wf - w_neg, hf - h_neg),
            (-w_neg, hf - h_neg),
        ];
        let mut path = String::from("M");
        for (i, (px, py)) in corners.iter().enumerate() {
            let rx2 = px * cos_r - py * sin_r;
            let ry2 = px * sin_r + py * cos_r;
            let sx = rx + rx2;
            let sy = ry + ry2;
            if i > 0 {
                path.push('L');
            }
            path.push_str(&format!("{sx:.1},{sy:.1}"));
        }
        path.push('Z');
        let fill = apply_color(color);
        write!(svg, r#"<path d="{path}" {fill}/>"#).unwrap();
    }
}
