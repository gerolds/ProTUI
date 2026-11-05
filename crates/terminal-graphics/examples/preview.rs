// ADR: One preview drives both the live viewer and headless captures through the same `draw`, so an exported PNG shows exactly what the terminal would get.
// ADR: The encoder and cell aspect are explicit settings (flags or keys); the preview never guesses the font.
use std::error::Error;
use std::io::{self, BufReader, Cursor, IsTerminal};
use std::path::{Path, PathBuf};

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
use terminal_graphics::capture::{Animation, CellFont, Picture};
use terminal_graphics::{Encoder, ImageCache, Ink, Rgb, StillImage, calibration, encode_image};

type Result<T, E = Box<dyn Error>> = std::result::Result<T, E>;

const HELP: &str = "Terminal graphics preview: a still image beside the cell-aspect guide.
Usage: cargo run -p terminal-graphics --example preview -- [options]
  --png PATH           Show this PNG instead of the built-in sample
  --quadrant           Start with quadrants (default: octants)
  --aspect X           Cell height/width (default 2.0; export: the font's)
  --export OUT.png     Headless: write OUT.png, OUT-sheet.png and OUT.apng
  --font FONT.ttf      Font for --export; must have the octant glyphs
  --smoke              Headless: encode every layout without a font
  --help               Print this help
Live keys: Tab source, Space quadrant/octant, Left/Right aspect -/+0.05,
Backspace aspect 2.0, Esc or q quit.
";

const EXPORT_SIZE: (u16, u16) = (100, 30);

struct Options {
    png: Option<PathBuf>,
    encoder: Encoder,
    aspect: Option<f32>,
    export: Option<PathBuf>,
    font: Option<PathBuf>,
    smoke: bool,
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("preview: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let mut options = Options {
        png: None,
        encoder: Encoder::Octant,
        aspect: None,
        export: None,
        font: None,
        smoke: false,
    };
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        match arg.to_str() {
            Some("--help") => {
                print!("{HELP}");
                return Ok(());
            }
            Some("--png") => options.png = Some(value("--png")?.into()),
            Some("--quadrant") => options.encoder = Encoder::Quadrant,
            Some("--aspect") => {
                let text = value("--aspect")?;
                let aspect = text
                    .to_str()
                    .and_then(|t| t.parse::<f32>().ok())
                    .filter(|a| a.is_finite() && *a > 0.0)
                    .ok_or(format!("--aspect {text:?}: expected a positive number"))?;
                options.aspect = Some(aspect);
            }
            Some("--export") => options.export = Some(value("--export")?.into()),
            Some("--font") => options.font = Some(value("--font")?.into()),
            Some("--smoke") => options.smoke = true,
            _ => return Err(format!("unknown option {arg:?}; use --help").into()),
        }
    }
    let mut preview = Preview::new(options.png.as_deref(), options.encoder)?;
    match (options.export, options.font, options.smoke) {
        (Some(out), Some(font), false) => export(&mut preview, &out, &font, options.aspect),
        (None, None, true) => smoke(&mut preview),
        (None, None, false) => {
            preview.set_aspect(options.aspect.unwrap_or(2.0))?;
            interactive(&mut preview)
        }
        _ => Err("use --export OUT --font FONT together, or --smoke alone".into()),
    }
}

struct Preview {
    images: [StillImage; 2],
    names: [&'static str; 2],
    cache: ImageCache,
    guide: Option<((u16, u16, u32), StillImage)>,
    source: usize,
    encoder: Encoder,
    aspect: f32,
}

impl Preview {
    fn new(png: Option<&Path>, encoder: Encoder) -> Result<Self> {
        let still = match png {
            Some(path) => StillImage::from_png(
                BufReader::new(std::fs::File::open(path)?),
                Rgb::from_srgb8([12; 3]),
            )?,
            // The built-in sample goes through the real PNG decoder too.
            None => StillImage::from_png(Cursor::new(sample_png()?), Rgb::BLACK)?,
        };
        let ascii = StillImage::from_ascii(SAMPLE_ROOM, &SAMPLE_PALETTE)?;
        Ok(Self {
            images: [still, ascii],
            names: ["PNG", "ASCII"],
            cache: ImageCache::default(),
            guide: None,
            source: 0,
            encoder,
            aspect: 2.0,
        })
    }

    fn set_aspect(&mut self, aspect: f32) -> Result<()> {
        self.cache.set_cell_aspect(aspect)?;
        self.aspect = aspect;
        Ok(())
    }

