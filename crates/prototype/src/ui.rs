// ADR: Screens draw straight into the frame buffer from `&App` plus the render-side `View` (previews, frame counter) and allocate nothing per frame: numbers go through a stack buffer, text is cropped with an ellipsis, never wrapped.
// ADR: Focus: the cursor's item gets `▶` and a reversed backing (never colour alone); a changed draft value shows the saved value beside it; key hints read their caps from the key table.
// ADR: Setup puts its previews right of the list from 100 columns, else below it, and hides them with a one-line note when fewer than 4 rows are left. From 100 columns the hint row ends with version, commit and frames per second.
// ADR: Text widths count one cell per character: every string drawn here is single-width except paths, which may crop early.
// ADR: Sound status: without audio, home and setup's check rows read `audio off: <reason>`; a sound problem (bad manifest edit, device or log failure) holds the row above the key hints as a one-line strip until it clears, the notice moving above it.
use std::fmt::Write as _;
use std::time::Instant;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use crate::app::{App, Button, Group, HomeItem, NOTICE, NoticeKind, Row, Screen, Setup};
use crate::keys::{Intent, cap};
use crate::preview::View;
use crate::settings::{Settings, Volume};
use crate::sound::Background;

const TITLE: &str = "Protui prototype";
/// Version and commit for the corner and `--version`.
pub const BUILD: &str = concat!(
    "v",
    env!("CARGO_PKG_VERSION"),
    " ",
    env!("PROTOTYPE_COMMIT")
);
/// From this width the setup previews sit right of the list and the corner shows.
const WIDE: u16 = 100;
/// The setup list's column width in the wide layout.
const LIST_W: u16 = 56;

/// One key hint: the intents whose caps are shown together, then what they do.
type Hint = (&'static [Intent], &'static str);

const HOME_HINTS: &[Hint] = &[
    (&[Intent::Up, Intent::Down], "move"),
    (&[Intent::Activate], "select"),
    (&[Intent::Quit], "quit"),
];
const ROWS_HINTS: &[Hint] = &[
    (&[Intent::Up, Intent::Down], "move"),
    (&[Intent::Left, Intent::Right], "change"),
    (&[Intent::FineLeft, Intent::FineRight], "fine"),
    (&[Intent::NextFocus], "buttons"),
    (&[Intent::Back], "cancel"),
    (&[Intent::Quit], "quit"),
];
const BUTTONS_HINTS: &[Hint] = &[
    (&[Intent::Left, Intent::Right], "choose"),
    (&[Intent::Activate], "press"),
    (&[Intent::NextFocus], "settings"),
    (&[Intent::Back], "cancel"),
    (&[Intent::Quit], "quit"),
];

/// Draws `app` into `area` of `buf` at `now`; `view` holds the previews and counts the frame.
pub fn draw(app: &App, view: &mut View, area: Rect, buf: &mut Buffer, now: Instant) {
    view.begin(now);
    let mut p = Painter { buf, area };
    p.clear();
    let (screen_name, hints, first_start) = match &app.screen {
        Screen::Home { .. } => ("Home", HOME_HINTS, false),
        Screen::Setup(setup) if setup.group == Group::Rows => {
            ("Setup", ROWS_HINTS, setup.first_start)
        }
        Screen::Setup(setup) => ("Setup", BUTTONS_HINTS, setup.first_start),
    };
    p.title_bar(screen_name);
    let bottom = area.bottom();
    // The last row holds the key hints, the one above the sound problem strip, then a notice,
    // when there is room.
    let hints_y = (area.height >= 2).then(|| bottom - 1);
    let problem_y = (area.height >= 5 && app.sound_problem.is_some()).then(|| bottom - 2);
    let notice_above = problem_y.or(hints_y).unwrap_or(bottom);
    let notice_room = if problem_y.is_some() { 6 } else { 4 };
    let notice_y = (area.height >= notice_room && app.notice.is_some()).then(|| notice_above - 1);
    let body_end = notice_y.or(problem_y).or(hints_y).unwrap_or(bottom);
    match &app.screen {
        Screen::Home { item } => home(&mut p, app, *item, body_end),
        Screen::Setup(setup) => {
            setup_screen(&mut p, view, app, setup, body_end, now);
        }
    }
    if let (Some(y), Some(problem)) = (problem_y, &app.sound_problem) {
        let x = p.area.x + 1;
        let label_end = p.text(x, y, 7, "sound: ", Style::new().fg(Color::Red));
        p.text(label_end, y, p.room(label_end), problem, Style::new());
    }
    if let (Some(y), Some(notice)) = (notice_y, &app.notice) {
        p.notice(
            y,
            notice.kind,
            &notice.text,
            now.saturating_duration_since(notice.shown),
        );
    }
    if let Some(y) = hints_y {
        p.hints(y, hints, first_start);
        if area.width >= WIDE {
            p.corner(y, view.fps());
        }
    }
}

