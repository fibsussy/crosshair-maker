use crate::types::Piece;
use super::{split_dim, write_rect};

pub fn draw_dot(svg: &mut String, cx: i32, cy: i32, piece: &Piece) {
    let Piece::Dot {
        origin,
        size,
        color,
        odd_anchor,
        ..
    } = piece
    else {
        return;
    };

    let s = size.cast_signed();
    let (ox, oy) = *origin;
    let (ax, ay) = odd_anchor.offset();
    let (neg_x, _pos_x) = split_dim(s, ax);
    let (neg_y, _pos_y) = split_dim(s, ay);

    let svg_x = cx as f64 + ox as f64 - neg_x;
    let svg_y = cy as f64 - oy as f64 - neg_y;
    write_rect(svg, svg_x, svg_y, s as f64, s as f64, color);
}
