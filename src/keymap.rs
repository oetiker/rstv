//! Data-driven, process-global keymap shared by the editor and input line.
//!
//! Models the VS Code keybindings shape — a chord (1–2 keystrokes) maps to a
//! `Command` by name. Generalizes the C++ editor's `firstKeys`/`quickKeys`/
//! `blockKeys` tables and `key_state` prefix machine. See
//! `docs/superpowers/specs/2026-06-12-configurable-keymap-design.md`.

use crate::command::Command;
use crate::event::{Key, KeyEvent, KeyModifiers};
use std::collections::{HashMap, HashSet};
use std::sync::{OnceLock, RwLock};

/// One normalized keystroke: a `Key` plus the three real modifiers.
///
/// Normalization (`from_event`) folds two cases so presets stay small and the
/// C++ "second prefix key is uppercased" / "shift+arrow == arrow" behaviors are
/// preserved:
/// * **Alphabetic `Char`** → lowercased, `shift` forced false (letter commands
///   never depend on shift; `ctrl+q a` == `ctrl+q A`).
/// * **Cursor-pad keys** (`Left/Right/Up/Down/Home/End/PageUp/PageDown`) →
///   `shift` forced false. Shift on those is a *selection* modifier handled in
///   the widgets, never a distinct binding (so `shift+Left` resolves to the
///   same movement as `Left`).
/// * **Everything else** (`Insert/Delete/Tab/Enter/F-keys/punctuation`) keeps
///   `shift` — so `shift+Insert` (paste) stays distinct from `Insert`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct KeyStroke {
    pub key: Key,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl KeyStroke {
    /// Normalize a raw key event into a lookup key.
    pub fn from_event(ke: KeyEvent) -> Self {
        let KeyModifiers { shift, ctrl, alt } = ke.modifiers;
        Self::normalize(ke.key, ctrl, alt, shift)
    }

    pub(crate) fn normalize(key: Key, ctrl: bool, alt: bool, shift: bool) -> Self {
        match key {
            Key::Char(c) if c.is_ascii_alphabetic() => KeyStroke {
                key: Key::Char(c.to_ascii_lowercase()),
                ctrl,
                alt,
                shift: false,
            },
            Key::Left
            | Key::Right
            | Key::Up
            | Key::Down
            | Key::Home
            | Key::End
            | Key::PageUp
            | Key::PageDown => KeyStroke {
                key,
                ctrl,
                alt,
                shift: false,
            },
            _ => KeyStroke {
                key,
                ctrl,
                alt,
                shift,
            },
        }
    }
}

/// A chord: one keystroke, or two for a prefix sequence (Ctrl-K / Ctrl-Q style).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Chord(pub Vec<KeyStroke>);

/// Parse a VS Code-style chord string: space-separated strokes, each a
/// `+`-joined list of `ctrl|shift|alt|cmd|meta` modifiers ending in a key name.
/// Pure (no I/O). `cmd`/`meta` are accepted as aliases for `ctrl` (portability).
pub fn parse_chord(s: &str) -> Result<Chord, String> {
    let strokes: Vec<&str> = s.split_whitespace().collect();
    if strokes.is_empty() {
        return Err(format!("empty chord: {s:?}"));
    }
    let mut out = Vec::with_capacity(strokes.len());
    for stroke in strokes {
        out.push(parse_stroke(stroke)?);
    }
    Ok(Chord(out))
}

fn parse_stroke(s: &str) -> Result<KeyStroke, String> {
    let (mut ctrl, mut alt, mut shift) = (false, false, false);
    let mut key: Option<Key> = None;
    for tok in s.split('+') {
        match tok.to_ascii_lowercase().as_str() {
            "ctrl" | "cmd" | "meta" => ctrl = true,
            "alt" | "opt" | "option" => alt = true,
            "shift" => shift = true,
            other => key = Some(parse_key(other)?),
        }
    }
    let key = key.ok_or_else(|| format!("no key in stroke {s:?}"))?;
    Ok(KeyStroke::normalize(key, ctrl, alt, shift))
}