fn home(p: &mut Painter<'_>, app: &App, item: usize, end: u16) {
    let x = p.area.x + 1;
    let mut y = p.area.y + 2;
    let plain = Style::new();
    let quiet = Style::new().add_modifier(Modifier::DIM);
    for line in [
        "The game is not built yet.",
        "Setup sets how pictures are drawn and how loud sound plays.",
    ] {
        p.line(x, y, end, line, plain);
        y += 1;
    }
    y += 1;
    let label_w = 15;
    if y < end {
        p.text(x, y, label_w, "Settings file", quiet);
        p.text(
            x + label_w + 1,
            y,
            p.room(x + label_w + 1),
            &app.settings_path,
            plain,
        );
    }
    y += 1;
    if y < end {
        p.text(x, y, label_w, "Assets folder", quiet);
        p.text(
            x + label_w + 1,
            y,
            p.room(x + label_w + 1),
            &app.assets,
            plain,
        );
    }
    y += 1;
    if y < end {
        p.text(x, y, label_w, "Sound", quiet);
        audio_state(p, x + label_w + 1, y, app, "on", plain);
    }
    y += 2;
    for (n, entry) in HomeItem::ALL.iter().enumerate() {
        let (name, about) = match entry {
            HomeItem::Setup => ("Setup", "change drawing and sound settings"),
            HomeItem::Quit => ("Quit", "leave the program"),
        };
        if y >= end {
            break;
        }
        let focused = n == item;
        p.item(x, y, 10, name, focused);
        p.text(x + 12, y, p.room(x + 12), about, quiet);
        y += 1;
    }
}

fn setup_screen(
    p: &mut Painter<'_>,
    view: &mut View,
    app: &App,
    setup: &Setup,
    end: u16,
    now: Instant,
) {
    let x = p.area.x + 1;
    let mut y = p.area.y + 2;
    let intro = if setup.first_start {
        "First start: choose how the game draws and sounds; Save writes the file."
    } else {
        "Change how the game draws and sounds. Save writes the file; Esc discards."
    };
    p.line(x, y, end, intro, Style::new());
    y += 2;
    let below = setup_list(p, app, setup, x, y, end);
    // Previews draw after the list, so a long list text never covers them.
    if p.area.width >= WIDE {
        let px = x + LIST_W + 2;
        let panel = Rect::new(
            px,
            y,
            p.area.right().saturating_sub(px + 1),
            end.saturating_sub(y),
        );
        previews_beside(p, view, panel, &setup.draft, now);
    } else if let Some(y) = below
        && y < end
    {
        let panel = Rect::new(x, y, p.room(x).saturating_sub(1), end - y);
        previews_below(p, view, panel, &setup.draft, now);
    }
}

