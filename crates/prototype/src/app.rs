// ADR: `App` holds the screen state and turns intents into changes; it never sees crossterm events or draws. The clock is passed in, so tests and headless runs use set instants.
// ADR: The setup screen edits a draft; only its Save writes the file, so moving the cursor or changing a value commits nothing. A first start (no settings file) opens setup and has no Cancel; Esc there does nothing.
// ADR: A notice shows for `NOTICE` with a draining bar of its remaining time; it is the only animation, so the loop polls at frame rate only while one shows.
// ADR: Audio settings apply live: each change of a volume or mute in the draft requests that mix; leaving setup without Save requests the saved one again. Leaving setup turns the background test off. Only the two check rows request sounds; moving the cursor never does.
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::keys::Intent;
use crate::settings::{self, CellAspect, Settings, Volume};
use crate::sound::{Background, Request};

/// How long a notice stays.
pub const NOTICE: Duration = Duration::from_secs(3);

/// The setup screen's settings rows, in list order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    CellAspect,
    Renderer,
    Master,
    Music,
    Effects,
    Muted,
    /// Enter plays the test cue.
    TestSound,
    /// Enter cycles the background test layer.
    Background,
}

impl Row {
    pub const ALL: [Row; 8] = [
        Row::CellAspect,
        Row::Renderer,
        Row::Master,
        Row::Music,
        Row::Effects,
        Row::Muted,
        Row::TestSound,
        Row::Background,
    ];
}

/// The setup screen's buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Save,
    Cancel,
}

/// Which group of the setup screen holds the cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Rows,
    Buttons,
}

/// The home screen's menu items, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HomeItem {
    Setup,
    Quit,
}

impl HomeItem {
    pub const ALL: [HomeItem; 2] = [HomeItem::Setup, HomeItem::Quit];
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Setup {
    /// No settings file existed: Save is the only way on.
    pub first_start: bool,
    pub draft: Settings,
    pub group: Group,
    pub row: usize,
    pub button: usize,
}

impl Setup {
    fn new(first_start: bool, draft: Settings) -> Setup {
        Setup {
            first_start,
            draft,
            group: Group::Rows,
            row: 0,
            button: 0,
        }
    }

