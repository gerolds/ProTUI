// ADR: One example drives the whole pipeline on the neutral sample place: live (pan, encoder, raw view, cell aspect), a headless frame bench, and a font-rendered PNG export through terminal-graphics `capture`.
use std::error::Error;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use terminal_graphics::Encoder;
use terminal_graphics::capture::CellFont;
use terminal_tilemap::{
    Camera, DynamicLight, Frame, Light, Pin, TileRect, World, draw_pins, draw_raw, load_place,
};

type Result<T, E = Box<dyn Error>> = std::result::Result<T, E>;

const HELP: &str = "Lit tile map example on a neutral sample place
Usage: cargo run --release -p terminal-tilemap --example place -- [options]
  (no options)             Live view; stdin and stdout must be terminals
  --frames N --size WxH    Headless bench: N frames per run, prints frame-build ms
  --export OUT.png --font FONT.ttf [--size WxH]
                           Render one lit frame (default 80x24) and save it with an octant font
  --place FILE.toml        Use another place file instead of examples/sample.toml
  --help                   Print help
Live keys: arrows pan, o octant/quadrant, r raw view, + / - cell aspect, q quits.
A light follows the pan focus, marked with a '+' pin.";

/// Tiles the live view frames around the focus.
const WINDOW: (f32, f32) = (24.0, 14.0);
/// Zoom limit in cells per tile.
const LIMIT: f32 = 8.0;

#[derive(Default)]
struct Options {
    frames: Option<usize>,
    size: Option<(u16, u16)>,
    export: Option<PathBuf>,
    font: Option<PathBuf>,
    place: Option<PathBuf>,
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("place: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let options = parse_options()?;
    let path = options
        .place
        .clone()
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/sample.toml"));
    let started = Instant::now();
    let world = World::new(load_place(&path)?);
    let build_ms = ms(started.elapsed());
    match (options.frames, options.export, options.font) {
        (Some(frames), None, None) => {
            let size = options.size.ok_or("--frames needs --size WxH")?;
            bench(&world, frames, size, build_ms)
        }
        (None, Some(out), Some(font)) => {
            export(&world, &out, &font, options.size.unwrap_or((80, 24)))
        }
        (None, None, None) if options.size.is_none() => live(&world),
        _ => Err("use --frames with --size, --export with --font, or no mode; see --help".into()),
    }
}

fn parse_options() -> Result<Options> {
    let mut options = Options::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or(format!("{arg} needs a value"));
        match arg.as_str() {
            "--help" => {
                println!("{HELP}");
                std::process::exit(0);
            }
            "--frames" if options.frames.is_none() => {
                let n: usize = value()?.parse()?;
                if n == 0 {
                    return Err("--frames must be at least 1".into());
                }
                options.frames = Some(n);
            }
            "--size" if options.size.is_none() => options.size = Some(parse_size(&value()?)?),
            "--export" if options.export.is_none() => options.export = Some(value()?.into()),
            "--font" if options.font.is_none() => options.font = Some(value()?.into()),
            "--place" if options.place.is_none() => options.place = Some(value()?.into()),
            _ => return Err(format!("unknown or repeated option {arg:?}; use --help").into()),
        }
    }
    Ok(options)
}

