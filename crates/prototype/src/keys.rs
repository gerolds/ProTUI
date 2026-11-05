// ADR: One table maps key presses to generic intents and gives every on-screen hint its key cap; screens see intents, never crossterm events.
// ADR: Keys act on press only (the Windows console also reports releases); letters match either case; Shift with ← → sends the fine intents; Esc means back, Ctrl+C quits.
// ADR: A press the table does not name sends `Intent::Other`, so "any key" means any key.
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// What the player asks for, whatever the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    Up,
    Down,
    Left,
    Right,
    /// Shift with ←: a smaller step of the value under the cursor.
    FineLeft,
    /// Shift with →: a smaller step of the value under the cursor.
    FineRight,
    /// Move focus to the next group of the screen.
    NextFocus,
    /// Move focus to the previous group of the screen.
    PrevFocus,
    /// Activate the item under the cursor.
    Activate,
    /// Go back or cancel; never quits.
    Back,
    /// Leave the program from any screen.
    Quit,
    /// A key the table does not name.
    Other,
}

/// One row of the key table: the intent, the cap screens show, and the key with its modifiers.
struct Binding {
    intent: Intent,
    cap: &'static str,
    code: KeyCode,
    modifiers: KeyModifiers,
}

const fn bind(
    intent: Intent,
    cap: &'static str,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Binding {
    Binding {
        intent,
        cap,
        code,
        modifiers,
    }
}

/// The one key table. `Intent::Other` has no row: it is every key the table does not name.
const KEYS: &[Binding] = &[
    bind(Intent::Up, "↑", KeyCode::Up, KeyModifiers::NONE),
    bind(Intent::Down, "↓", KeyCode::Down, KeyModifiers::NONE),
    bind(Intent::Left, "←", KeyCode::Left, KeyModifiers::NONE),
    bind(Intent::Right, "→", KeyCode::Right, KeyModifiers::NONE),
    bind(
        Intent::FineLeft,
        "Shift+←",
        KeyCode::Left,
        KeyModifiers::SHIFT,
    ),
    bind(
        Intent::FineRight,
        "Shift+→",
        KeyCode::Right,
        KeyModifiers::SHIFT,
    ),
    bind(Intent::NextFocus, "Tab", KeyCode::Tab, KeyModifiers::NONE),
    // Terminals send Shift+Tab as BackTab, some with SHIFT set; `normalize` drops it.
    bind(
        Intent::PrevFocus,
        "Shift+Tab",
        KeyCode::BackTab,
        KeyModifiers::NONE,
    ),
    bind(
        Intent::Activate,
        "Enter",
        KeyCode::Enter,
        KeyModifiers::NONE,
    ),
    bind(Intent::Back, "Esc", KeyCode::Esc, KeyModifiers::NONE),
    bind(
        Intent::Quit,
        "Ctrl+C",
        KeyCode::Char('c'),
        KeyModifiers::CONTROL,
    ),
];

/// The modifiers the table distinguishes; others (Super, Hyper, Meta) are ignored.
const MATTERS: KeyModifiers = KeyModifiers::SHIFT
    .union(KeyModifiers::CONTROL)
    .union(KeyModifiers::ALT);

/// The intent a key press sends; `None` for a release or a repeat.
pub fn intent(key: KeyEvent) -> Option<Intent> {
    if key.kind != KeyEventKind::Press {
        return None;
    }
    let (code, modifiers) = normalize(key.code, key.modifiers & MATTERS);
    let named = KEYS
        .iter()
        .find(|b| b.code == code && b.modifiers == modifiers)
        .map(|b| b.intent);
    Some(named.unwrap_or(Intent::Other))
}

/// Letters in lower case without Shift (Caps Lock and Shift do not change a letter's intent);
/// BackTab without Shift (it already means Shift+Tab).
fn normalize(code: KeyCode, modifiers: KeyModifiers) -> (KeyCode, KeyModifiers) {
    match code {
        KeyCode::Char(c) => (
            KeyCode::Char(c.to_ascii_lowercase()),
            modifiers - KeyModifiers::SHIFT,
        ),
        KeyCode::BackTab => (code, modifiers - KeyModifiers::SHIFT),
        _ => (code, modifiers),
    }
}

/// The cap of the key that sends `intent`, for key hints.
pub fn cap(intent: Intent) -> &'static str {
    if intent == Intent::Other {
        return "any key";
    }
    KEYS.iter()
        .find(|b| b.intent == intent)
        .map_or("?", |b| b.cap)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new_with_kind(code, modifiers, KeyEventKind::Press)
    }

    #[test]
    fn every_row_sends_its_intent_and_has_a_cap() {
        for binding in KEYS {
            assert_eq!(
                intent(press(binding.code, binding.modifiers)),
                Some(binding.intent)
            );
            assert_eq!(cap(binding.intent), binding.cap);
        }
        assert_eq!(cap(Intent::Other), "any key");
    }

    #[test]
    fn only_presses_act() {
        for kind in [KeyEventKind::Release, KeyEventKind::Repeat] {
            let key = KeyEvent::new_with_kind(KeyCode::Enter, KeyModifiers::NONE, kind);
            assert_eq!(intent(key), None);
        }
    }

    #[test]
    fn letters_match_either_case() {
        for code in [KeyCode::Char('c'), KeyCode::Char('C')] {
            for shift in [KeyModifiers::NONE, KeyModifiers::SHIFT] {
                assert_eq!(
                    intent(press(code, KeyModifiers::CONTROL | shift)),
                    Some(Intent::Quit)
                );
            }
        }
        // Plain c is no quit: only Ctrl+C is.
        assert_eq!(
            intent(press(KeyCode::Char('c'), KeyModifiers::NONE)),
            Some(Intent::Other)
        );
    }

    #[test]
    fn shift_arrows_send_fine_intents() {
        assert_eq!(
            intent(press(KeyCode::Left, KeyModifiers::SHIFT)),
            Some(Intent::FineLeft)
        );
        assert_eq!(
            intent(press(KeyCode::Right, KeyModifiers::SHIFT)),
            Some(Intent::FineRight)
        );
        assert_eq!(
            intent(press(KeyCode::Left, KeyModifiers::NONE)),
            Some(Intent::Left)
        );
        // Other modifiers on arrows are not the fine step.
        assert_eq!(
            intent(press(KeyCode::Left, KeyModifiers::CONTROL)),
            Some(Intent::Other)
        );
    }

    #[test]
    fn back_tab_is_prev_focus_with_or_without_shift() {
        for shift in [KeyModifiers::NONE, KeyModifiers::SHIFT] {
            assert_eq!(
                intent(press(KeyCode::BackTab, shift)),
                Some(Intent::PrevFocus)
            );
        }
    }

    #[test]
    fn esc_is_back_and_unnamed_keys_are_other() {
        assert_eq!(
            intent(press(KeyCode::Esc, KeyModifiers::NONE)),
            Some(Intent::Back)
        );
        assert_eq!(
            intent(press(KeyCode::Char('q'), KeyModifiers::NONE)),
            Some(Intent::Other)
        );
        assert_eq!(
            intent(press(KeyCode::F(5), KeyModifiers::NONE)),
            Some(Intent::Other)
        );
    }
}