/// The settings list, buttons and help line from `y`; the row below them, or `None` when the
/// rows ran out.
fn setup_list(
    p: &mut Painter<'_>,
    app: &App,
    setup: &Setup,
    x: u16,
    mut y: u16,
    end: u16,
) -> Option<u16> {
    let saved = app.settings.as_ref();
    let rows_focused = setup.group == Group::Rows;
    let heading = |focused: bool| {
        if focused {
            Style::new().add_modifier(Modifier::BOLD)
        } else {
            Style::new().add_modifier(Modifier::DIM)
        }
    };
    p.line(x, y, end, "Settings", heading(rows_focused));
    y += 1;
    let label_w = 17;
    let value_x = x + label_w + 2;
    for (n, &row) in Row::ALL.iter().enumerate() {
        if y >= end {
            return None;
        }
        let focused = rows_focused && n == setup.row;
        p.item(x, y, label_w, label(row), focused);
        let after = value(p, value_x, y, app, &setup.draft, row);
        if let Some(saved) = saved
            && differs(saved, &setup.draft, row)
        {
            let mut s = Scratch::new();
            let _ = write!(s, "saved: ");
            write_value(&mut s, saved, row);
            p.text(
                after + 2,
                y,
                p.room(after + 2),
                s.as_str(),
                Style::new().add_modifier(Modifier::DIM),
            );
        }
        y += 1;
    }
    y += 1;
    if y >= end {
        return None;
    }
    let mut bx = x;
    for (n, &button) in setup.buttons().iter().enumerate() {
        let name = match button {
            Button::Save => "Save",
            Button::Cancel => "Cancel",
        };
        let focused = !rows_focused && n == setup.button;
        bx = p.item(bx, y, 8, name, focused) + 2;
    }
    y += 2;
    if y >= end {
        return None;
    }
    let help = if rows_focused {
        row_help(Row::ALL[setup.row])
    } else {
        "Save keeps these settings and opens the home screen."
    };
    p.line(x, y, end, help, Style::new().add_modifier(Modifier::ITALIC));
    Some(y + 2)
}

/// Width of the calibration panel for `rows` rows: the circle and square side by side.
fn calibration_width(rows: u16, draft: &Settings) -> u16 {
    (f32::from(rows) * draft.cell_aspect.get() * 2.6).round() as u16
}

/// The wide layout: calibration above, the place below, each under a label line.
fn previews_beside(
    p: &mut Painter<'_>,
    view: &mut View,
    panel: Rect,
    draft: &Settings,
    now: Instant,
) {
    if panel.height < 4 || panel.width < 8 {
        return;
    }
    let quiet = Style::new().add_modifier(Modifier::DIM);
    let cal_rows = (panel.height / 3).clamp(3, 12).min(panel.height - 3);
    let cal = Rect::new(
        panel.x,
        panel.y + 1,
        calibration_width(cal_rows, draft).min(panel.width),
        cal_rows,
    );
    p.text(
        panel.x,
        panel.y,
        panel.width,
        "Aspect check: round circle, square square?",
        quiet,
    );
    calibration(p, view, cal, draft);
    let place_y = cal.bottom() + 1;
    if place_y + 2 > panel.bottom() {
        return;
    }
    place_label(p, panel.x, place_y, panel.width, draft);
    let place = Rect::new(
        panel.x,
        place_y + 1,
        panel.width,
        panel.bottom() - place_y - 1,
    );
    place_preview(p, view, place, draft, now);
}

/// The narrow layout: one label line, then calibration and place side by side; a note when the
/// rows left are too few.
fn previews_below(
    p: &mut Painter<'_>,
    view: &mut View,
    panel: Rect,
    draft: &Settings,
    now: Instant,
) {
    let quiet = Style::new().add_modifier(Modifier::DIM);
    if panel.height < 4 || panel.width < 20 {
        p.text(
            panel.x,
            panel.y,
            panel.width,
            "Previews hidden: the window is too small.",
            quiet,
        );
        return;
    }
    let rows = panel.height - 1;
    let cal_w = calibration_width(rows, draft).min(panel.width / 3);
    p.text(panel.x, panel.y, cal_w + 1, "Aspect check", quiet);
    calibration(p, view, Rect::new(panel.x, panel.y + 1, cal_w, rows), draft);
    let px = panel.x + cal_w + 2;
    let pw = panel.right().saturating_sub(px);
    place_label(p, px, panel.y, pw, draft);
    place_preview(p, view, Rect::new(px, panel.y + 1, pw, rows), draft, now);
}

