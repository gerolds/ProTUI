// ADR: Probe, not product: the lit sample place with flicker and a moving light, shown in a fixed cell rect as sixel at 2×4, 5×10 or 10×20 pixels per cell (Windows Terminal's 10×20 virtual cell divided by whole numbers) or as octants, beside ordinary Ratatui text panels.
// ADR: Headless `--bench` measures build, encode and bytes per density and palette; live mode logs measured fps and frame times once a second to `--log`, because an agent cannot see motion.
use std::error::Error;
use std::fs::File;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::crossterm::terminal;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Paragraph, Widget};
use terminal_graphics::Encoder;
use terminal_graphics::sixel::{Palette, SixelEncoder, SixelLayer, SixelOptions, WT_CELL_PX};
use terminal_tilemap::{Camera, DynamicLight, Frame, Light, TileRect, World, load_place};

type Result<T, E = Box<dyn Error>> = std::result::Result<T, E>;

const HELP: &str = "Sixel probe on the lit sample place
Usage: cargo run --release -p terminal-tilemap --example sixel_probe -- [options]
  (no options)          Live view; stdin and stdout must be terminals
  --density D           2x4 | 5x10 | 10x20 sixel pixels per cell (default 2x4)
  --palette P           fixed | adaptive | adaptive64 (default adaptive)
  --octant              Start in the octant view
  --uncapped            Draw as fast as the terminal takes it instead of 30 fps
  --seconds N           Quit after N seconds
  --log FILE            Append one stats line per second
  --bench N             Headless: N frames per density and palette, no terminal
  --query FILE          Ask the terminal for DA1, cell size and mode 2026; write the replies
  --script S            Press keys at set times, e.g. 2:o,4:o,6:m,8:c (seconds:key)
  --help                Print help
Live keys: d density, p palette, o octant/sixel, m move and shrink the image, c clear screen,
q quits.";

const VIEW: (u16, u16) = (60, 20);
const DENSITIES: [(usize, usize); 3] = [(2, 4), (5, 10), (10, 20)];
const PALETTES: [(Palette, &str); 3] = [
    (Palette::Fixed, "fixed"),
    (Palette::Adaptive { colors: 256 }, "adaptive"),
    (Palette::Adaptive { colors: 64 }, "adaptive64"),
];
const FRAME: Duration = Duration::from_micros(33_333);