    /// Returns true when the key asks to quit.
    fn input(&mut self, key: KeyCode) -> Result<bool> {
        match key {
            KeyCode::Tab => self.source = 1 - self.source,
            KeyCode::Char(' ') => {
                self.encoder = match self.encoder {
                    Encoder::Octant => Encoder::Quadrant,
                    Encoder::Quadrant => Encoder::Octant,
                }
            }
            KeyCode::Left => self.set_aspect((self.aspect - 0.05).max(1.0))?,
            KeyCode::Right => self.set_aspect((self.aspect + 0.05).min(4.0))?,
            KeyCode::Backspace => self.set_aspect(2.0)?,
            KeyCode::Esc | KeyCode::Char('q') => return Ok(true),
            _ => {}
        }
        Ok(false)
    }

    fn draw(&mut self, area: Rect, buffer: &mut Buffer) -> Result<()> {
        let [heading, body, footer] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(area);
        Paragraph::new("terminal-graphics preview").render(heading, buffer);
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(body);
        let guide_block = Block::default()
            .borders(Borders::ALL)
            .title("Aspect guide: round circle, square square?");
        let guide_area = guide_block.inner(left);
        guide_block.render(left, buffer);
        if !guide_area.is_empty() {
            let key = (guide_area.width, guide_area.height, self.aspect.to_bits());
            if self.guide.as_ref().is_none_or(|(k, _)| *k != key) {
                let ground = Ink::opaque(Rgb::from_srgb8([12; 3]))?;
                let ink = Ink::opaque(Rgb::from_srgb8([230; 3]))?;
                let image = calibration(
                    guide_area.width,
                    guide_area.height,
                    self.aspect,
                    ground,
                    ink,
                )?;
                self.guide = Some((key, image));
            }
            if let Some((_, image)) = &self.guide {
                encode_image(image.raster(), self.encoder, guide_area, buffer);
            }
        }
        let still_block = Block::default()
            .borders(Borders::ALL)
            .title(format!("{} still, centre-cropped", self.names[self.source]));
        let still_area = still_block.inner(right);
        still_block.render(right, buffer);
        if !still_area.is_empty() {
            self.cache.draw(
                &self.images[self.source],
                Rgb::WHITE,
                self.encoder,
                still_area,
                buffer,
            )?;
        }
        Paragraph::new(format!(
            "{:?}  cell h/w {:.2}  |  Tab source  Space encoder  Left/Right aspect  q quit",
            self.encoder, self.aspect
        ))
        .render(footer, buffer);
        Ok(())
    }

    fn frame(&mut self, size: (u16, u16)) -> Result<Buffer> {
        let area = Rect::new(0, 0, size.0, size.1);
        let mut buffer = Buffer::empty(area);
        self.draw(area, &mut buffer)?;
        Ok(buffer)
    }
}

/// A plain test room: walls, floor, one round object.
const SAMPLE_ROOM: &str = "\
################
#..............#
#..............#
#.....oooo.....#
#....oooooo....#
#....oooooo....#
#.....oooo.....#
#..............#
#..............#
################";

const SAMPLE_PALETTE: [(u8, Rgb); 3] = [
    (b'#', Rgb::new(0.35, 0.35, 0.35)),
    (b'.', Rgb::new(0.04, 0.04, 0.04)),
    (b'o', Rgb::new(0.9, 0.9, 0.9)),
];

/// A neutral test card: grey ramp, white ring, red square, blue triangle.
fn sample_png() -> Result<Vec<u8>> {
    const W: u32 = 80;
    const H: u32 = 64;
    let mut rgb = Vec::with_capacity((W * H * 3) as usize);
    for y in 0..H {
        for x in 0..W {
            let (dx, dy) = (x as f32 - 39.5, y as f32 - 33.5);
            let ring = (dx * dx + dy * dy).sqrt();
            let color = if (12.0..18.0).contains(&ring) {
                [240, 240, 240]
            } else if (56..68).contains(&x) && (4..16).contains(&y) {
                [200, 40, 40]
            } else if (46..62).contains(&y) && x >= 12 && x - 12 <= y - 46 {
                [40, 80, 210]
            } else {
                let v = (x * 120 / W) as u8 + 20;
                [v, v, v]
            };
            rgb.extend_from_slice(&color);
        }
    }
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, W, H);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&rgb)?;
        writer.finish()?;
    }
    Ok(bytes)
}