fn place_label(p: &mut Painter<'_>, x: u16, y: u16, width: u16, draft: &Settings) {
    let mut s = Scratch::new();
    let _ = write!(s, "Sample place, {} renderer", draft.renderer.name());
    p.text(
        x,
        y,
        width,
        s.as_str(),
        Style::new().add_modifier(Modifier::DIM),
    );
}

fn calibration(p: &mut Painter<'_>, view: &mut View, area: Rect, draft: &Settings) {
    if let Err(error) = view.draw_calibration(area, draft.cell_aspect, draft.renderer, p.buf) {
        p.error_in(area, &error);
    }
}

fn place_preview(p: &mut Painter<'_>, view: &mut View, area: Rect, draft: &Settings, now: Instant) {
    if let Err(error) = view.draw_place(area, draft.cell_aspect, draft.renderer, now, p.buf) {
        p.error_in(area, &error);
    }
}

fn label(row: Row) -> &'static str {
    match row {
        Row::CellAspect => "Cell aspect",
        Row::Renderer => "Renderer",
        Row::Master => "Master volume",
        Row::Music => "Music volume",
        Row::Effects => "Effects volume",
        Row::Muted => "Sound",
        Row::TestSound => "Test sound",
        Row::Background => "Background test",
    }
}

fn row_help(row: Row) -> &'static str {
    match row {
        Row::CellAspect => "Cell height over width. ←→ 0.05, Shift+←→ 0.01.",
        Row::Renderer => "Octant if the preview shows fine blocks, else quadrant.",
        Row::Master => "Scales all sound. ←→ step 5, Shift+←→ 1.",
        Row::Music => "Background music level. ←→ step 5, Shift+←→ 1.",
        Row::Effects => "Sound effect level. ←→ step 5, Shift+←→ 1.",
        Row::Muted => "Enter or ←→ switches mute; volumes are kept.",
        Row::TestSound => "Enter plays a short interface sound to check the device.",
        Row::Background => "Enter: off, ambience, music, off; leaving setup stops it.",
    }
}

fn differs(a: &Settings, b: &Settings, row: Row) -> bool {
    match row {
        Row::CellAspect => a.cell_aspect != b.cell_aspect,
        Row::Renderer => a.renderer != b.renderer,
        Row::Master => a.audio.master != b.audio.master,
        Row::Music => a.audio.music != b.audio.music,
        Row::Effects => a.audio.effects != b.audio.effects,
        Row::Muted => a.audio.muted != b.audio.muted,
        Row::TestSound | Row::Background => false,
    }
}

fn write_value(s: &mut Scratch, settings: &Settings, row: Row) {
    let _ = match row {
        Row::CellAspect => write!(s, "{}", settings.cell_aspect),
        Row::Renderer => write!(s, "{}", settings.renderer.name()),
        Row::Master => write!(s, "{}%", settings.audio.master.percent()),
        Row::Music => write!(s, "{}%", settings.audio.music.percent()),
        Row::Effects => write!(s, "{}%", settings.audio.effects.percent()),
        Row::Muted => write!(s, "{}", if settings.audio.muted { "muted" } else { "on" }),
        Row::TestSound | Row::Background => Ok(()),
    };
}

/// Draws `row`'s value of `draft` at `x`; returns the column after it.
fn value(p: &mut Painter<'_>, x: u16, y: u16, app: &App, draft: &Settings, row: Row) -> u16 {
    let style = Style::new();
    let volume = match row {
        Row::Master => Some(draft.audio.master),
        Row::Music => Some(draft.audio.music),
        Row::Effects => Some(draft.audio.effects),
        Row::TestSound => return audio_state(p, x, y, app, "Enter plays it", style),
        Row::Background => {
            let state = match app.background {
                Background::Off => "off",
                Background::Ambience => "ambience (room tone)",
                Background::Music => "music (drone)",
            };
            return audio_state(p, x, y, app, state, style);
        }
        _ => None,
    };
    let mut at = x;
    if let Some(volume) = volume {
        at = p.gauge(at, y, 10, volume);
        at += 1;
    }
    let mut s = Scratch::new();
    write_value(&mut s, draft, row);
    if row == Row::CellAspect {
        let _ = write!(s, " (1.50 to 3.00)");
    }
    p.text(at, y, p.room(at), s.as_str(), style)
}

