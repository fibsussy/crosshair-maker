use crate::types::Piece;

pub fn offset_piece(piece: &Piece, dx: i32, dy: i32) -> Piece {
    match piece {
        Piece::Cross {
            origin,
            h_gap,
            v_gap,
            length,
            thickness,
            color,
            color_type,
            visible,
            odd_anchor,
            lock_gap,
        } => {
            let (ox, oy) = *origin;
            Piece::Cross {
                origin: (ox + dx, oy + dy),
                h_gap: *h_gap,
                v_gap: *v_gap,
                length: *length,
                thickness: *thickness,
                color: color.clone(),
                color_type: color_type.clone(),
                visible: *visible,
                odd_anchor: *odd_anchor,
                lock_gap: *lock_gap,
            }
        }
        Piece::Dot {
            origin,
            size,
            color,
            color_type,
            visible,
            odd_anchor,
        } => {
            let (ox, oy) = *origin;
            Piece::Dot {
                origin: (ox + dx, oy + dy),
                size: *size,
                color: color.clone(),
                color_type: color_type.clone(),
                visible: *visible,
                odd_anchor: *odd_anchor,
            }
        }
        Piece::Line {
            origin,
            vector,
            thickness,
            color,
            color_type,
            visible,
            odd_anchor,
        } => {
            let (ox, oy) = *origin;
            Piece::Line {
                origin: (ox + dx, oy + dy),
                vector: *vector,
                thickness: *thickness,
                color: color.clone(),
                color_type: color_type.clone(),
                visible: *visible,
                odd_anchor: *odd_anchor,
            }
        }
        Piece::Rectangle {
            origin,
            width,
            height,
            rotation,
            color,
            color_type,
            visible,
            odd_anchor,
        } => {
            let (ox, oy) = *origin;
            Piece::Rectangle {
                origin: (ox + dx, oy + dy),
                width: *width,
                height: *height,
                rotation: *rotation,
                color: color.clone(),
                color_type: color_type.clone(),
                visible: *visible,
                odd_anchor: *odd_anchor,
            }
        }
        Piece::RectPattern {
            origin,
            x_distance,
            x_quantity,
            y_distance,
            y_quantity,
            obj,
            visible,
        } => {
            let (ox, oy) = *origin;
            Piece::RectPattern {
                origin: (ox + dx, oy + dy),
                x_distance: *x_distance,
                x_quantity: *x_quantity,
                y_distance: *y_distance,
                y_quantity: *y_quantity,
                obj: Box::new(offset_piece(obj, dx, dy)),
                visible: *visible,
            }
        }
        Piece::CircPattern {
            origin,
            radius,
            quantity,
            start_deg,
            obj,
            visible,
        } => {
            let (ox, oy) = *origin;
            Piece::CircPattern {
                origin: (ox + dx, oy + dy),
                radius: *radius,
                quantity: *quantity,
                start_deg: *start_deg,
                obj: Box::new(offset_piece(obj, dx, dy)),
                visible: *visible,
            }
        }
        Piece::HappyFace {
            origin,
            size,
            color,
            color_type,
            visible,
            odd_anchor,
        } => {
            let (ox, oy) = *origin;
            Piece::HappyFace {
                origin: (ox + dx, oy + dy),
                size: *size,
                color: color.clone(),
                color_type: color_type.clone(),
                visible: *visible,
                odd_anchor: *odd_anchor,
            }
        }
    }
}

fn collect_pattern_pieces(piece: &Piece, pieces: &mut Vec<Piece>) {
    if !piece.is_visible() {
        return;
    }
    match piece {
        Piece::RectPattern {
            origin,
            x_distance,
            x_quantity,
            y_distance,
            y_quantity,
            obj,
            ..
        } => {
            let (ox, oy) = *origin;
            let xq: u32 = *x_quantity;
            let yq: u32 = *y_quantity;
            for xi in 0..xq {
                for yi in 0..yq {
                    let dx = ox + (xi as i32) * *x_distance;
                    let dy = oy + (yi as i32) * *y_distance;
                    let offset_obj = offset_piece(obj, dx, dy);
                    collect_pattern_pieces(&offset_obj, pieces);
                }
            }
        }
        Piece::CircPattern {
            origin,
            radius,
            quantity,
            start_deg,
            obj,
            ..
        } => {
            if *quantity == 0 {
                return;
            }
            let (ox, oy) = *origin;
            let angle_step = 360.0 / f64::from(*quantity);
            for i in 0..*quantity {
                let angle_deg = f64::from(i).mul_add(angle_step, *start_deg);
                let angle_rad = angle_deg.to_radians();
                #[allow(clippy::cast_possible_truncation)]
                let dx = ox + (f64::from(*radius) * angle_rad.cos()).round() as i32;
                #[allow(clippy::cast_possible_truncation)]
                let dy = oy + (f64::from(*radius) * angle_rad.sin()).round() as i32;
                let offset_obj = offset_piece(obj, dx, dy);
                collect_pattern_pieces(&offset_obj, pieces);
            }
        }
        _ => {
            pieces.push(piece.clone());
        }
    }
}

pub fn expand_pieces(pieces: &[Piece]) -> Vec<Piece> {
    let mut expanded = Vec::new();
    for piece in pieces {
        collect_pattern_pieces(piece, &mut expanded);
    }
    expanded
}
