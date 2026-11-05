// ADR: Contract test on the shipped sample place: it loads strictly, lights every lamp tile, encodes through both encoders and its raw view reproduces the authored map.
use std::path::Path;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use terminal_graphics::Encoder;
use terminal_tilemap::{Camera, Frame, TileRect, World, draw_raw, load_place};

fn sample() -> World {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/sample.toml");
    World::new(load_place(&path).expect("the sample place loads"))
}

#[test]
fn the_sample_lights_every_lamp_and_encodes_lit_cells() {
    let world = sample();
    let place = world.place();
    let lamp_tiles = (0..place.height() as i32)
        .flat_map(|y| (0..place.width() as i32).map(move |x| (x, y)))
        .filter(|&(x, y)| place.tile(x, y).is_some_and(|(_, d)| d.light.is_some()))
        .count();
    assert_eq!(world.lamps().len(), lamp_tiles);
    assert!(lamp_tiles > 0);
    let mut frame = Frame::default();
    for (encoder, size) in [(Encoder::Octant, (80, 24)), (Encoder::Quadrant, (160, 50))] {
        let area = Rect::new(0, 0, size.0, size.1);
        let rect = TileRect::whole(place.width(), place.height());
        let camera = Camera::overview(area, rect, (0.0, 0.0), 8.0, 2.0).unwrap();
        let mut buf = Buffer::empty(area);
        world
            .render(&mut frame, 0.5, &camera, encoder, area, &mut buf)
            .unwrap();
        let lit = buf
            .content()
            .iter()
            .filter(|c| matches!(c.bg, Color::Rgb(r, g, b) if u16::from(r) + u16::from(g) + u16::from(b) > 150))
            .count();
        assert!(lit > 0, "{encoder:?}: some cells show lit ground");
        for lamp in world.lamps() {
            let light = world.light_at(&frame, (lamp.x, lamp.y)).unwrap();
            assert!(light.luminance() > place.ambient().luminance() * 4.0);
        }
    }
}

#[test]
fn the_raw_view_reproduces_the_authored_map() {
    let world = sample();
    let place = world.place();
    let (w, h) = (place.width() as u16, place.height() as u16);
    let area = Rect::new(0, 0, w, h);
    let mut buf = Buffer::empty(area);
    let camera = Camera::raw(area, (place.width(), place.height()), (0.0, 0.0));
    draw_raw(place, &camera, area, &mut buf);
    let source =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/sample.toml"))
            .unwrap();
    let map: Vec<&str> = source
        .split("'''")
        .nth(1)
        .unwrap()
        .trim_start_matches(['\r', '\n'])
        .lines()
        .collect();
    for (y, row) in map.iter().enumerate() {
        let drawn: String = (0..w).map(|x| buf[(x, y as u16)].symbol()).collect();
        assert_eq!(&drawn, row, "row {y}");
    }
}
