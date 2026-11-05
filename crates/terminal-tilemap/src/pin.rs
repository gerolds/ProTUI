// ADR: A pin is a crisp char drawn over the encoded cell holding its point; its backing is the cell's encoded colours averaged and darkened, so the glyph reads on any lit ground.
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

use crate::camera::Camera;
use crate::place::TILE_PX;

/// Share of the cell's colour a pin's backing keeps.
const BACKING: f32 = 0.6;

/// A char pinned to a point of a place, for markers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pin {
    /// Tile units; tile centres sit at .5.
    pub at: (f32, f32),
    pub glyph: char,
    /// sRGB colour of the glyph.
    pub color: [u8; 3],
}

/// Draws `pins` over `area` of `buf` through `camera`, each on the cell holding its point; pins
/// outside the view draw nothing, and a later pin wins a shared cell.
pub fn draw_pins(pins: &[Pin], camera: &Camera, area: Rect, buf: &mut Buffer) {
    let tile = TILE_PX as f32;
    for pin in pins {
        let Some((col, row)) = camera.cell_of(area, pin.at.0 * tile, pin.at.1 * tile) else {
            continue;
        };
        let Some(cell) = buf.cell_mut((area.x + col, area.y + row)) else {
            continue;
        };
        let backing = match (cell.fg, cell.bg) {
            (Color::Rgb(a, b, c), Color::Rgb(d, e, f)) => {
                let mix = |x: u8, y: u8| ((f32::from(x) + f32::from(y)) * 0.5 * BACKING) as u8;
                Color::Rgb(mix(a, d), mix(b, e), mix(c, f))
            }
            (_, Color::Rgb(r, g, b)) => {
                let dim = |v: u8| (f32::from(v) * BACKING) as u8;
                Color::Rgb(dim(r), dim(g), dim(b))
            }
            (_, bg) => bg,
        };
        let [r, g, b] = pin.color;
        cell.set_char(pin.glyph)
            .set_fg(Color::Rgb(r, g, b))
            .set_bg(backing);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pin_lands_on_its_tiles_cell_and_outside_pins_draw_nothing() {
        let area = Rect::new(2, 1, 10, 5);
        let mut buf = Buffer::empty(Rect::new(0, 0, 12, 6));
        buf.set_style(
            area,
            ratatui::style::Style::new().bg(Color::Rgb(100, 50, 200)),
        );
        let camera = Camera::raw(area, (10, 5), (5.0, 2.5));
        let pins = [
            Pin {
                at: (3.5, 2.5),
                glyph: '@',
                color: [255, 255, 0],
            },
            Pin {
                at: (40.0, 2.5),
                glyph: 'x',
                color: [255, 0, 0],
            },
        ];
        let before = buf.clone();
        draw_pins(&pins, &camera, area, &mut buf);
        let cell = &buf[(2 + 3, 1 + 2)];
        assert_eq!(cell.symbol(), "@");
        assert_eq!(cell.fg, Color::Rgb(255, 255, 0));
        assert_eq!(cell.bg, Color::Rgb(60, 30, 120));
        let changed = buf
            .content()
            .iter()
            .zip(before.content())
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(changed, 1);
    }
}
