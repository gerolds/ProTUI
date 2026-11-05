// ADR: Contract tests drive the public API end to end; the font-dependent glyph sheet is an ignored test because no octant font ships with the repository.
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use terminal_graphics::capture::CellFont;
use terminal_graphics::{Encoder, Ink, Rgb, calibration, encode_image};

#[test]
fn calibration_encodes_into_every_cell_of_its_area() {
    let ground = Ink::opaque(Rgb::BLACK).unwrap();
    let ink = Ink::opaque(Rgb::WHITE).unwrap();
    let image = calibration(30, 10, 2.0, ground, ink).unwrap();
    for encoder in [Encoder::Octant, Encoder::Quadrant] {
        let area = Rect::new(2, 1, 30, 10);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 34, 12));
        encode_image(image.raster(), encoder, area, &mut buffer);
        let glyphs = (area.top()..area.bottom())
            .flat_map(|y| (area.left()..area.right()).map(move |x| (x, y)))
            .filter(|&p| buffer[p].symbol() != " ")
            .count();
        assert!(glyphs > 20, "{encoder:?}: {glyphs}");
        assert_eq!(buffer[(0, 0)], ratatui::buffer::Cell::default());
    }
}

/// Writes every U+2500–U+259F glyph, styled cells and an octant row to
/// `target/glyphs.png`. Run with `TERMINAL_GRAPHICS_FONT=<octant font>
/// cargo test -p terminal-graphics --test capture -- --ignored`.
#[test]
#[ignore = "needs TERMINAL_GRAPHICS_FONT pointing at a font with octants"]
fn export_glyph_sheet() {
    let path = std::env::var_os("TERMINAL_GRAPHICS_FONT").expect("TERMINAL_GRAPHICS_FONT");
    let mut font = CellFont::load(path.as_ref()).unwrap();
    let mut buffer = Buffer::empty(Rect::new(0, 0, 33, 11));
    for (i, code) in (0x2500u32..=0x259F).enumerate() {
        let (x, y) = ((i % 32) as u16, (i / 32) as u16 * 2);
        buffer[(x, y)].set_char(char::from_u32(code).unwrap());
    }
    let styled = Modifier::BOLD | Modifier::ITALIC | Modifier::UNDERLINED;
    buffer.set_string(0, 10, "bold ", Style::default().add_modifier(styled));
    buffer.set_string(
        5,
        10,
        "rev",
        Style::default().add_modifier(Modifier::REVERSED),
    );
    let octants = ratatui::symbols::pixel::OCTANTS;
    for x in 0..24u16 {
        buffer[(9 + x, 10)].set_char(octants[usize::from(x) * 10 + 7]);
    }
    let picture = font.rasterize(&buffer).unwrap();
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/glyphs.png");
    picture.save_png(&out).unwrap();
    println!("{}: {:?}", out.display(), picture.size());
}
