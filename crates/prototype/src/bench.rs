// ADR: --bench draws headless into a fixed 160x50 crossterm viewport whose writer only counts bytes, with one scripted intent per frame on a fake 30 fps clock; it prints frame build time and bytes written, and writes no files.
use std::cell::Cell;
use std::io::{self, Write};
use std::rc::Rc;
use std::time::{Duration, Instant};

use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::{Terminal, TerminalOptions, Viewport};

use crate::FRAME;
use crate::app::App;
use crate::keys::Intent;
use crate::preview::View;
use crate::settings::Settings;
use crate::ui;

/// Value changes and moves that never reach the buttons, so nothing is saved.
const CYCLE: [Intent; 12] = [
    Intent::Right,
    Intent::Down,
    Intent::Right,
    Intent::FineLeft,
    Intent::Up,
    Intent::Left,
    Intent::Down,
    Intent::Down,
    Intent::Left,
    Intent::Up,
    Intent::Up,
    Intent::FineRight,
];

struct CountingWriter(Rc<Cell<u64>>);

impl Write for CountingWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.0.set(self.0.get() + data.len() as u64);
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Draws `frames` frames: home, then setup with a value change each frame (the place preview in
/// whichever renderer the draft holds); every 60th frame cancels setup (a notice with its
/// draining bar) and the next opens it again.
pub fn run(frames: u32, mut view: View, output: &mut impl Write) -> io::Result<()> {
    // Never written: the bench presses no Save.
    let data = std::env::temp_dir().join("prototype-bench-unused");
    let mut app = App::open(data, None, Some(Settings::default()));
    let area = Rect::new(0, 0, 160, 50);
    let bytes = Rc::new(Cell::new(0u64));
    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(CountingWriter(Rc::clone(&bytes))),
        TerminalOptions {
            viewport: Viewport::Fixed(area),
        },
    )?;
    let clock = Instant::now();
    // (build time, bytes written) per frame: lit place previews, then all other frames.
    let mut lit = Vec::with_capacity(frames as usize);
    let mut other = Vec::with_capacity(frames as usize);
    for n in 0..frames {
        let now = clock + FRAME * n;
        let intent = match n % 60 {
            0 => Intent::Activate,
            59 => Intent::Back,
            k => CYCLE[(k as usize - 1) % CYCLE.len()],
        };
        app.step(now);
        app.intent(intent, now);
        let before = bytes.get();
        let mut build = Duration::ZERO;
        terminal.draw(|frame| {
            let started = Instant::now();
            ui::draw(&app, &mut view, frame.area(), frame.buffer_mut(), now);
            build = started.elapsed();
        })?;
        let sample = (build, bytes.get() - before);
        // The first frame paints every cell; it is reported apart.
        if n == 0 {
            writeln!(output, "first frame: {} B written", sample.1)?;
        } else if view.animating() {
            lit.push(sample);
        } else {
            other.push(sample);
        }
    }
    report(output, "setup with lit place preview", &lit)?;
    report(output, "home, ascii preview, other", &other)
}

fn report(output: &mut impl Write, label: &str, samples: &[(Duration, u64)]) -> io::Result<()> {
    let n = samples.len().max(1);
    let ms = |d: Duration| d.as_secs_f64() * 1000.0;
    let mean = samples.iter().map(|s| ms(s.0)).sum::<f64>() / n as f64;
    let worst = samples.iter().map(|s| ms(s.0)).fold(0.0, f64::max);
    let mean_bytes = samples.iter().map(|s| s.1).sum::<u64>() / n as u64;
    let worst_bytes = samples.iter().map(|s| s.1).max().unwrap_or(0);
    writeln!(
        output,
        "{} frames at 160x50, {label}: build mean {mean:.3} ms, worst {worst:.3} ms (budget {} ms); \
         written mean {mean_bytes} B, worst {worst_bytes} B per frame",
        samples.len(),
        FRAME.as_millis()
    )
}