/// `on` while audio plays, else `audio off: <reason>`, at `x`; returns the column after it.
fn audio_state(p: &mut Painter<'_>, x: u16, y: u16, app: &App, on: &str, style: Style) -> u16 {
    match &app.audio_off {
        Some(reason) => {
            let at = p.text(x, y, p.room(x), "audio off: ", style);
            p.text(at, y, p.room(at), reason, style)
        }
        None => p.text(x, y, p.room(x), on, style),
    }
}

/// Writes into the buffer's cells inside `area`; every write is cropped to it.
struct Painter<'a> {
    buf: &'a mut Buffer,
    area: Rect,
}

impl Painter<'_> {
    fn clear(&mut self) {
        for y in self.area.top()..self.area.bottom() {
            for x in self.area.left()..self.area.right() {
                self.put(x, y, ' ', Style::reset());
            }
        }
    }

    /// Columns left from `x` to the area's right edge.
    fn room(&self, x: u16) -> u16 {
        self.area.right().saturating_sub(x)
    }

    fn put(&mut self, x: u16, y: u16, ch: char, style: Style) {
        if x < self.area.right()
            && y < self.area.bottom()
            && let Some(cell) = self.buf.cell_mut((x, y))
        {
            cell.set_char(ch).set_style(style);
        }
    }

    /// Writes `text` at `x`, `y` in at most `width` cells, ending a cut text with `…`. Returns the
    /// column after the last written cell.
    fn text(&mut self, x: u16, y: u16, width: u16, text: &str, style: Style) -> u16 {
        let width = width.min(self.room(x));
        if width == 0 || y >= self.area.bottom() {
            return x;
        }
        let fits = text.chars().count() <= usize::from(width);
        let mut at = x;
        for (n, ch) in text.chars().enumerate() {
            let last = n + 1 == usize::from(width);
            if n == usize::from(width) {
                break;
            }
            self.put(at, y, if last && !fits { '…' } else { ch }, style);
            at += 1;
        }
        at
    }

    /// A text line from `x` to the right edge, when `y` is above `end`.
    fn line(&mut self, x: u16, y: u16, end: u16, text: &str, style: Style) {
        if y < end {
            self.text(x, y, self.room(x), text, style);
        }
    }

    /// A focusable item: `▶ name` reversed when focused, `  name` otherwise, padded to `width`.
    /// Returns the column after it.
    fn item(&mut self, x: u16, y: u16, width: u16, name: &str, focused: bool) -> u16 {
        let style = if focused {
            Style::new().add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            Style::new()
        };
        self.put(x, y, if focused { '▶' } else { ' ' }, style);
        self.put(x + 1, y, ' ', style);
        let end = x + 2 + width;
        let mut at = self.text(x + 2, y, width, name, style);
        while at < end && at < self.area.right() {
            self.put(at, y, ' ', style);
            at += 1;
        }
        end.min(self.area.right())
    }

    /// A bar of `width` cells filled to `volume`. Returns the column after it.
    fn gauge(&mut self, x: u16, y: u16, width: u16, volume: Volume) -> u16 {
        let filled = (u32::from(width) * u32::from(volume.percent()) + 50) / 100;
        for c in 0..width {
            let ch = if u32::from(c) < filled { '█' } else { '░' };
            self.put(x + c, y, ch, Style::new());
        }
        (x + width).min(self.area.right())
    }

    fn title_bar(&mut self, screen: &str) {
        let y = self.area.y;
        if self.area.height == 0 {
            return;
        }
        let style = Style::new().add_modifier(Modifier::REVERSED | Modifier::BOLD);
        for x in self.area.left()..self.area.right() {
            self.put(x, y, ' ', style);
        }
        let mut at = self.text(self.area.x + 1, y, self.room(self.area.x + 1), TITLE, style);
        at = self.text(at, y, self.room(at), " │ ", style);
        self.text(at, y, self.room(at), screen, style);
    }

    /// The notice text with a bar draining over its remaining time.
    fn notice(&mut self, y: u16, kind: NoticeKind, text: &str, age: std::time::Duration) {
        let style = match kind {
            NoticeKind::Info => Style::new(),
            NoticeKind::Error => Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
        };
        let bar_w: u16 = 8;
        let x = self.area.x + 1;
        let left = 1.0 - (age.as_secs_f32() / NOTICE.as_secs_f32()).min(1.0);
        let filled = (f32::from(bar_w) * left).ceil() as u16;
        if self.room(x) > bar_w + 2 {
            for c in 0..bar_w {
                self.put(
                    x + c,
                    y,
                    if c < filled { '▬' } else { ' ' },
                    Style::new().add_modifier(Modifier::DIM),
                );
            }
            let tx = x + bar_w + 1;
            let prefix = if kind == NoticeKind::Error { "! " } else { "" };
            let at = self.text(tx, y, self.room(tx), prefix, style);
            self.text(at, y, self.room(at), text, style);
        } else {
            self.text(x, y, self.room(x), text, style);
        }
    }

    /// Version, commit and frames in the last whole second, right-aligned on row `y`, with a
    /// blank cell before it so it never runs into the hints.
    fn corner(&mut self, y: u16, fps: u32) {
        let mut s = Scratch::new();
        let _ = write!(s, " {BUILD} · {fps} fps");
        let width = s.as_str().chars().count() as u16;
        let x = self.area.right().saturating_sub(width + 1).max(self.area.x);
        self.text(
            x,
            y,
            width,
            s.as_str(),
            Style::new().add_modifier(Modifier::DIM),
        );
    }

    /// A preview's error in its first row.
    fn error_in(&mut self, area: Rect, error: &dyn std::fmt::Display) {
        let mut s = Scratch::new();
        let _ = write!(s, "{error}");
        let style = Style::new().fg(Color::Red);
        self.text(area.x, area.y, area.width, s.as_str(), style);
    }

    /// Key hints: each hint's caps, then its words, while they fit. A first start shows no Esc.
    fn hints(&mut self, y: u16, hints: &[Hint], first_start: bool) {
        let cap_style = Style::new().add_modifier(Modifier::BOLD);
        let word_style = Style::new().add_modifier(Modifier::DIM);
        let mut at = self.area.x + 1;
        for &(intents, words) in hints {
            if first_start && intents.contains(&Intent::Back) {
                continue;
            }
            // Caps that share a modifier prefix show it once ("Shift+←→"); single-character
            // caps run together; other caps are joined with `/`.
            let first = intents.first().map_or("", |&i| cap(i));
            let prefix = first.rfind('+').map_or("", |at| &first[..=at]);
            let shared = intents.iter().all(|&i| cap(i).starts_with(prefix));
            let part = |n: usize, intent: Intent| {
                let cap = cap(intent);
                if n > 0 && shared {
                    &cap[prefix.len()..]
                } else {
                    cap
                }
            };
            let joined = intents
                .iter()
                .enumerate()
                .any(|(n, &i)| n > 0 && part(n, i).chars().count() > 1);
            let caps = intents
                .iter()
                .enumerate()
                .map(|(n, &i)| part(n, i).chars().count())
                .sum::<usize>();
            let need = caps + usize::from(joined) * (intents.len() - 1) + 1 + words.chars().count();
            if usize::from(self.room(at)) < need {
                self.text(at, y, self.room(at), "…", word_style);
                return;
            }
            for (n, &intent) in intents.iter().enumerate() {
                if joined && n > 0 {
                    at = self.text(at, y, self.room(at), "/", word_style);
                }
                at = self.text(at, y, self.room(at), part(n, intent), cap_style);
            }
            at = self.text(at + 1, y, self.room(at + 1), words, word_style);
            at += 2;
        }
    }
}