fn parse_size(text: &str) -> Result<(u16, u16)> {
    let (w, h) = text.split_once('x').ok_or("size must be WxH, e.g. 80x24")?;
    let (w, h): (u16, u16) = (w.parse()?, h.parse()?);
    if w == 0 || h == 0 {
        return Err("size must be at least 1x1".into());
    }
    Ok((w, h))
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// A steady neutral light at `at` (tile units), as the host would add one per frame.
fn focus_light(at: (f32, f32)) -> DynamicLight {
    DynamicLight {
        key: 1,
        light: Light {
            x: at.0,
            y: at.1,
            color: terminal_graphics::Rgb::new(0.9, 0.9, 0.85),
            intensity: 1.4,
            radius: 4.0,
            flicker: 0.0,
            glow: 0.8,
            seed: 7,
        },
    }
}

fn whole(world: &World) -> TileRect {
    TileRect::whole(world.place().width(), world.place().height())
}

fn bench(world: &World, frames: usize, (w, h): (u16, u16), build_ms: f64) -> Result<()> {
    let area = Rect::new(0, 0, w, h);
    let rect = whole(world);
    let centre = ((rect.x1 / 2.0), (rect.y1 / 2.0));
    let camera = Camera::overview(area, rect, centre, LIMIT, 2.0)?;
    println!(
        "place {}x{} tiles, world build {build_ms:.1} ms, view {w}x{h} at {:.2} cells per tile, {frames} frames per run",
        world.place().width(),
        world.place().height(),
        camera.cells_per_tile()
    );
    for (encoder, name) in [(Encoder::Octant, "octant"), (Encoder::Quadrant, "quadrant")] {
        for moving in [false, true] {
            let mut frame = Frame::default();
            let mut buf = Buffer::empty(area);
            let (mut total, mut worst, mut recomputed) = (0.0f64, 0.0f64, 0);
            for i in 0..frames {
                let t = i as f32 / 30.0;
                frame.dynamic.clear();
                if moving {
                    // Walk the corridor row, a quarter tile per frame.
                    let x = 1.5 + (i as f32 * 0.25) % (rect.x1 - 3.0);
                    frame.dynamic.push(focus_light((x, 5.5)));
                }
                let started = Instant::now();
                world.render(&mut frame, t, &camera, encoder, area, &mut buf)?;
                let elapsed = ms(started.elapsed());
                total += elapsed;
                worst = worst.max(elapsed);
                recomputed += frame.recomputed();
            }
            let run = if moving { "moving light" } else { "lamps only" };
            println!(
                "{name:8} {run:12}: mean {:.2} ms, worst {:.2} ms, {recomputed} field recomputes",
                total / frames as f64,
                worst
            );
        }
    }
    Ok(())
}

fn export(world: &World, out: &Path, font: &Path, (w, h): (u16, u16)) -> Result<()> {
    let area = Rect::new(0, 0, w, h);
    let rect = whole(world);
    let focus = (rect.x1 / 2.0, rect.y1 / 2.0);
    let camera = Camera::overview(area, rect, focus, LIMIT, 2.0)?;
    let mut frame = Frame::default();
    let mut buf = Buffer::empty(area);
    world.render(&mut frame, 1.0, &camera, Encoder::Octant, area, &mut buf)?;
    let pins = [Pin {
        at: (2.5, 11.5),
        glyph: '@',
        color: [255, 236, 120],
    }];
    draw_pins(&pins, &camera, area, &mut buf);
    let mut font = CellFont::load(font)?;
    let picture = font.rasterize(&buf)?;
    picture.save_png(out)?;
    let (pw, ph) = picture.size();
    println!(
        "wrote {} ({pw}x{ph} px, {} bytes) from a {w}x{h} octant frame at {:.2} cells per tile",
        out.display(),
        std::fs::metadata(out)?.len(),
        camera.cells_per_tile()
    );
    Ok(())
}

struct Live {
    focus: (f32, f32),
    encoder: Encoder,
    raw: bool,
    aspect: f32,
}

fn live(world: &World) -> Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("the live view needs a terminal; use --frames or --export headless".into());
    }
    let mut state = Live {
        focus: (6.5, 3.5),
        encoder: Encoder::Octant,
        raw: false,
        aspect: 2.0,
    };
    let mut terminal = ratatui::init();
    let started = Instant::now();
    let mut frame = Frame::default();
    let result = (|| -> Result<()> {
        loop {
            let t = started.elapsed().as_secs_f32();
            let mut failure = None;
            terminal.draw(|f| {
                let full = f.area();
                if let Err(e) = draw(world, &state, &mut frame, t, full, f.buffer_mut()) {
                    failure = Some(e);
                }
            })?;
            if let Some(e) = failure {
                return Err(e);
            }
            if !event::poll(Duration::from_millis(33))? {
                continue;
            }
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            let (w, h) = (world.place().width() as f32, world.place().height() as f32);
            let step = |v: f32, d: f32, max: f32| (v + d).clamp(0.5, max - 0.5);
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Left => state.focus.0 = step(state.focus.0, -1.0, w),
                KeyCode::Right => state.focus.0 = step(state.focus.0, 1.0, w),
                KeyCode::Up => state.focus.1 = step(state.focus.1, -1.0, h),
                KeyCode::Down => state.focus.1 = step(state.focus.1, 1.0, h),
                KeyCode::Char('o') => {
                    state.encoder = match state.encoder {
                        Encoder::Octant => Encoder::Quadrant,
                        Encoder::Quadrant => Encoder::Octant,
                    }
                }
                KeyCode::Char('r') => state.raw = !state.raw,
                KeyCode::Char('+') => state.aspect = (state.aspect + 0.1).min(4.0),
                KeyCode::Char('-') => state.aspect = (state.aspect - 0.1).max(1.0),
                _ => {}
            }
        }
    })();
    ratatui::restore();
    result
}

fn draw(
    world: &World,
    state: &Live,
    frame: &mut Frame,
    t: f32,
    full: Rect,
    buf: &mut Buffer,
) -> Result<()> {
    let area = Rect {
        height: full.height.saturating_sub(1),
        ..full
    };
    let pins = [Pin {
        at: state.focus,
        glyph: '+',
        color: [255, 255, 255],
    }];
    let place = world.place();
    let mode = if state.raw {
        let camera = Camera::raw(area, (place.width(), place.height()), state.focus);
        draw_raw(place, &camera, area, buf);
        draw_pins(&pins, &camera, area, buf);
        "raw".to_string()
    } else {
        let rect = TileRect::around_within(state.focus, WINDOW.0, WINDOW.1, whole(world));
        let camera = Camera::overview(area, rect, state.focus, LIMIT, state.aspect)?;
        frame.dynamic.clear();
        frame.dynamic.push(focus_light(state.focus));
        world.render(frame, t, &camera, state.encoder, area, buf)?;
        draw_pins(&pins, &camera, area, buf);
        format!(
            "{:?} {:.2} cells/tile",
            state.encoder,
            camera.cells_per_tile()
        )
    };
    let status = format!(
        " {mode} | aspect {:.1} | focus {:.1},{:.1} | arrows pan  o encoder  r raw  +/- aspect  q quit",
        state.aspect, state.focus.0, state.focus.1
    );
    let line = Rect {
        y: full.bottom().saturating_sub(1),
        height: full.height.min(1),
        ..full
    };
    buf.set_style(
        line,
        Style::new()
            .bg(Color::Rgb(24, 24, 28))
            .fg(Color::Rgb(200, 200, 200)),
    );
    buf.set_stringn(
        line.x,
        line.y,
        status,
        usize::from(line.width),
        Style::new(),
    );
    Ok(())
}
