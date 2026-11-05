// ADR: --smoke runs the start-up path headlessly in a fresh temporary data folder with the sample place's view built once: key table, first start → setup → save → home, the settings round trip, a refused unreadable file left as it is, setup previews in each renderer, and frames at 80x24, 160x50 and 1x1 (80x24 printed).
// ADR: Its sound part never opens a device: the offline mixer plays setup's check rows (test cue, background off → ambience → music → off) from scripted presses and prints each request's log line with the recorded peak dBFS.
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use terminal_audio::{Level, OfflineBackend, Recording};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::{App, Row, Screen};
use crate::keys::{self, Intent};
use crate::preview::View;
use crate::settings::{self, Renderer, Settings};
use crate::sound::{self, Loaded, Sound};
use crate::ui;

const SIZES: [(u16, u16); 3] = [(80, 24), (160, 50), (1, 1)];

/// The offline mixer's rate.
const RATE: u32 = 44_100;

pub fn run(mut view: View, loaded: Loaded, output: &mut impl Write) -> io::Result<()> {
    check_keys()?;
    writeln!(
        output,
        "Keys OK: presses only, letters either case, Shift+arrows fine, Esc is back."
    )?;

    let dir = std::env::temp_dir().join(format!("prototype-smoke-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let result = flow(&dir, &mut view, output).and_then(|()| sound_checks(&dir, loaded, output));
    let _ = fs::remove_dir_all(&dir);
    result?;
    writeln!(
        output,
        "Smoke OK (headless; no real terminal was opened; temporary folder removed)."
    )
}

/// Setup's sound rows through the offline mixer: Enter on "Test sound", then three Enters on
/// "Background test", each followed by a render; prints each log line with its peak.
fn sound_checks(dir: &Path, loaded: Loaded, output: &mut impl Write) -> io::Result<()> {
    let settings = Settings::default();
    let mut sound = Sound::offline(loaded, settings.audio, RATE, 1).map_err(io::Error::other)?;
    let now = Instant::now();
    let mut app = App::open(dir.to_path_buf(), None, Some(settings));
    app.intent(Intent::Activate, now);
    while matches!(&app.screen, Screen::Setup(s) if Row::ALL[s.row] != Row::TestSound) {
        app.intent(Intent::Down, now);
    }
    require(
        app.requests.is_empty(),
        "moving the cursor requests no sound",
    )?;
    writeln!(
        output,
        "Sound (offline mixer, no device; peak of the mix after each request):"
    )?;
    let mut clock = Duration::ZERO;
    let mut press = |app: &mut App, length: Duration, output: &mut dyn Write| {
        app.intent(Intent::Activate, now);
        let mut peak = f32::NEG_INFINITY;
        for request in app.requests.drain(..) {
            let line = sound.handle(request, clock);
            let engine = sound
                .engine_mut()
                .ok_or_else(|| io::Error::other("the offline mixer is closed"))?;
            peak = render(engine, clock, length)?;
            clock += length;
            if let Some(line) = line {
                writeln!(output, "  {line} | peak {peak:.1} dBFS over {length:?}")?;
            }
        }
        if let Some(problem) = sound.problem() {
            return Err(io::Error::other(format!("sound problem: {problem}")));
        }
        Ok::<f32, io::Error>(peak)
    };
    let ms = Duration::from_millis;
    let test = press(&mut app, ms(500), output)?;
    app.intent(Intent::Down, now);
    let fade = sound::BACKGROUND_FADE + ms(100);
    let ambience = press(&mut app, fade, output)?;
    let music = press(&mut app, fade, output)?;
    press(&mut app, fade, output)?;
    // After the fade-out the layer is silent.
    let tail = {
        let engine = sound
            .engine_mut()
            .ok_or_else(|| io::Error::other("the offline mixer is closed"))?;
        render(engine, clock, ms(100))?
    };
    writeln!(output, "  silence after the fade-out: peak {tail:.1} dBFS")?;
    require(test > -40.0, "the test cue is audible in the mix")?;
    require(
        ambience > -60.0 && music > -60.0,
        "both background cues play",
    )?;
    require(tail < -90.0, "the background test fades out")?;
    app.intent(Intent::Back, now);
    require(
        app.screen == (Screen::Home { item: 0 }) && app.requests.is_empty(),
        "leaving setup with the background off requests nothing",
    )?;
    Ok(())
}

/// Renders `length` from `from`; the peak in dBFS.
fn render(
    engine: &mut terminal_audio::Engine<OfflineBackend>,
    from: Duration,
    length: Duration,
) -> io::Result<f32> {
    let mut recording = Recording::new(RATE);
    engine
        .run(from, length, &mut recording)
        .map_err(io::Error::other)?;
    Ok(Level::of(recording.samples()).peak_dbfs())
}

fn flow(dir: &Path, view: &mut View, output: &mut impl Write) -> io::Result<()> {
    let now = Instant::now();
    let loaded = settings::load(dir).map_err(io::Error::other)?;
    require(loaded.is_none(), "an empty data folder is a first start")?;
    let mut app = App::open(dir.to_path_buf(), None, loaded);
    require(
        matches!(&app.screen, Screen::Setup(s) if s.first_start),
        "a first start opens setup",
    )?;
    writeln!(output, "First start: no settings file; setup opens.")?;
    renderers(&mut app, view, output)?;
    frames(&app, view, "first-start setup", output)?;

    // Aspect 2.00 → 2.06, renderer quadrant, master 80 → 70, mute; then Tab to Save and Enter.
    let presses = [
        Intent::Right,
        Intent::FineRight,
        Intent::Down,
        Intent::Activate,
        Intent::Down,
        Intent::Left,
        Intent::Left,
        Intent::Back,
        Intent::Down,
        Intent::Down,
        Intent::Down,
        Intent::Right,
    ];
    for intent in presses {
        app.intent(intent, now);
    }
    require(
        !settings::path(dir).exists(),
        "nothing is written before Save",
    )?;
    app.intent(Intent::NextFocus, now);
    frames(&app, view, "setup with Save focused", output)?;
    app.intent(Intent::Activate, now);
    require(app.screen == Screen::Home { item: 0 }, "Save opens home")?;
    let saved = app
        .settings
        .ok_or_else(|| io::Error::other("Save kept no settings"))?;
    writeln!(output, "Saved; home opens. File text:")?;
    let text = fs::read_to_string(settings::path(dir))?;
    for line in text.lines() {
        writeln!(output, "  | {line}")?;
    }
    frames(&app, view, "home after save", output)?;

    let reread = settings::load(dir).map_err(io::Error::other)?;
    require(
        reread == Some(saved),
        "settings read back equal to what was saved",
    )?;
    require(
        saved.cell_aspect.hundredths() == 206
            && saved.renderer == settings::Renderer::Quadrant
            && saved.audio.master.percent() == 70
            && saved.audio.muted,
        "saved values are the edited ones",
    )?;
    writeln!(
        output,
        "Round trip OK: reread settings equal the saved ones."
    )?;

    // A later start opens home; setup from home, a change, Esc: home, file unchanged.
    let before = fs::read(settings::path(dir))?;
    let mut app = App::open(dir.to_path_buf(), None, reread);
    require(
        app.screen == Screen::Home { item: 0 },
        "a later start opens home",
    )?;
    for intent in [Intent::Activate, Intent::Right, Intent::Back] {
        app.intent(intent, now);
    }
    require(
        app.screen == Screen::Home { item: 0 } && !app.quit,
        "Esc goes back, never quits",
    )?;
    require(
        fs::read(settings::path(dir))? == before,
        "Esc writes nothing",
    )?;
    writeln!(
        output,
        "Later start: home; setup → change → Esc returns home, file unchanged."
    )?;

    let bad = b"version = 1\ncell_aspect = 9.0\n";
    fs::write(settings::path(dir), bad)?;
    match settings::load(dir) {
        Ok(_) => return Err(io::Error::other("an invalid settings file was accepted")),
        Err(error) => writeln!(output, "Invalid file refused: {error}")?,
    }
    require(
        settings::save(dir, &saved).is_err() && fs::read(settings::path(dir))? == bad,
        "an unreadable file is never overwritten",
    )?;
    writeln!(
        output,
        "Invalid file left as it was; a save over it is refused."
    )?;
    Ok(())
}

/// Draws `app` once at `width` × `height`; the buffer's rows.
fn draw(app: &App, view: &mut View, width: u16, height: u16) -> Vec<String> {
    let mut terminal =
        Terminal::new(TestBackend::new(width, height)).unwrap_or_else(|never| match never {});
    terminal
        .draw(|frame| ui::draw(app, view, frame.area(), frame.buffer_mut(), Instant::now()))
        .unwrap_or_else(|never| match never {});
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect())
        .collect()
}

fn print_rows(output: &mut impl Write, rows: &[String]) -> io::Result<()> {
    for (y, row) in rows.iter().enumerate() {
        let trimmed = row.trim_end();
        if !trimmed.is_empty() {
            writeln!(output, "  {y:02} | {trimmed}")?;
        }
    }
    Ok(())
}

/// Draws `app` at every size in `SIZES`; prints 80x24 whole and the others as a count.
fn frames(app: &App, view: &mut View, label: &str, output: &mut impl Write) -> io::Result<()> {
    for (width, height) in SIZES {
        let rows = draw(app, view, width, height);
        if (width, height) == (80, 24) {
            writeln!(output, "Frame {label} {width}x{height} (nonblank rows):")?;
            print_rows(output, &rows)?;
        } else {
            let cells = rows
                .iter()
                .flat_map(|r| r.chars())
                .filter(|c| *c != ' ')
                .count();
            writeln!(
                output,
                "Frame {label} {width}x{height}: drawn, {cells} nonblank cells"
            )?;
        }
    }
    Ok(())
}

fn is_octant(c: char) -> bool {
    ('\u{1CD00}'..='\u{1CDE5}').contains(&c)
}

fn is_quadrant(c: char) -> bool {
    ('\u{2596}'..='\u{259F}').contains(&c)
}

/// Setup with both previews in each renderer at 80x24 (printed) and 160x50: octant draws octant
/// glyphs; quadrant none; ascii shows the place's `#` walls and its pictures in quadrant.
fn renderers(app: &mut App, view: &mut View, output: &mut impl Write) -> io::Result<()> {
    for renderer in Renderer::ALL {
        let Screen::Setup(setup) = &mut app.screen else {
            return Err(io::Error::other("renderer check needs the setup screen"));
        };
        setup.draft.renderer = renderer;
        for (width, height) in [(80, 24), (160, 50)] {
            let rows = draw(app, view, width, height);
            // The preview rows: below the list at 80x24, right of it at 160x50.
            let preview: Vec<char> = if width < 100 {
                rows[16..23].iter().flat_map(|r| r.chars()).collect()
            } else {
                rows[4..49]
                    .iter()
                    .flat_map(|r| r.chars().skip(59))
                    .collect()
            };
            let octants = preview.iter().filter(|c| is_octant(**c)).count();
            let quadrants = preview.iter().filter(|c| is_quadrant(**c)).count();
            let walls = preview.iter().filter(|c| **c == '#').count();
            let ok = match renderer {
                Renderer::Octant => octants > 0,
                Renderer::Quadrant => octants == 0 && quadrants > 0,
                Renderer::Ascii => octants == 0 && quadrants > 0 && walls > 0,
            };
            writeln!(
                output,
                "Previews {} {width}x{height}: {octants} octant, {quadrants} quadrant, {walls} '#' cells",
                renderer.name()
            )?;
            require(ok, "the previews draw in the chosen renderer")?;
            if width == 80 {
                print_rows(output, &rows)?;
            }
        }
    }
    if let Screen::Setup(setup) = &mut app.screen {
        setup.draft.renderer = Renderer::Octant;
    }
    Ok(())
}

fn check_keys() -> io::Result<()> {
    let press = |code, modifiers| KeyEvent::new_with_kind(code, modifiers, KeyEventKind::Press);
    let cases = [
        (press(KeyCode::Esc, KeyModifiers::NONE), Some(Intent::Back)),
        (
            press(KeyCode::Char('C'), KeyModifiers::CONTROL),
            Some(Intent::Quit),
        ),
        (
            press(KeyCode::Left, KeyModifiers::SHIFT),
            Some(Intent::FineLeft),
        ),
        (
            press(KeyCode::BackTab, KeyModifiers::SHIFT),
            Some(Intent::PrevFocus),
        ),
        (
            press(KeyCode::Char('q'), KeyModifiers::NONE),
            Some(Intent::Other),
        ),
        (
            KeyEvent::new_with_kind(KeyCode::Enter, KeyModifiers::NONE, KeyEventKind::Release),
            None,
        ),
    ];
    for (key, expected) in cases {
        let actual = keys::intent(key);
        require(
            actual == expected,
            &format!("key {key:?}: expected {expected:?}, got {actual:?}"),
        )?;
    }
    Ok(())
}

fn require(condition: bool, message: &str) -> io::Result<()> {
    if condition {
        Ok(())
    } else {
        Err(io::Error::other(message.to_string()))
    }
}
