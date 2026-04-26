use crate::types::Piece;
use super::{split_dim, write_rect};

pub fn draw_cross(svg: &mut String, cx: i32, cy: i32, piece: &Piece) {
    let Piece::Cross {
        origin,
        left_gap, right_gap, top_gap, bottom_gap,
        left_thickness, right_thickness, top_thickness, bottom_thickness,
        left_length, right_length, top_length, bottom_length,
        color,
        odd_anchor,
        ..
    } = piece
    else {
        return;
    };

    let (ox, oy) = *origin;
    let (ax, ay) = odd_anchor.offset();
    let cross_cx = cx as f64 + ox as f64;
    let cross_cy = cy as f64 - oy as f64;

    let (_, top_gap_neg) = split_dim(*top_gap, ay);
    let (bottom_gap_pos, _) = split_dim(*bottom_gap, ay);
    let (_, left_gap_neg) = split_dim(*left_gap, ax);
    let (right_gap_pos, _) = split_dim(*right_gap, ax);

    let (_, top_thick_neg) = split_dim(*top_thickness, ay);
    let (bottom_thick_pos, _) = split_dim(*bottom_thickness, ay);
    let (_, left_thick_neg) = split_dim(*left_thickness, ax);
    let (right_thick_pos, _) = split_dim(*right_thickness, ax);

    let top_x = cross_cx - top_thick_neg;
    let bot_x = cross_cx - bottom_thick_pos;
    let left_y = cross_cy - left_thick_neg;
    let right_y = cross_cy - right_thick_pos;

    write_rect(svg, top_x, cross_cy - top_gap_neg - *top_length as f64, *top_thickness as f64, *top_length as f64, color);
    write_rect(svg, bot_x, cross_cy + bottom_gap_pos, *bottom_thickness as f64, *bottom_length as f64, color);

    write_rect(svg, cross_cx - left_gap_neg - *left_length as f64, left_y, *left_length as f64, *left_thickness as f64, color);
    write_rect(svg, cross_cx + right_gap_pos, right_y, *right_length as f64, *right_thickness as f64, color);
}