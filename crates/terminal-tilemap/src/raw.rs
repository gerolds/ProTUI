// ADR: The raw view is the ASCII source view: one cell per tile with the legend char in its surface colour on a black ground; no light, no raster. Pins draw over it with `draw_pins`.
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

use crate::camera::Camera;
use crate::place::{Place, TILE_PX};

/// Ground colour of every raw cell, and of the lit view outside the place.
pub const OUTSIDE: [u8; 3] = [0, 0, 0];

/// Draws `place` through `camera` (`Camera::raw`) into `area` of `buf`: each cell shows the char of
/// the tile under its centre, blank outside the place. Cells outside `buf` are skipped.
pub fn draw_raw(place: &Place, camera: &Camera, area: Rect, buf: &mut Buffer) {
    let tile = TILE_PX as f32;
    let [r, g, b] = OUTSIDE;
    let ground = Color::Rgb(r, g, b);
    let visible = area.intersection(buf.area);
    for y in visible.top()..visible.bottom() {
        for x in visible.left()..visible.right() {
            let (cx, cy) = camera.cell_centre(x - area.x, y - area.y);
            let cell = &mut buf[(x, y)];
            cell.reset();
            cell.set_bg(ground);
            if let Some((glyph, def)) =
                place.tile((cx / tile).floor() as i32, (cy / tile).floor() as i32)
            {
                let [r, g, b] = def.surface.color;
                cell.set_char(glyph).set_fg(Color::Rgb(r, g, b));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::place::tests::legend;

    #[test]
    fn the_raw_view_shows_the_source_chars_in_legend_colours() {
        let place = Place::new("###\n#.#\n###", legend(), [0; 3], 1).unwrap();
        let area = Rect::new(0, 0, 5, 3);
        let mut buf = Buffer::empty(area);
        draw_raw(
            &place,
            &Camera::raw(area, (3, 3), (1.5, 1.5)),
            area,
            &mut buf,
        );
        let row = |y: u16| (0..5).map(|x| buf[(x, y)].symbol()).collect::<String>();
        assert_eq!([row(0), row(1), row(2)], [" ### ", " #.# ", " ### "]);
        assert_eq!(buf[(2, 1)].fg, Color::Rgb(160, 160, 160));
        assert_eq!(buf[(0, 1)].bg, Color::Rgb(0, 0, 0));
    }
}
