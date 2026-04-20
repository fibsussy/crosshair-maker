use crate::types::Piece;
use super::{split_dim, write_rect};

pub fn draw_line(svg: &mut String, cx: i32, cy: i32, piece: &Piece) {
    let Piece::Line {
        origin,
        vector,
        thickness,
        color,
        odd_anchor,
        ..
    } = piece
    else {
        return;
    };

    let (ox, oy) = *origin;
    let (vx, vy) = *vector;
    let (ax, ay) = odd_anchor.offset();
    let t = *thickness;

    let x1 = cx + ox;
    let y1 = cy - oy;
    let x2 = cx + ox + vx;
    let y2 = cy - oy - vy;

    let dx = (x2 - x1).abs();
    let dy = (y2 - y1).abs();

    if dx >= dy {
        let min_x = x1.min(x2);
        let max_x = x1.max(x2);
        let y_center = (y1 + y2) as f64 / 2.0;
        let (t_neg, _t_pos) = split_dim(t, ay);
        let w = (max_x - min_x + 1) as f64;
        write_rect(svg, min_x as f64, y_center - t_neg, w, t as f64, color);
    } else {
        let min_y = y1.min(y2);
        let max_y = y1.max(y2);
        let x_center = (x1 + x2) as f64 / 2.0;
        let (t_neg, _t_pos) = split_dim(t, ax);
        let h = (max_y - min_y + 1) as f64;
        write_rect(svg, x_center - t_neg, min_y as f64, t as f64, h, color);
    }
}