    /// The buttons this setup shows: Save, and Cancel unless it is the first start.
    pub fn buttons(&self) -> &'static [Button] {
        if self.first_start {
            &[Button::Save]
        } else {
            &[Button::Save, Button::Cancel]
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Screen {
    Home { item: usize },
    Setup(Setup),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    Info,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub kind: NoticeKind,
    pub text: String,
    pub shown: Instant,
}

pub struct App {
    /// The data folder settings are written to.
    pub data: PathBuf,
    /// The settings file's path, for display.
    pub settings_path: String,
    /// The assets folder for display, or why there is none.
    pub assets: String,
    /// The saved settings; `None` until the first save.
    pub settings: Option<Settings>,
    pub screen: Screen,
    pub notice: Option<Notice>,
    pub quit: bool,
    /// Why there is no sound, when there is none.
    pub audio_off: Option<String>,
    /// The sound problem strip's line (a bad manifest edit, a device or log failure).
    pub sound_problem: Option<String>,
    /// The background test layer's state.
    pub background: Background,
    /// Sound requests not yet handed to the sound host; the loop takes them after a draw.
    pub requests: Vec<Request>,
}

impl App {
    /// The app on `data` with the settings read from there (`None`: a first start).
    pub fn open(data: PathBuf, assets: Option<PathBuf>, settings: Option<Settings>) -> App {
        let screen = match settings {
            Some(_) => Screen::Home { item: 0 },
            None => Screen::Setup(Setup::new(true, Settings::default())),
        };
        App {
            settings_path: settings::path(&data).display().to_string(),
            assets: assets.map_or_else(
                || "none found (pass --assets <dir>)".to_string(),
                |dir| dir.display().to_string(),
            ),
            data,
            settings,
            screen,
            notice: None,
            quit: false,
            audio_off: None,
            sound_problem: None,
            background: Background::Off,
            requests: Vec::new(),
        }
    }

    /// Whether something moves on screen, so the loop draws every frame.
    pub fn animating(&self) -> bool {
        self.notice.is_some()
    }

    /// Advances clock-driven state to `now`; `true` when the screen changed.
    pub fn step(&mut self, now: Instant) -> bool {
        let expired = self
            .notice
            .as_ref()
            .is_some_and(|notice| now.saturating_duration_since(notice.shown) >= NOTICE);
        if expired {
            self.notice = None;
        }
        expired
    }

    pub fn intent(&mut self, intent: Intent, now: Instant) {
        if intent == Intent::Quit {
            self.quit = true;
            return;
        }
        match &mut self.screen {
            Screen::Home { item } => match intent {
                Intent::Up => *item = item.saturating_sub(1),
                Intent::Down => *item = (*item + 1).min(HomeItem::ALL.len() - 1),
                Intent::Activate => match HomeItem::ALL[*item] {
                    HomeItem::Setup => {
                        let draft = self.settings.unwrap_or_default();
                        self.screen = Screen::Setup(Setup::new(false, draft));
                    }
                    HomeItem::Quit => self.quit = true,
                },
                _ => {}
            },
            Screen::Setup(setup) => {
                match setup_intent(setup, intent, &mut self.background, &mut self.requests) {
                    SetupStep::Stay => {}
                    SetupStep::Save(draft) => self.save(draft, now),
                    SetupStep::Close => {
                        let saved = self.settings.unwrap_or_default();
                        let changed = self.settings.is_some_and(|saved| saved != setup.draft);
                        if saved.audio != setup.draft.audio {
                            self.requests.push(Request::Mix(saved.audio));
                        }
                        self.leave_setup();
                        if changed {
                            self.notify(NoticeKind::Info, "Changes discarded.".to_string(), now);
                        }
                    }
                }
            }
        }
    }

    /// Opens home and turns the background test off.
    fn leave_setup(&mut self) {
        self.screen = Screen::Home { item: 0 };
        if self.background != Background::Off {
            self.background = Background::Off;
            self.requests.push(Request::Background(Background::Off));
        }
    }

    fn save(&mut self, draft: Settings, now: Instant) {
        match settings::save(&self.data, &draft) {
            Ok(()) => {
                self.settings = Some(draft);
                self.leave_setup();
                self.notify(NoticeKind::Info, "Settings saved.".to_string(), now);
            }
            Err(error) => self.notify(NoticeKind::Error, format!("Not saved: {error}"), now),
        }
    }

    fn notify(&mut self, kind: NoticeKind, text: String, now: Instant) {
        self.notice = Some(Notice {
            kind,
            text,
            shown: now,
        });
    }
}

enum SetupStep {
    Stay,
    Save(Settings),
    Close,
}

fn setup_intent(
    setup: &mut Setup,
    intent: Intent,
    background: &mut Background,
    requests: &mut Vec<Request>,
) -> SetupStep {
    let buttons = setup.buttons();
    let audio = setup.draft.audio;
    let step = match (setup.group, intent) {
        (_, Intent::Back) if setup.first_start => SetupStep::Stay,
        (_, Intent::Back) => SetupStep::Close,
        (_, Intent::NextFocus | Intent::PrevFocus) => {
            setup.group = match setup.group {
                Group::Rows => Group::Buttons,
                Group::Buttons => Group::Rows,
            };
            SetupStep::Stay
        }
        (Group::Rows, Intent::Up) => {
            setup.row = setup.row.saturating_sub(1);
            SetupStep::Stay
        }
        (Group::Rows, Intent::Down) => {
            if setup.row + 1 < Row::ALL.len() {
                setup.row += 1;
            } else {
                setup.group = Group::Buttons;
            }
            SetupStep::Stay
        }
        (Group::Rows, Intent::Left | Intent::Right | Intent::FineLeft | Intent::FineRight) => {
            let up = matches!(intent, Intent::Right | Intent::FineRight);
            let fine = matches!(intent, Intent::FineLeft | Intent::FineRight);
            change(&mut setup.draft, Row::ALL[setup.row], up, fine);
            SetupStep::Stay
        }
        (Group::Rows, Intent::Activate) => {
            match Row::ALL[setup.row] {
                Row::Renderer => setup.draft.renderer = setup.draft.renderer.cycle(true),
                Row::Muted => setup.draft.audio.muted = !setup.draft.audio.muted,
                Row::TestSound => requests.push(Request::Test),
                Row::Background => {
                    *background = background.next();
                    requests.push(Request::Background(*background));
                }
                _ => {}
            }
            SetupStep::Stay
        }
        (Group::Buttons, Intent::Up) => {
            setup.group = Group::Rows;
            SetupStep::Stay
        }
        (Group::Buttons, Intent::Left) => {
            setup.button = setup.button.saturating_sub(1);
            SetupStep::Stay
        }
        (Group::Buttons, Intent::Right) => {
            setup.button = (setup.button + 1).min(buttons.len() - 1);
            SetupStep::Stay
        }
        (Group::Buttons, Intent::Activate) => match buttons[setup.button.min(buttons.len() - 1)] {
            Button::Save => SetupStep::Save(setup.draft),
            Button::Cancel => SetupStep::Close,
        },
        _ => SetupStep::Stay,
    };
    if setup.draft.audio != audio {
        requests.push(Request::Mix(setup.draft.audio));
    }
    step
}

/// Changes `row` of `draft` one step up (`up`) or down; `fine` takes the small step.
fn change(draft: &mut Settings, row: Row, up: bool, fine: bool) {
    let volume = |v: Volume| {
        v.step(
            up,
            if fine {
                Volume::FINE_STEP
            } else {
                Volume::STEP
            },
        )
    };
    match row {
        Row::CellAspect => {
            let by = if fine {
                CellAspect::FINE_STEP
            } else {
                CellAspect::STEP
            };
            draft.cell_aspect = draft.cell_aspect.step(up, by);
        }
        Row::Renderer => draft.renderer = draft.renderer.cycle(up),
        Row::Master => draft.audio.master = volume(draft.audio.master),
        Row::Music => draft.audio.music = volume(draft.audio.music),
        Row::Effects => draft.audio.effects = volume(draft.audio.effects),
        Row::Muted => draft.audio.muted = !draft.audio.muted,
        Row::TestSound | Row::Background => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("prototype-app-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn setup(app: &App) -> &Setup {
        match &app.screen {
            Screen::Setup(setup) => setup,
            other => panic!("expected setup, got {other:?}"),
        }
    }

    #[test]
    fn first_start_edits_a_draft_and_writes_only_on_save() {
        let dir = scratch("first");
        let now = Instant::now();
        let mut app = App::open(dir.clone(), None, None);
        assert!(setup(&app).first_start);
        for intent in [
            Intent::Right,
            Intent::FineRight,
            Intent::Down,
            Intent::Activate,
            Intent::Back,
        ] {
            app.intent(intent, now);
        }
        // Esc on a first start does nothing; nothing is written before Save.
        assert_eq!(setup(&app).draft.cell_aspect.hundredths(), 206);
        assert!(!settings::path(&dir).exists());
        app.intent(Intent::NextFocus, now);
        app.intent(Intent::Right, now);
        assert_eq!(setup(&app).button, 0, "a first start has only Save");
        app.intent(Intent::Activate, now);
        assert_eq!(app.screen, Screen::Home { item: 0 });
        let saved = settings::load(&dir).expect("reads").expect("written");
        assert_eq!(saved.cell_aspect.hundredths(), 206);
        assert_eq!(saved.renderer, settings::Renderer::Quadrant);
        assert_eq!(app.settings, Some(saved));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn setup_from_home_cancels_with_esc_and_keeps_the_file() {
        let dir = scratch("later");
        let saved = Settings::default();
        settings::save(&dir, &saved).expect("saves");
        let before = std::fs::read(settings::path(&dir)).expect("bytes");
        let now = Instant::now();
        let mut app = App::open(dir.clone(), None, Some(saved));
        app.intent(Intent::Back, now);
        assert!(!app.quit, "Esc never quits");
        app.intent(Intent::Activate, now);
        assert!(!setup(&app).first_start);
        app.intent(Intent::Down, now);
        app.intent(Intent::Left, now);
        assert_eq!(setup(&app).draft.renderer, settings::Renderer::Ascii);
        app.intent(Intent::Back, now);
        assert_eq!(app.screen, Screen::Home { item: 0 });
        assert_eq!(app.settings, Some(saved));
        assert!(app.notice.is_some());
        assert_eq!(std::fs::read(settings::path(&dir)).expect("bytes"), before);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn notice_expires_on_the_clock() {
        let dir = scratch("notice");
        let now = Instant::now();
        let mut app = App::open(dir, None, Some(Settings::default()));
        app.notify(NoticeKind::Info, "x".to_string(), now);
        assert!(app.animating());
        assert!(!app.step(now + NOTICE - Duration::from_millis(1)));
        assert!(app.step(now + NOTICE));
        assert!(!app.animating());
    }

    #[test]
    fn home_menu_quits_and_ctrl_c_quits_anywhere() {
        let now = Instant::now();
        let mut app = App::open(scratch("quit"), None, Some(Settings::default()));
        app.intent(Intent::Down, now);
        app.intent(Intent::Down, now);
        app.intent(Intent::Activate, now);
        assert!(app.quit);
        let mut app = App::open(scratch("quit2"), None, None);
        app.intent(Intent::Quit, now);
        assert!(app.quit);
    }

    /// Opens setup from home on saved defaults and moves the cursor to `row`.
    fn setup_at(row: Row) -> App {
        let now = Instant::now();
        let mut app = App::open(scratch("audio"), None, Some(Settings::default()));
        app.intent(Intent::Activate, now);
        while Row::ALL[setup(&app).row] != row {
            app.intent(Intent::Down, now);
        }
        app
    }

    #[test]
    fn navigation_requests_no_sound_and_check_rows_do() {
        let now = Instant::now();
        let mut app = setup_at(Row::Background);
        app.intent(Intent::Up, now);
        app.intent(Intent::Left, now);
        assert_eq!(
            app.requests,
            [],
            "moving and the check rows' arrows are silent"
        );
        app.intent(Intent::Activate, now);
        app.intent(Intent::Down, now);
        for _ in 0..3 {
            app.intent(Intent::Activate, now);
        }
        assert_eq!(
            app.requests,
            [
                Request::Test,
                Request::Background(Background::Ambience),
                Request::Background(Background::Music),
                Request::Background(Background::Off),
            ]
        );
        assert_eq!(app.background, Background::Off);
    }

    #[test]
    fn volume_changes_apply_live_and_esc_restores_the_saved_mix() {
        let now = Instant::now();
        let saved = Settings::default().audio;
        let mut app = setup_at(Row::Music);
        app.intent(Intent::Left, now);
        let live = setup(&app).draft.audio;
        assert_eq!(live.music.percent(), saved.music.percent() - Volume::STEP);
        // The background test plays; Esc stops it and restores the saved volumes.
        app.intent(Intent::Down, now);
        app.intent(Intent::Down, now);
        app.intent(Intent::Down, now);
        app.intent(Intent::Down, now);
        app.intent(Intent::Activate, now);
        app.intent(Intent::Back, now);
        assert_eq!(
            app.requests,
            [
                Request::Mix(live),
                Request::Background(Background::Ambience),
                Request::Mix(saved),
                Request::Background(Background::Off),
            ]
        );
        assert_eq!(app.background, Background::Off);
    }

    #[test]
    fn save_keeps_the_live_mix() {
        let dir = scratch("save-mix");
        let now = Instant::now();
        let mut app = App::open(dir.clone(), None, None);
        app.intent(Intent::Down, now);
        app.intent(Intent::Down, now);
        app.intent(Intent::Right, now);
        app.intent(Intent::NextFocus, now);
        app.intent(Intent::Activate, now);
        let live = app.settings.expect("saved").audio;
        assert_eq!(
            app.requests,
            [Request::Mix(live)],
            "Save requests nothing more"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
