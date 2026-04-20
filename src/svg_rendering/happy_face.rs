use crate::types::Piece;
use super::{split_dim, write_rect};

pub fn draw_happy_face(svg: &mut String, cx: i32, cy: i32, piece: &Piece) {
    let Piece::HappyFace {
        origin,
        size,
        color,
        odd_anchor,
        ..
    } = piece
    else {
        return;
    };

    let (ox, oy) = *origin;
    let (ax, ay) = odd_anchor.offset();
    let s = f64::from(size.cast_signed());
    let scale = s / 3.0;
    let dot = size.cast_signed();
    let (d_neg_x, _d_pos_x) = split_dim(dot, ax);
    let (d_neg_y, _d_pos_y) = split_dim(dot, ay);

    let offsets: [(i32, i32); 7] = [
        (-3, 4), (3, 4), (-6, -1), (-3, -4), (0, -4), (3, -4), (6, -1),
    ];

    for (dx, dy) in offsets {
        let px = cx as f64 + ox as f64 + (f64::from(dx) * scale).round();
        let py = cy as f64 - oy as f64 - (f64::from(dy) * scale).round();
        write_rect(
            svg,
            px - d_neg_x,
            py - d_neg_y,
            dot as f64,
            dot as f64,
            color,
        );
    }
}
