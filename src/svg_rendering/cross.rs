use crate::types::Piece;
use super::{split_dim, write_rect};

pub fn draw_cross(svg: &mut String, cx: i32, cy: i32, piece: &Piece) {
    let Piece::Cross {
        origin,
        h_gap,
        v_gap,
        length,
        thickness,
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
    let len = *length as f64;

    // ── gap splits ──────────────────────────────────────────────
    let (gap_top, gap_bot) = split_dim(*v_gap, ay);
    let (gap_left, gap_right) = split_dim(*h_gap, ax);

    // ── reach: symmetric outer boundary ─────────────────────────
    let v_reach = (*v_gap as f64 / 2.0).ceil() + len;
    let h_reach = (*h_gap as f64 / 2.0).ceil() + len;

    let arm_len_top = v_reach - gap_top;
    let arm_len_bot = v_reach - gap_bot;
    let arm_len_left = h_reach - gap_left;
    let arm_len_right = h_reach - gap_right;

    // ── thickness splits ────────────────────────────────────────
    let (thick_neg_x, _thick_pos_x) = split_dim(*thickness, ax);
    let (thick_neg_y, _thick_pos_y) = split_dim(*thickness, ay);
    let thick_w = *thickness as f64;
    let thick_h = *thickness as f64;

    // ── vertical arms (top & bottom) ────────────────────────────
    let vert_x = cross_cx - thick_neg_x;
    write_rect(svg, vert_x, cross_cy - gap_top - arm_len_top, thick_w, arm_len_top, color);
    write_rect(svg, vert_x, cross_cy + gap_bot, thick_w, arm_len_bot, color);

    // ── horizontal arms (left & right) ──────────────────────────
    let horiz_y = cross_cy - thick_neg_y;
    write_rect(svg, cross_cx - gap_left - arm_len_left, horiz_y, arm_len_left, thick_h, color);
    write_rect(svg, cross_cx + gap_right, horiz_y, arm_len_right, thick_h, color);
}