fn parse_key(name: &str) -> Result<Key, String> {
    Ok(match name {
        "backspace" | "bs" => Key::Backspace,
        "delete" | "del" => Key::Delete,
        "insert" | "ins" => Key::Insert,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" | "pgup" => Key::PageUp,
        "pagedown" | "pgdn" => Key::PageDown,
        "left" => Key::Left,
        "right" => Key::Right,
        "up" => Key::Up,
        "down" => Key::Down,
        "enter" | "return" => Key::Enter,
        "tab" => Key::Tab,
        "esc" | "escape" => Key::Esc,
        "space" => Key::Char(' '),
        f if f.starts_with('f') && f[1..].parse::<u8>().is_ok() => Key::F(f[1..].parse().unwrap()),
        c if c.chars().count() == 1 => Key::Char(c.chars().next().unwrap()),
        other => return Err(format!("unknown key name {other:?}")),
    })
}

// Placeholders needed by later tasks — will be filled in.
pub enum Resolve {
    Command(Command),
    Prefix,
    None,
}
#[derive(Clone, Default)]
pub struct Keymap {
    bindings: HashMap<Chord, Command>,
    prefixes: HashSet<KeyStroke>,
}
impl Keymap {
    pub fn new() -> Self {
        Keymap::default()
    }
}

pub fn set_global(_km: Keymap) {}
pub fn resolve_global(_pending: Option<KeyStroke>, _stroke: KeyStroke) -> Resolve {
    Resolve::None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(key: Key, ctrl: bool, alt: bool, shift: bool) -> KeyEvent {
        KeyEvent::new(key, KeyModifiers { shift, ctrl, alt })
    }

    #[test]
    fn letters_fold_case_and_shift() {
        let lower = KeyStroke::from_event(ev(Key::Char('a'), true, false, false));
        let upper = KeyStroke::from_event(ev(Key::Char('A'), true, false, true));
        assert_eq!(lower, upper);
        assert_eq!(lower.key, Key::Char('a'));
        assert!(!lower.shift);
    }

    #[test]
    fn shift_arrow_equals_arrow() {
        let plain = KeyStroke::from_event(ev(Key::Left, false, false, false));
        let shifted = KeyStroke::from_event(ev(Key::Left, false, false, true));
        assert_eq!(plain, shifted);
    }

    #[test]
    fn shift_insert_stays_distinct() {
        let plain = KeyStroke::from_event(ev(Key::Insert, false, false, false));
        let shifted = KeyStroke::from_event(ev(Key::Insert, false, false, true));
        assert_ne!(plain, shifted);
    }

    #[test]
    fn parse_single_stroke() {
        let c = parse_chord("ctrl+c").unwrap();
        assert_eq!(c.0.len(), 1);
        assert_eq!(
            c.0[0],
            KeyStroke::normalize(Key::Char('c'), true, false, false)
        );
    }

    #[test]
    fn parse_named_and_modifiers() {
        assert_eq!(
            parse_chord("shift+insert").unwrap().0[0],
            KeyStroke::normalize(Key::Insert, false, false, true)
        );
        assert_eq!(
            parse_chord("alt+backspace").unwrap().0[0],
            KeyStroke::normalize(Key::Backspace, false, true, false)
        );
        assert_eq!(
            parse_chord("f5").unwrap().0[0],
            KeyStroke::normalize(Key::F(5), false, false, false)
        );
    }

    #[test]
    fn parse_two_stroke_chord() {
        let c = parse_chord("ctrl+k ctrl+c").unwrap();
        assert_eq!(c.0.len(), 2);
    }

    #[test]
    fn parse_rejects_garbage() {
        assert!(parse_chord("ctrl+nope").is_err());
        assert!(parse_chord("").is_err());
    }
}