fn smoke(preview: &mut Preview) -> Result<()> {
    for size in [(80, 24), (160, 50), (3, 3), (1, 1)] {
        for source in 0..2 {
            for encoder in [Encoder::Octant, Encoder::Quadrant] {
                preview.source = source;
                preview.encoder = encoder;
                let buffer = preview.frame(size)?;
                let glyphs = buffer
                    .content
                    .iter()
                    .filter(|c| {
                        let table = match encoder {
                            Encoder::Octant => &ratatui::symbols::pixel::OCTANTS[1..],
                            Encoder::Quadrant => &ratatui::symbols::pixel::QUADRANTS[1..],
                        };
                        c.symbol()
                            .chars()
                            .next()
                            .is_some_and(|s| table.contains(&s))
                    })
                    .count();
                println!(
                    "{}x{} {} {encoder:?}: {glyphs} block cells",
                    size.0, size.1, preview.names[source]
                );
            }
        }
    }
    Ok(())
}

fn export(preview: &mut Preview, out: &Path, font: &Path, aspect: Option<f32>) -> Result<()> {
    let mut font = CellFont::load(font)?;
    let font_aspect = font.cell_aspect();
    let aspect = aspect.unwrap_or(font_aspect);
    preview.set_aspect(aspect)?;
    let frame = font.rasterize(&preview.frame(EXPORT_SIZE)?)?;
    frame.save_png(out)?;
    let (cw, ch) = font.cell_size();
    println!(
        "{}: {:?} px, {:?}, cell {cw}x{ch} px, aspect {aspect:.3} (font {font_aspect:.3})",
        out.display(),
        frame.size(),
        preview.encoder
    );

    // Both encoders at the font's aspect and at 2.0, on a half-size frame.
    let small = (EXPORT_SIZE.0 / 2 + 10, EXPORT_SIZE.1 / 2 + 4);
    let mut panels: Vec<(String, Picture)> = Vec::new();
    for panel_aspect in [font_aspect, 2.0] {
        for encoder in [Encoder::Octant, Encoder::Quadrant] {
            preview.encoder = encoder;
            preview.set_aspect(panel_aspect)?;
            let picture = font.rasterize(&preview.frame(small)?)?;
            panels.push((format!("{encoder:?}, aspect {panel_aspect:.2}"), picture));
        }
    }
    let refs: Vec<(&str, &Picture)> = panels.iter().map(|(t, p)| (t.as_str(), p)).collect();
    let sheet = font.contact_sheet(&refs, 2)?;
    let sheet_path = sibling(out, "-sheet", "png");
    sheet.save_png(&sheet_path)?;
    println!(
        "{}: {:?} px, {} panels",
        sheet_path.display(),
        sheet.size(),
        refs.len()
    );

    // The aspect setting swept from 1.6 to 3.0, as a player would adjust it.
    preview.encoder = Encoder::Octant;
    const FRAMES: u32 = 15;
    let sweep = |i: u32| 1.6 + 1.4 * i as f32 / (FRAMES - 1) as f32;
    preview.set_aspect(sweep(0))?;
    let apng_path = sibling(out, "", "apng");
    let first = font.rasterize(&preview.frame(small)?)?;
    let mut animation = Animation::create(&apng_path, first, FRAMES, 4)?;
    for i in 1..FRAMES {
        preview.set_aspect(sweep(i))?;
        animation.push(font.rasterize(&preview.frame(small)?)?)?;
    }
    animation.finish()?;
    println!("{}: {FRAMES} frames at 4 fps", apng_path.display());
    Ok(())
}

/// `out` with `suffix` added to its stem and extension `ext`.
fn sibling(out: &Path, suffix: &str, ext: &str) -> PathBuf {
    let stem = out.file_stem().unwrap_or_default().to_string_lossy();
    out.with_file_name(format!("{stem}{suffix}.{ext}"))
}

fn interactive(preview: &mut Preview) -> Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("live mode needs a terminal; use --export or --smoke".into());
    }
    let mut terminal = ratatui::try_init()?;
    let result = (|| -> Result<()> {
        loop {
            let mut drawn = Ok(());
            terminal.draw(|frame| {
                let area = frame.area();
                drawn = preview.draw(area, frame.buffer_mut());
            })?;
            drawn?;
            loop {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        if preview.input(key.code)? {
                            return Ok(());
                        }
                        break;
                    }
                    Event::Resize(..) => break,
                    _ => {}
                }
            }
        }
    })();
    let restored = ratatui::try_restore();
    result?;
    restored?;
    Ok(())
}