/// A small text buffer on the stack, for numbers; text past its end is dropped.
struct Scratch {
    bytes: [u8; 64],
    len: usize,
}

impl Scratch {
    fn new() -> Scratch {
        Scratch {
            bytes: [0; 64],
            len: 0,
        }
    }

    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

impl std::fmt::Write for Scratch {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        // Whole characters only, so the text stays valid UTF-8.
        for ch in s.chars() {
            let n = ch.len_utf8();
            if self.len + n > self.bytes.len() {
                return Err(std::fmt::Error);
            }
            ch.encode_utf8(&mut self.bytes[self.len..]);
            self.len += n;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::keys::Intent;

    fn rows(app: &App, view: &mut View, width: u16, height: u16) -> Vec<String> {
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        draw(app, view, area, &mut buf, Instant::now());
        (0..height)
            .map(|y| (0..width).map(|x| buf[(x, y)].symbol()).collect())
            .collect()
    }

    fn first_start() -> App {
        App::open(std::env::temp_dir().join("prototype-ui-unused"), None, None)
    }

    #[test]
    fn every_size_draws_without_panic() {
        let mut view = crate::preview::tests::small_view();
        let mut app = first_start();
        // Tiny sizes, the narrow layout and the wide one (previews beside the list).
        for (w, h) in [(1, 1), (2, 2), (5, 3), (20, 6), (80, 24), (100, 30)] {
            rows(&app, &mut view, w, h);
        }
        app.intent(Intent::NextFocus, Instant::now());
        for (w, h) in [(1, 1), (80, 24)] {
            rows(&app, &mut view, w, h);
        }
    }

    #[test]
    fn wide_layout_shows_the_corner() {
        let mut view = crate::preview::tests::small_view();
        let frame = rows(&first_start(), &mut view, 100, 30);
        assert!(frame[29].trim_end().ends_with("0 fps"), "{}", frame[29]);
        assert!(frame[29].contains(BUILD), "{}", frame[29]);
        let narrow = rows(&first_start(), &mut view, 99, 30);
        assert!(!narrow[29].contains("fps"), "{}", narrow[29]);
    }

    #[test]
    fn cursor_is_marked_and_hints_come_from_the_key_table() {
        let app = first_start();
        let frame = rows(&app, &mut crate::preview::tests::small_view(), 80, 24);
        assert!(
            frame.iter().any(|r| r.contains("▶ Cell aspect")),
            "{frame:#?}"
        );
        let hints = &frame[23];
        assert!(hints.contains(cap(Intent::Quit)), "{hints}");
        assert!(
            !hints.contains(cap(Intent::Back)),
            "no Esc on a first start: {hints}"
        );
    }

    #[test]
    fn cut_text_ends_with_an_ellipsis() {
        let area = Rect::new(0, 0, 6, 1);
        let mut buf = Buffer::empty(area);
        let mut p = Painter {
            buf: &mut buf,
            area,
        };
        assert_eq!(p.text(0, 0, 6, "abcdefgh", Style::new()), 6);
        let row: String = (0..6).map(|x| buf[(x, 0)].symbol()).collect();
        assert_eq!(row, "abcde…");
    }

    #[test]
    fn audio_off_and_the_problem_strip_show_in_one_line() {
        let mut view = crate::preview::tests::small_view();
        let mut app = App::open(
            std::env::temp_dir().join("prototype-ui-unused"),
            None,
            Some(Settings::default()),
        );
        app.audio_off = Some("no output device".to_string());
        app.sound_problem = Some("sounds.toml:3: bus: unknown key".to_string());
        let home = rows(&app, &mut view, 80, 24);
        assert!(
            home.iter()
                .any(|r| r.contains("audio off: no output device")),
            "{home:#?}"
        );
        assert!(
            home[22].contains("sound: sounds.toml:3: bus: unknown key"),
            "{home:#?}"
        );
        app.intent(Intent::Activate, Instant::now());
        let setup = rows(&app, &mut view, 80, 24);
        let checks: Vec<_> = setup
            .iter()
            .filter(|r| r.contains("audio off: no output device"))
            .collect();
        assert_eq!(checks.len(), 2, "both check rows: {setup:#?}");
        assert!(setup[22].starts_with(" sound: "), "{setup:#?}");
    }
}