struct Options {
    density: usize,
    palette: usize,
    octant: bool,
    uncapped: bool,
    seconds: Option<f64>,
    log: Option<PathBuf>,
    bench: Option<usize>,
    query: Option<PathBuf>,
    script: Vec<(f64, char)>,
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("sixel_probe: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let options = parse_options()?;
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/sample.toml");
    let world = World::new(load_place(&path)?);
    if let Some(frames) = options.bench {
        return bench(&world, frames);
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("the live view needs a terminal; use --bench headless".into());
    }
    if let Some(out) = &options.query {
        return query(out);
    }
    live(&world, &options)
}

fn parse_options() -> Result<Options> {
    let mut options = Options {
        density: 0,
        palette: 1,
        octant: false,
        uncapped: false,
        seconds: None,
        log: None,
        bench: None,
        query: None,
        script: Vec::new(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or(format!("{arg} needs a value"));
        match arg.as_str() {
            "--help" => {
                println!("{HELP}");
                std::process::exit(0);
            }
            "--density" => {
                let text = value()?;
                options.density = DENSITIES
                    .iter()
                    .position(|&(x, y)| text == format!("{x}x{y}"))
                    .ok_or(format!("unknown density {text:?}; use 2x4, 5x10 or 10x20"))?;
            }
            "--palette" => {
                let text = value()?;
                options.palette = PALETTES
                    .iter()
                    .position(|&(_, name)| text == name)
                    .ok_or(format!("unknown palette {text:?}"))?;
            }
            "--octant" => options.octant = true,
            "--uncapped" => options.uncapped = true,
            "--seconds" => options.seconds = Some(value()?.parse()?),
            "--log" => options.log = Some(value()?.into()),
            "--bench" => options.bench = Some(value()?.parse()?),
            "--query" => options.query = Some(value()?.into()),
            "--script" => options.script = parse_script(&value()?)?,
            _ => return Err(format!("unknown option {arg:?}; use --help").into()),
        }
    }
    Ok(options)
}

/// `SECONDS:KEY` pairs, comma separated, in time order.
fn parse_script(text: &str) -> Result<Vec<(f64, char)>> {
    let mut script = Vec::new();
    for step in text.split(',') {
        let (at, key) = step
            .split_once(':')
            .ok_or(format!("script step {step:?} is not SECONDS:KEY"))?;
        let mut chars = key.chars();
        let (Some(key), None) = (chars.next(), chars.next()) else {
            return Err(format!("script key {key:?} is not one character").into());
        };
        let at: f64 = at.parse()?;
        if script.last().is_some_and(|&(last, _)| at < last) {
            return Err(format!("script step {step:?} is earlier than the one before").into());
        }
        script.push((at, key));
    }
    Ok(script)
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// The animated scene at `t`: lamps flicker by themselves; one light walks the corridor.
fn scene(world: &World, frame: &mut Frame, t: f32) {
    let span = world.place().width() as f32 - 3.0;
    frame.dynamic.clear();
    frame.dynamic.push(DynamicLight {
        key: 1,
        light: Light {
            x: 1.5 + (t * 6.0) % span,
            y: 5.5,
            color: terminal_graphics::Rgb::new(0.95, 0.8, 0.55),
            intensity: 1.6,
            radius: 5.0,
            flicker: 0.2,
            glow: 1.0,
            seed: 7,
        },
    });
}

fn camera(world: &World, view: Rect) -> Result<Camera> {
    let rect = TileRect::whole(world.place().width(), world.place().height());
    let centre = (rect.x1 / 2.0, rect.y1 / 2.0);
    Ok(Camera::overview(view, rect, centre, 8.0, 2.0)?)
}

/// Mean and worst of a series.
#[derive(Default)]
struct Series {
    sum: f64,
    worst: f64,
    count: usize,
}

impl Series {
    fn add(&mut self, v: f64) {
        self.sum += v;
        self.worst = self.worst.max(v);
        self.count += 1;
    }
    fn mean(&self) -> f64 {
        self.sum / self.count.max(1) as f64
    }
}

/// Counts bytes a backend writes.
#[derive(Default)]
struct Count(usize);

impl Write for Count {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0 += buf.len();
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn bench(world: &World, frames: usize) -> Result<()> {
    let view = Rect::new(0, 0, VIEW.0, VIEW.1);
    let camera = camera(world, view)?;
    println!(
        "view {}x{} cells at {:.2} cells per tile, {frames} frames per run, moving light + flicker",
        VIEW.0,
        VIEW.1,
        camera.cells_per_tile()
    );
    // Octant baseline: render into a buffer, diff against the last frame, count the bytes.
    let mut frame = Frame::default();
    let (mut prev, mut next) = (Buffer::empty(view), Buffer::empty(view));
    let (mut build, mut bytes) = (Series::default(), Series::default());
    for i in 0..frames {
        let t = i as f32 / 30.0;
        scene(world, &mut frame, t);
        let started = Instant::now();
        world.render(&mut frame, t, &camera, Encoder::Octant, view, &mut next)?;
        build.add(ms(started.elapsed()));
        let mut sink = Count::default();
        CrosstermBackend::new(&mut sink).draw(prev.diff_iter(&next))?;
        bytes.add(sink.0 as f64);
        std::mem::swap(&mut prev, &mut next);
    }
    println!(
        "octant                 : build+encode {:.2} ms (worst {:.2}), {:.0} bytes/frame (worst {:.0})",
        build.mean(),
        build.worst,
        bytes.mean(),
        bytes.worst
    );
    let mut encoder = SixelEncoder::new();
    let mut out = Vec::new();
    for (sx, sy) in DENSITIES {
        let scale = (WT_CELL_PX.0 / sx as u16, WT_CELL_PX.1 / sy as u16);
        for (palette, name) in PALETTES {
            let options = SixelOptions { palette, scale };
            let (mut build, mut encode, mut bytes, mut colors) = (
                Series::default(),
                Series::default(),
                Series::default(),
                Series::default(),
            );
            for i in 0..frames {
                let t = i as f32 / 30.0;
                scene(world, &mut frame, t);
                let started = Instant::now();
                let raster = world.render_raster(&mut frame, t, &camera, view, (sx, sy))?;
                build.add(ms(started.elapsed()));
                out.clear();
                let started = Instant::now();
                encoder.encode(raster, &options, &mut out)?;
                encode.add(ms(started.elapsed()));
                bytes.add(out.len() as f64);
                colors.add(encoder.colors() as f64);
            }
            println!(
                "sixel {sx:>2}x{sy:<2} {name:10}: build {:.2} ms, encode {:.2} ms (worst {:.2}), {:.0} bytes/frame (worst {:.0}), {:.0} colours",
                build.mean(),
                encode.mean(),
                encode.worst,
                bytes.mean(),
                bytes.worst,
                colors.mean()
            );
        }
    }
    Ok(())
}

/// Sends DA1, the cell-size and text-area queries and a DECRQM for mode 2026, and writes what
/// comes back (as key events) to `out`.
fn query(out: &Path) -> Result<()> {
    terminal::enable_raw_mode()?;
    let result = (|| -> Result<String> {
        let mut stdout = io::stdout();
        stdout.write_all(b"\x1b[c\x1b[16t\x1b[14t\x1b[?2026$p")?;
        stdout.flush()?;
        let mut reply = String::new();
        let deadline = Instant::now() + Duration::from_millis(1500);
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            if !event::poll(left)? {
                break;
            }
            if let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                match key.code {
                    KeyCode::Esc => reply.push_str("ESC"),
                    KeyCode::Char(c) => reply.push(c),
                    other => reply.push_str(&format!("<{other:?}>")),
                }
            }
        }
        Ok(reply)
    })();
    terminal::disable_raw_mode()?;
    let reply = result?;
    std::fs::write(out, format!("{reply}\n"))?;
    println!("replies: {reply}");
    std::thread::sleep(Duration::from_secs(3));
    Ok(())
}

#[derive(Default)]
struct Stats {
    build: Series,
    encode: Series,
    draw: Series,
    frame: Series,
    bytes: Series,
    frames: usize,
}

fn live(world: &World, options: &Options) -> Result<()> {
    let mut log = match &options.log {
        Some(path) => Some(File::options().create(true).append(true).open(path)?),
        None => None,
    };
    let mut terminal = ratatui::init();
    let result = live_loop(world, options, &mut terminal, &mut log);
    ratatui::restore();
    result
}

/// What the live view shows; changed by keys or by `--script`.
struct View {
    density: usize,
    palette: usize,
    octant: bool,
    small: bool,
}

enum Action {
    None,
    Clear,
    Quit,
}

impl View {
    fn key(&mut self, key: char) -> Action {
        match key {
            'q' => return Action::Quit,
            'c' => return Action::Clear,
            'd' => self.density = (self.density + 1) % DENSITIES.len(),
            'p' => self.palette = (self.palette + 1) % PALETTES.len(),
            'o' => self.octant = !self.octant,
            'm' => self.small = !self.small,
            _ => {}
        }
        Action::None
    }

    fn mode(&self) -> String {
        let (sx, sy) = DENSITIES[self.density];
        let place = if self.small { ", moved" } else { "" };
        if self.octant {
            format!("octant{place}")
        } else {
            format!("sixel {sx}x{sy} {}{place}", PALETTES[self.palette].1)
        }
    }

    /// The image rect inside the frame `outer`: the whole inside, or a smaller, shifted rect.
    fn rect(&self, outer: Rect) -> Rect {
        let inside = outer.inner(ratatui::layout::Margin::new(1, 1));
        if !self.small {
            return inside;
        }
        let (w, h) = (inside.width * 2 / 3, inside.height * 3 / 5);
        Rect::new(inside.right() - w, inside.bottom() - h, w, h)
    }
}

fn live_loop(
    world: &World,
    options: &Options,
    terminal: &mut ratatui::DefaultTerminal,
    log: &mut Option<File>,
) -> Result<()> {
    let mut layer = SixelLayer::new(WT_CELL_PX);
    let mut frame = Frame::default();
    let mut state = View {
        density: options.density,
        palette: options.palette,
        octant: options.octant,
        small: false,
    };
    let mut script = options.script.iter().peekable();
    let started = Instant::now();
    let mut window = Stats::default();
    let mut shown = String::from("measuring…");
    let mut window_start = Instant::now();
    let mut next_frame = Instant::now();
    let mut revision = 0u64;
    loop {
        let frame_start = Instant::now();
        let t = started.elapsed().as_secs_f32();
        let (sx, sy) = DENSITIES[state.density];
        let mut build_ms = 0.0;
        let mut encode_ms = 0.0;
        let mut failure: Option<Box<dyn Error>> = None;
        let mode = state.mode();
        revision += 1;
        layer.draw(terminal, |f, layer| {
            let full = f.area();
            let outer = Rect::new(
                0,
                0,
                (VIEW.0 + 2).min(full.width),
                (VIEW.1 + 2).min(full.height),
            );
            let view = state.rect(outer);
            Block::bordered()
                .title(format!(" {mode} "))
                .render(outer, f.buffer_mut());
            let panel = Rect::new(
                outer.right(),
                0,
                full.width.saturating_sub(outer.right()),
                outer.height,
            );
            Paragraph::new(shown.as_str())
                .block(Block::bordered().title(" probe "))
                .render(panel, f.buffer_mut());
            let keys = Rect::new(
                0,
                outer.bottom().min(full.bottom().saturating_sub(1)),
                full.width,
                1,
            );
            f.buffer_mut().set_stringn(
                keys.x,
                keys.y,
                " d density  p palette  o octant/sixel  m move  c clear  q quit",
                usize::from(keys.width),
                Style::new().fg(Color::Rgb(200, 200, 200)),
            );
            if view.is_empty() {
                return;
            }
            let step = (|| -> Result<()> {
                let camera = camera(world, view)?;
                scene(world, &mut frame, t);
                let begun = Instant::now();
                if state.octant {
                    world.render(
                        &mut frame,
                        t,
                        &camera,
                        Encoder::Octant,
                        view,
                        f.buffer_mut(),
                    )?;
                    build_ms = ms(begun.elapsed());
                    return Ok(());
                }
                let raster = world.render_raster(&mut frame, t, &camera, view, (sx, sy))?;
                build_ms = ms(begun.elapsed());
                let begun = Instant::now();
                let palette = PALETTES[state.palette].0;
                layer.place(f.buffer_mut(), 1, revision, view, raster, palette)?;
                encode_ms = ms(begun.elapsed());
                Ok(())
            })();
            if let Err(e) = step {
                failure = Some(e);
            }
        })?;
        if let Some(e) = failure {
            return Err(e);
        }
        let frame_ms = ms(frame_start.elapsed());
        window.build.add(build_ms);
        window.encode.add(encode_ms);
        window.draw.add(frame_ms - build_ms - encode_ms);
        window.frame.add(frame_ms);
        window.bytes.add(layer.pending().len() as f64);
        window.frames += 1;
        let elapsed = window_start.elapsed();
        if elapsed >= Duration::from_secs(1) {
            let fps = window.frames as f64 / elapsed.as_secs_f64();
            shown = format!(
                "{mode}\nfps {fps:.1} ({})\nbuild {:.2} ms\nencode {:.2} ms\nwrite+diff {:.2} ms\nframe {:.2} ms (worst {:.1})\nsixel bytes/frame {:.0}\ncolours {}",
                if options.uncapped {
                    "uncapped"
                } else {
                    "target 30"
                },
                window.build.mean(),
                window.encode.mean(),
                window.draw.mean(),
                window.frame.mean(),
                window.frame.worst,
                window.bytes.mean(),
                layer.colors()
            );
            if let Some(file) = log.as_mut() {
                let size = terminal.size()?;
                writeln!(
                    file,
                    "t={t:.1} mode={mode:?} fps={fps:.1} build={:.2} encode={:.2} write={:.2} (worst {:.2}) frame={:.2} (worst {:.2}) bytes={:.0} size={}x{}",
                    window.build.mean(),
                    window.encode.mean(),
                    window.draw.mean(),
                    window.draw.worst,
                    window.frame.mean(),
                    window.frame.worst,
                    window.bytes.mean(),
                    size.width,
                    size.height,
                )?;
                file.flush()?;
            }
            window = Stats::default();
            window_start = Instant::now();
        }
        if options
            .seconds
            .is_some_and(|s| started.elapsed().as_secs_f64() >= s)
        {
            return Ok(());
        }
        next_frame += FRAME;
        let now = Instant::now();
        if options.uncapped || next_frame < now {
            next_frame = now;
        }
        let mut keys = Vec::new();
        while let Some(&&(at, key)) = script.peek() {
            if started.elapsed().as_secs_f64() < at {
                break;
            }
            keys.push(key);
            script.next();
        }
        // Keys are read until the next frame is due; the wait shrinks as time passes.
        while event::poll(next_frame.saturating_duration_since(Instant::now()))? {
            if let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
                && let KeyCode::Char(c) = key.code
            {
                keys.push(c);
            }
        }
        for key in keys {
            match state.key(key) {
                Action::Quit => return Ok(()),
                Action::Clear => {
                    terminal.clear()?;
                    layer.invalidate();
                }
                Action::None => {}
            }
        }
    }
}
