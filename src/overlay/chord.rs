//! The show / hide hotkey as a value: a [`Chord`] of Ctrl / Shift / Alt plus one key, its one text
//! form (`Ctrl+Shift+Alt+<key>`, the same in the Config button, `ff-settings.json`, the tray and
//! the status line), the egui-key table, and the pure capture state machine the Config tab's
//! keybind button runs on.
//!
//! Everything here is pure: no egui context, no OS call. `global-hotkey`'s own `FromStr` /
//! `Display` text (`shift+control+KeyZ`) is deliberately not used; this module owns the format.
//!
//! Known limitations:
//! - Numpad digits arrive from egui as the main-row digits (egui exposes no key location), so a
//!   chord captured on the numpad is saved and registered as the main-row digit; the numpad key
//!   itself will not trigger it.
//! - On layouts with AltGr, AltGr is Ctrl+Alt, so a `Ctrl+Alt+<key>` chord also swallows the
//!   character AltGr+<key> types, system-wide.
//! - Windows reserves F12 for debuggers; registering it may be refused by the OS, which the normal
//!   refusal path reports.

use global_hotkey::hotkey::{Code, HotKey, Modifiers};

/// A key combination: any of Ctrl / Shift / Alt plus one key. The Windows key is never part of
/// one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chord {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: ChordKey,
}

/// The keys a chord can end in: those egui reports and `global-hotkey` can register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChordKey {
    /// `b'A'..=b'Z'`.
    Letter(u8),
    /// `0..=9`.
    Digit(u8),
    /// `1..=24`.
    F(u8),
    Space,
    Tab,
    Enter,
    Backspace,
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    Comma,
    Period,
    Slash,
    Semicolon,
    Quote,
    BracketLeft,
    BracketRight,
    Minus,
    Equal,
    Backquote,
    Backslash,
}

/// The default show / hide hotkey, `Ctrl+Shift+Z`.
pub const DEFAULT: Chord = Chord {
    ctrl: true,
    shift: true,
    alt: false,
    key: ChordKey::Letter(b'Z'),
};

/// Chords the capture and `ff-settings.json` refuse, because binding them system-wide would
/// break FerrisFilter itself or a Windows-wide shortcut:
/// - `Ctrl+V`: the in-game reload after Apply pastes `/reloaditemfilter` with an injected Ctrl+V,
///   which a hotkey on that chord would swallow (and show the overlay instead).
/// - `Ctrl+C`, `Ctrl+X`, `Ctrl+A`, `Ctrl+Z`: copy, cut, select all and undo in every program.
/// - `Alt+F4`, `Alt+Tab`, `Shift+Alt+Tab`, `Alt+Space`: close, switch and window menu.
/// - `Ctrl+Alt+Delete`: the Windows security screen.
///
/// Escape (`Ctrl+Esc`, `Alt+Esc`, `Ctrl+Shift+Esc`) and the Windows key are refused outright by
/// [`Chord::parse`] and the capture, so they need no entry here. Only the exact chord is
/// reserved: `Ctrl+Shift+C` is free.
pub const RESERVED: &[Chord] = &[
    reserved(true, false, false, ChordKey::Letter(b'V')),
    reserved(true, false, false, ChordKey::Letter(b'C')),
    reserved(true, false, false, ChordKey::Letter(b'X')),
    reserved(true, false, false, ChordKey::Letter(b'A')),
    reserved(true, false, false, ChordKey::Letter(b'Z')),
    reserved(false, false, true, ChordKey::F(4)),
    reserved(false, false, true, ChordKey::Tab),
    reserved(false, true, true, ChordKey::Tab),
    reserved(false, false, true, ChordKey::Space),
    reserved(true, false, true, ChordKey::Delete),
];

const fn reserved(ctrl: bool, shift: bool, alt: bool, key: ChordKey) -> Chord {
    Chord {
        ctrl,
        shift,
        alt,
        key,
    }
}

/// Longest text [`Chord::parse`] looks at; anything longer is refused outright.
const MAX_TEXT_LEN: usize = 64;

/// Why a chord text was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChordError {
    Empty,
    TooLong,
    UnknownToken,
    NoKey,
    Duplicate,
    Forbidden,
}

/// Named keys and punctuation: (variant, canonical text).
const NAMED: &[(ChordKey, &str)] = &[
    (ChordKey::Space, "Space"),
    (ChordKey::Tab, "Tab"),
    (ChordKey::Enter, "Enter"),
    (ChordKey::Backspace, "Backspace"),
    (ChordKey::Insert, "Insert"),
    (ChordKey::Delete, "Delete"),
    (ChordKey::Home, "Home"),
    (ChordKey::End, "End"),
    (ChordKey::PageUp, "PageUp"),
    (ChordKey::PageDown, "PageDown"),
    (ChordKey::Up, "Up"),
    (ChordKey::Down, "Down"),
    (ChordKey::Left, "Left"),
    (ChordKey::Right, "Right"),
    (ChordKey::Comma, ","),
    (ChordKey::Period, "."),
    (ChordKey::Slash, "/"),
    (ChordKey::Semicolon, ";"),
    (ChordKey::Quote, "'"),
    (ChordKey::BracketLeft, "["),
    (ChordKey::BracketRight, "]"),
    (ChordKey::Minus, "-"),
    (ChordKey::Equal, "="),
    (ChordKey::Backquote, "`"),
    (ChordKey::Backslash, "\\"),
];

impl ChordKey {
    /// The key's text in the chord format.
    pub fn text(self) -> String {
        match self {
            ChordKey::Letter(c) => char::from(c).to_string(),
            ChordKey::Digit(d) => d.to_string(),
            ChordKey::F(n) => format!("F{n}"),
            other => NAMED
                .iter()
                .find(|(k, _)| *k == other)
                .map(|(_, t)| (*t).to_string())
                .unwrap_or_default(),
        }
    }

    /// Whether the value is in range (a letter A-Z, a digit 0-9, F1-F24, or a named key).
    fn is_valid(self) -> bool {
        match self {
            ChordKey::Letter(c) => c.is_ascii_uppercase(),
            ChordKey::Digit(d) => d <= 9,
            ChordKey::F(n) => (1..=24).contains(&n),
            _ => true,
        }
    }

    fn parse(token: &str) -> Result<ChordKey, ChordError> {
        let bytes = token.as_bytes();
        if bytes.len() == 1 {
            let b = bytes[0];
            if b.is_ascii_alphabetic() {
                return Ok(ChordKey::Letter(b.to_ascii_uppercase()));
            }
            if b.is_ascii_digit() {
                return Ok(ChordKey::Digit(b - b'0'));
            }
        }
        if let Some(rest) = token.strip_prefix(['F', 'f'])
            && !rest.is_empty()
            && rest.bytes().all(|b| b.is_ascii_digit())
        {
            if rest.starts_with('0') {
                return Err(ChordError::UnknownToken);
            }
            return match rest.parse::<u8>() {
                Ok(n) if (1..=24).contains(&n) => Ok(ChordKey::F(n)),
                _ => Err(ChordError::UnknownToken),
            };
        }
        if token.eq_ignore_ascii_case("escape") || token.eq_ignore_ascii_case("esc") {
            return Err(ChordError::Forbidden);
        }
        NAMED
            .iter()
            .find(|(_, t)| t.eq_ignore_ascii_case(token))
            .map(|(k, _)| *k)
            .ok_or(ChordError::UnknownToken)
    }

    fn code(self) -> Code {
        match self {
            ChordKey::Letter(c) => letter_code(c),
            ChordKey::Digit(d) => digit_code(d),
            ChordKey::F(n) => f_code(n),
            ChordKey::Space => Code::Space,
            ChordKey::Tab => Code::Tab,
            ChordKey::Enter => Code::Enter,
            ChordKey::Backspace => Code::Backspace,
            ChordKey::Insert => Code::Insert,
            ChordKey::Delete => Code::Delete,
            ChordKey::Home => Code::Home,
            ChordKey::End => Code::End,
            ChordKey::PageUp => Code::PageUp,
            ChordKey::PageDown => Code::PageDown,
            ChordKey::Up => Code::ArrowUp,
            ChordKey::Down => Code::ArrowDown,
            ChordKey::Left => Code::ArrowLeft,
            ChordKey::Right => Code::ArrowRight,
            ChordKey::Comma => Code::Comma,
            ChordKey::Period => Code::Period,
            ChordKey::Slash => Code::Slash,
            ChordKey::Semicolon => Code::Semicolon,
            ChordKey::Quote => Code::Quote,
            ChordKey::BracketLeft => Code::BracketLeft,
            ChordKey::BracketRight => Code::BracketRight,
            ChordKey::Minus => Code::Minus,
            ChordKey::Equal => Code::Equal,
            ChordKey::Backquote => Code::Backquote,
            ChordKey::Backslash => Code::Backslash,
        }
    }
}

fn letter_code(c: u8) -> Code {
    match c {
        b'A' => Code::KeyA,
        b'B' => Code::KeyB,
        b'C' => Code::KeyC,
        b'D' => Code::KeyD,
        b'E' => Code::KeyE,
        b'F' => Code::KeyF,
        b'G' => Code::KeyG,
        b'H' => Code::KeyH,
        b'I' => Code::KeyI,
        b'J' => Code::KeyJ,
        b'K' => Code::KeyK,
        b'L' => Code::KeyL,
        b'M' => Code::KeyM,
        b'N' => Code::KeyN,
        b'O' => Code::KeyO,
        b'P' => Code::KeyP,
        b'Q' => Code::KeyQ,
        b'R' => Code::KeyR,
        b'S' => Code::KeyS,
        b'T' => Code::KeyT,
        b'U' => Code::KeyU,
        b'V' => Code::KeyV,
        b'W' => Code::KeyW,
        b'X' => Code::KeyX,
        b'Y' => Code::KeyY,
        b'Z' => Code::KeyZ,
        _ => Code::Unidentified,
    }
}

fn digit_code(d: u8) -> Code {
    match d {
        0 => Code::Digit0,
        1 => Code::Digit1,
        2 => Code::Digit2,
        3 => Code::Digit3,
        4 => Code::Digit4,
        5 => Code::Digit5,
        6 => Code::Digit6,
        7 => Code::Digit7,
        8 => Code::Digit8,
        9 => Code::Digit9,
        _ => Code::Unidentified,
    }
}

fn f_code(n: u8) -> Code {
    match n {
        1 => Code::F1,
        2 => Code::F2,
        3 => Code::F3,
        4 => Code::F4,
        5 => Code::F5,
        6 => Code::F6,
        7 => Code::F7,
        8 => Code::F8,
        9 => Code::F9,
        10 => Code::F10,
        11 => Code::F11,
        12 => Code::F12,
        13 => Code::F13,
        14 => Code::F14,
        15 => Code::F15,
        16 => Code::F16,
        17 => Code::F17,
        18 => Code::F18,
        19 => Code::F19,
        20 => Code::F20,
        21 => Code::F21,
        22 => Code::F22,
        23 => Code::F23,
        24 => Code::F24,
        _ => Code::Unidentified,
    }
}

impl Chord {
    /// The chord in FerrisFilter's text form: modifiers in the order Ctrl, Shift, Alt, then the
    /// key, joined with `+` (`Ctrl+Shift+Z`).
    pub fn text(&self) -> String {
        let mut parts: Vec<String> = Vec::with_capacity(4);
        if self.ctrl {
            parts.push("Ctrl".to_string());
        }
        if self.shift {
            parts.push("Shift".to_string());
        }
        if self.alt {
            parts.push("Alt".to_string());
        }
        parts.push(self.key.text());
        parts.join("+")
    }

    /// Parses the text form. Modifiers and named keys are case-insensitive and modifiers may come
    /// in any order; the last token is the key (`+` is never a key, the `=` key is `=`). Refuses
    /// an empty or over-long text, a modifier-only chord, a repeated modifier, the Windows key
    /// (`Win`, `Super`, `Meta`, `Cmd`), Escape, unknown keys and more than one key.
    pub fn parse(s: &str) -> Result<Chord, ChordError> {
        if s.len() > MAX_TEXT_LEN {
            return Err(ChordError::TooLong);
        }
        if s.trim().is_empty() {
            return Err(ChordError::Empty);
        }
        let tokens: Vec<&str> = s.split('+').map(str::trim).collect();
        let (key_token, mods) = tokens.split_last().ok_or(ChordError::Empty)?;
        let mut chord = Chord {
            ctrl: false,
            shift: false,
            alt: false,
            key: ChordKey::Space,
        };
        for token in mods {
            let flag = match modifier_token(token)? {
                Some(Mod::Ctrl) => &mut chord.ctrl,
                Some(Mod::Shift) => &mut chord.shift,
                Some(Mod::Alt) => &mut chord.alt,
                None => return Err(ChordError::UnknownToken),
            };
            if *flag {
                return Err(ChordError::Duplicate);
            }
            *flag = true;
        }
        if key_token.is_empty() || modifier_token(key_token)?.is_some() {
            return Err(ChordError::NoKey);
        }
        chord.key = ChordKey::parse(key_token)?;
        Ok(chord)
    }

    /// Whether the chord has the modifier the rule asks for: any of Ctrl / Shift / Alt, or a bare
    /// F1-F24. Says nothing about [`RESERVED`]; see [`Chord::is_bindable`].
    pub fn has_modifier(&self) -> bool {
        self.key.is_valid()
            && (self.ctrl || self.shift || self.alt || matches!(self.key, ChordKey::F(_)))
    }

    /// Whether the chord is one of the [`RESERVED`] chords FerrisFilter refuses.
    pub fn is_reserved(&self) -> bool {
        RESERVED.contains(self)
    }

    /// Whether the chord may be registered: it [has a modifier](Chord::has_modifier) and is not
    /// [reserved](Chord::is_reserved).
    pub fn is_bindable(&self) -> bool {
        self.has_modifier() && !self.is_reserved()
    }

    /// The `global-hotkey` modifiers (never SUPER).
    pub fn modifiers(&self) -> Modifiers {
        let mut m = Modifiers::empty();
        if self.ctrl {
            m |= Modifiers::CONTROL;
        }
        if self.shift {
            m |= Modifiers::SHIFT;
        }
        if self.alt {
            m |= Modifiers::ALT;
        }
        m
    }

    /// The `global-hotkey` key code.
    pub fn code(&self) -> Code {
        self.key.code()
    }

    /// The id `global-hotkey` gives this chord (`GlobalHotKeyEvent::id` of a press). Pure.
    pub fn id(&self) -> u32 {
        HotKey::new(Some(self.modifiers()), self.code()).id()
    }
}

impl std::fmt::Display for Chord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text())
    }
}

enum Mod {
    Ctrl,
    Shift,
    Alt,
}

/// `Ok(Some)` for a modifier token, `Ok(None)` for anything else, `Err(Forbidden)` for the
/// Windows key.
fn modifier_token(token: &str) -> Result<Option<Mod>, ChordError> {
    let lower = token.to_ascii_lowercase();
    match lower.as_str() {
        "ctrl" => Ok(Some(Mod::Ctrl)),
        "shift" => Ok(Some(Mod::Shift)),
        "alt" => Ok(Some(Mod::Alt)),
        "win" | "windows" | "super" | "meta" | "cmd" => Err(ChordError::Forbidden),
        _ => Ok(None),
    }
}

/// Maps one egui key to a chord key, without any preference rule.
fn map_key(key: egui::Key) -> Option<ChordKey> {
    use egui::Key as K;
    let letter = |c: u8| Some(ChordKey::Letter(c));
    Some(match key {
        K::A => return letter(b'A'),
        K::B => return letter(b'B'),
        K::C => return letter(b'C'),
        K::D => return letter(b'D'),
        K::E => return letter(b'E'),
        K::F => return letter(b'F'),
        K::G => return letter(b'G'),
        K::H => return letter(b'H'),
        K::I => return letter(b'I'),
        K::J => return letter(b'J'),
        K::K => return letter(b'K'),
        K::L => return letter(b'L'),
        K::M => return letter(b'M'),
        K::N => return letter(b'N'),
        K::O => return letter(b'O'),
        K::P => return letter(b'P'),
        K::Q => return letter(b'Q'),
        K::R => return letter(b'R'),
        K::S => return letter(b'S'),
        K::T => return letter(b'T'),
        K::U => return letter(b'U'),
        K::V => return letter(b'V'),
        K::W => return letter(b'W'),
        K::X => return letter(b'X'),
        K::Y => return letter(b'Y'),
        K::Z => return letter(b'Z'),
        K::Num0 => ChordKey::Digit(0),
        K::Num1 => ChordKey::Digit(1),
        K::Num2 => ChordKey::Digit(2),
        K::Num3 => ChordKey::Digit(3),
        K::Num4 => ChordKey::Digit(4),
        K::Num5 => ChordKey::Digit(5),
        K::Num6 => ChordKey::Digit(6),
        K::Num7 => ChordKey::Digit(7),
        K::Num8 => ChordKey::Digit(8),
        K::Num9 => ChordKey::Digit(9),
        K::F1 => ChordKey::F(1),
        K::F2 => ChordKey::F(2),
        K::F3 => ChordKey::F(3),
        K::F4 => ChordKey::F(4),
        K::F5 => ChordKey::F(5),
        K::F6 => ChordKey::F(6),
        K::F7 => ChordKey::F(7),
        K::F8 => ChordKey::F(8),
        K::F9 => ChordKey::F(9),
        K::F10 => ChordKey::F(10),
        K::F11 => ChordKey::F(11),
        K::F12 => ChordKey::F(12),
        K::F13 => ChordKey::F(13),
        K::F14 => ChordKey::F(14),
        K::F15 => ChordKey::F(15),
        K::F16 => ChordKey::F(16),
        K::F17 => ChordKey::F(17),
        K::F18 => ChordKey::F(18),
        K::F19 => ChordKey::F(19),
        K::F20 => ChordKey::F(20),
        K::F21 => ChordKey::F(21),
        K::F22 => ChordKey::F(22),
        K::F23 => ChordKey::F(23),
        K::F24 => ChordKey::F(24),
        K::Space => ChordKey::Space,
        K::Tab => ChordKey::Tab,
        K::Enter => ChordKey::Enter,
        K::Backspace => ChordKey::Backspace,
        K::Insert => ChordKey::Insert,
        K::Delete => ChordKey::Delete,
        K::Home => ChordKey::Home,
        K::End => ChordKey::End,
        K::PageUp => ChordKey::PageUp,
        K::PageDown => ChordKey::PageDown,
        K::ArrowUp => ChordKey::Up,
        K::ArrowDown => ChordKey::Down,
        K::ArrowLeft => ChordKey::Left,
        K::ArrowRight => ChordKey::Right,
        K::Comma => ChordKey::Comma,
        K::Period => ChordKey::Period,
        K::Slash => ChordKey::Slash,
        K::Semicolon => ChordKey::Semicolon,
        K::Quote => ChordKey::Quote,
        K::OpenBracket => ChordKey::BracketLeft,
        K::CloseBracket => ChordKey::BracketRight,
        K::Minus => ChordKey::Minus,
        K::Equals => ChordKey::Equal,
        K::Backtick => ChordKey::Backquote,
        K::Backslash => ChordKey::Backslash,
        _ => return None,
    })
}

fn is_letter(key: egui::Key) -> bool {
    matches!(map_key(key), Some(ChordKey::Letter(_)))
}

fn is_modifier_key(key: egui::Key) -> bool {
    use egui::Key as K;
    matches!(
        key,
        K::ShiftLeft
            | K::ShiftRight
            | K::ControlLeft
            | K::ControlRight
            | K::AltLeft
            | K::AltRight
            | K::SuperLeft
            | K::SuperRight
    )
}

/// Resolves a key press to a chord key. A letter logical key wins (it matches the letter the
/// user's layout shows); any other key resolves through the physical key, which does not follow
/// Shift (Shift+`=` is still `=`), else the logical key. Escape, the modifier keys and keys
/// outside the table (F25-F35, IntlBackslash, ...) give `None`.
pub fn key_from_event(key: egui::Key, physical: Option<egui::Key>) -> Option<ChordKey> {
    if is_letter(key) {
        return map_key(key);
    }
    let k = physical.unwrap_or(key);
    if k == egui::Key::Escape || is_modifier_key(k) {
        return None;
    }
    map_key(k)
}

/// The keybind button's caption while capturing: `Press a key…` with no modifier held, else the
/// held modifiers (`Ctrl+Shift+…`).
pub fn held_caption(ctrl: bool, shift: bool, alt: bool) -> String {
    let mut out = String::new();
    for (held, name) in [(ctrl, "Ctrl+"), (shift, "Shift+"), (alt, "Alt+")] {
        if held {
            out.push_str(name);
        }
    }
    if out.is_empty() {
        "Press a key…".to_string()
    } else {
        out.push('…');
        out
    }
}

/// Why a captured press did not end the capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    NoModifier,
    WindowsKey,
    Unsupported,
    /// One of the [`RESERVED`] chords.
    Reserved,
}

impl RejectReason {
    /// The amber caption shown on the button until the next key or modifier change.
    pub fn caption(self) -> &'static str {
        match self {
            RejectReason::NoModifier => "Add Ctrl, Shift or Alt",
            RejectReason::WindowsKey => "Windows key not allowed",
            RejectReason::Unsupported => "Key not supported",
            RejectReason::Reserved => "Used by Windows or FerrisFilter",
        }
    }
}

/// The capture's state between frames.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CaptureState {
    /// The last refused press, shown on the button.
    pub rejected: Option<RejectReason>,
    /// Ctrl / Shift / Alt as seen last frame; a change clears `rejected`.
    last_held: (bool, bool, bool),
}

/// One frame of capture input: the key events drained this frame and the held keys.
pub struct CaptureInput<'a> {
    pub events: &'a [egui::Event],
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    /// The Windows key is held (egui reports it only as `SuperLeft/Right` key events).
    pub windows_key: bool,
    /// Insert is held: tells Ctrl+Insert apart from Ctrl+C, both of which egui turns into
    /// `Event::Copy`.
    pub insert_down: bool,
}

/// What the capture does after a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureStep {
    Continue,
    Cancel,
    Commit(Chord),
}

/// Runs one frame of the capture. Only fresh key presses and egui's Copy / Cut / Paste events
/// count (those three chords never arrive as key events); releases, repeats and the modifier
/// keys themselves are ignored. Escape, with or without modifiers, cancels. The first other press
/// commits when it makes a bindable chord (a modifier, not [`RESERVED`]) without the Windows key,
/// and is otherwise recorded as the rejection while the capture goes on.
pub fn capture_step(state: &mut CaptureState, input: &CaptureInput) -> CaptureStep {
    let held = (input.ctrl, input.shift, input.alt);
    if held != state.last_held {
        state.rejected = None;
        state.last_held = held;
    }
    for event in input.events {
        let (key, ctrl, shift, alt) = match event {
            egui::Event::Key {
                key,
                physical_key,
                pressed: true,
                repeat: false,
                modifiers,
            } => {
                if *key == egui::Key::Escape || *physical_key == Some(egui::Key::Escape) {
                    return CaptureStep::Cancel;
                }
                if is_modifier_key(*key) || physical_key.is_some_and(is_modifier_key) {
                    continue;
                }
                (
                    key_from_event(*key, *physical_key),
                    modifiers.ctrl,
                    modifiers.shift,
                    modifiers.alt,
                )
            }
            egui::Event::Copy => {
                let key = if input.insert_down {
                    ChordKey::Insert
                } else {
                    ChordKey::Letter(b'C')
                };
                (Some(key), input.ctrl, input.shift, input.alt)
            }
            egui::Event::Cut => {
                let key = if input.ctrl {
                    ChordKey::Letter(b'X')
                } else {
                    ChordKey::Delete
                };
                (Some(key), input.ctrl, input.shift, input.alt)
            }
            egui::Event::Paste(_) => {
                let key = if input.ctrl {
                    ChordKey::Letter(b'V')
                } else {
                    ChordKey::Insert
                };
                (Some(key), input.ctrl, input.shift, input.alt)
            }
            _ => continue,
        };
        let Some(key) = key else {
            state.rejected = Some(RejectReason::Unsupported);
            continue;
        };
        if input.windows_key {
            state.rejected = Some(RejectReason::WindowsKey);
            continue;
        }
        let chord = Chord {
            ctrl,
            shift,
            alt,
            key,
        };
        if !chord.has_modifier() {
            state.rejected = Some(RejectReason::NoModifier);
            continue;
        }
        if chord.is_reserved() {
            state.rejected = Some(RejectReason::Reserved);
            continue;
        }
        state.rejected = None;
        return CaptureStep::Commit(chord);
    }
    CaptureStep::Continue
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Key, Modifiers as EMods};

    fn all_keys() -> Vec<ChordKey> {
        let mut v: Vec<ChordKey> = (b'A'..=b'Z').map(ChordKey::Letter).collect();
        v.extend((0..=9).map(ChordKey::Digit));
        v.extend((1..=24).map(ChordKey::F));
        v.extend(NAMED.iter().map(|(k, _)| *k));
        v
    }

    fn chord(ctrl: bool, shift: bool, alt: bool, key: ChordKey) -> Chord {
        Chord {
            ctrl,
            shift,
            alt,
            key,
        }
    }

    fn mods(ctrl: bool, shift: bool, alt: bool) -> EMods {
        EMods {
            ctrl,
            command: ctrl,
            shift,
            alt,
            ..Default::default()
        }
    }

    fn press(key: Key, physical: Option<Key>, m: EMods) -> Event {
        Event::Key {
            key,
            physical_key: physical,
            pressed: true,
            repeat: false,
            modifiers: m,
        }
    }

    fn input<'a>(events: &'a [Event], ctrl: bool, shift: bool, alt: bool) -> CaptureInput<'a> {
        CaptureInput {
            events,
            ctrl,
            shift,
            alt,
            windows_key: false,
            insert_down: false,
        }
    }

    #[test]
    fn default_text_and_parse() {
        assert_eq!(DEFAULT.text(), "Ctrl+Shift+Z");
        assert_eq!(Chord::parse("Ctrl+Shift+Z"), Ok(DEFAULT));
        assert_eq!(Chord::parse("ctrl+shift+z"), Ok(DEFAULT));
        assert_eq!(Chord::parse("CTRL+SHIFT+Z"), Ok(DEFAULT));
        assert_eq!(
            Chord::parse("ctrl+shift+pageup").unwrap().text(),
            "Ctrl+Shift+PageUp"
        );
        assert_eq!(DEFAULT.to_string(), "Ctrl+Shift+Z");
    }

    #[test]
    fn modifier_order_is_canonical() {
        let c = Chord::parse("Alt+Ctrl+K").unwrap();
        assert_eq!(c, chord(true, false, true, ChordKey::Letter(b'K')));
        assert_eq!(c.text(), "Ctrl+Alt+K");
        assert_eq!(
            Chord::parse("Alt+Shift+Ctrl+1").unwrap().text(),
            "Ctrl+Shift+Alt+1"
        );
    }

    #[test]
    fn every_key_round_trips() {
        for key in all_keys() {
            for (c, s, a) in [
                (true, false, false),
                (false, true, true),
                (false, false, false),
            ] {
                let ch = chord(c, s, a, key);
                assert_eq!(Chord::parse(&ch.text()), Ok(ch), "{}", ch.text());
            }
            assert_ne!(key.code(), Code::Unidentified, "{key:?}");
        }
        assert_eq!(
            Chord::parse("Ctrl+Shift+="),
            Ok(chord(true, true, false, ChordKey::Equal))
        );
        assert_eq!(
            Chord::parse("Ctrl+-"),
            Ok(chord(true, false, false, ChordKey::Minus))
        );
    }

    #[test]
    fn parse_rejects_bad_texts() {
        let long = format!("Ctrl+{}", "A".repeat(70));
        let cases: &[(&str, ChordError)] = &[
            ("", ChordError::Empty),
            ("   ", ChordError::Empty),
            ("Ctrl+Shift", ChordError::NoKey),
            ("Ctrl+", ChordError::NoKey),
            ("Ctrl+Ctrl+Z", ChordError::Duplicate),
            ("Win+K", ChordError::Forbidden),
            ("Super+K", ChordError::Forbidden),
            ("Meta+K", ChordError::Forbidden),
            ("Cmd+K", ChordError::Forbidden),
            ("Ctrl+Win", ChordError::Forbidden),
            ("Ctrl+Escape", ChordError::Forbidden),
            ("Ctrl+Esc", ChordError::Forbidden),
            ("Ctrl+Nonsense", ChordError::UnknownToken),
            ("nonsense", ChordError::UnknownToken),
            ("Ctrl+F0", ChordError::UnknownToken),
            ("Ctrl+F25", ChordError::UnknownToken),
            ("Ctrl+F01", ChordError::UnknownToken),
            ("Ctrl+A+B", ChordError::UnknownToken),
            ("Ctrl++Z", ChordError::UnknownToken),
            (long.as_str(), ChordError::TooLong),
        ];
        for (text, want) in cases {
            assert_eq!(Chord::parse(text), Err(*want), "{text:?}");
        }
    }

    #[test]
    fn bindable_rule() {
        assert!(DEFAULT.is_bindable());
        assert!(chord(false, false, true, ChordKey::Letter(b'K')).is_bindable());
        assert!(chord(false, true, false, ChordKey::Digit(7)).is_bindable());
        for n in 1..=24 {
            assert!(chord(false, false, false, ChordKey::F(n)).is_bindable());
        }
        assert!(!chord(false, false, false, ChordKey::Letter(b'Z')).is_bindable());
        assert!(!chord(false, false, false, ChordKey::Digit(7)).is_bindable());
        assert!(!chord(false, false, false, ChordKey::Space).is_bindable());
        assert!(!Chord::parse("Z").unwrap().is_bindable());
    }

    #[test]
    fn reserved_chords_are_refused() {
        for text in [
            "Ctrl+V",
            "Ctrl+C",
            "Ctrl+X",
            "Ctrl+A",
            "Ctrl+Z",
            "Alt+F4",
            "Alt+Tab",
            "Shift+Alt+Tab",
            "Alt+Space",
            "Ctrl+Alt+Delete",
        ] {
            let c = Chord::parse(text).unwrap();
            assert!(c.has_modifier(), "{text}");
            assert!(c.is_reserved(), "{text}");
            assert!(!c.is_bindable(), "{text}");
        }
        assert!(!DEFAULT.is_reserved());
        // Only the exact chord is reserved: an extra modifier frees it.
        assert!(Chord::parse("Ctrl+Shift+C").unwrap().is_bindable());
        assert!(Chord::parse("Ctrl+Alt+F4").unwrap().is_bindable());
        for c in RESERVED {
            assert_eq!(Chord::parse(&c.text()), Ok(*c));
        }
    }

    #[test]
    fn capture_rejects_reserved_chords() {
        let mut st = CaptureState::default();
        let ev = [press(Key::V, Some(Key::V), mods(true, false, false))];
        assert_eq!(
            capture_step(&mut st, &input(&ev, true, false, false)),
            CaptureStep::Continue
        );
        assert_eq!(st.rejected, Some(RejectReason::Reserved));

        let mut st = CaptureState::default();
        let ev = [press(Key::F4, Some(Key::F4), mods(false, false, true))];
        assert_eq!(
            capture_step(&mut st, &input(&ev, false, false, true)),
            CaptureStep::Continue
        );
        assert_eq!(st.rejected, Some(RejectReason::Reserved));
    }

    #[test]
    fn code_modifiers_and_id() {
        assert_eq!(DEFAULT.code(), Code::KeyZ);
        assert_eq!(DEFAULT.modifiers(), Modifiers::CONTROL | Modifiers::SHIFT);
        assert_eq!(
            chord(true, false, false, ChordKey::Digit(3)).code(),
            Code::Digit3
        );
        assert_eq!(chord(true, false, false, ChordKey::F(17)).code(), Code::F17);
        assert_eq!(
            chord(true, false, false, ChordKey::Up).code(),
            Code::ArrowUp
        );
        assert_eq!(
            chord(true, false, false, ChordKey::BracketLeft).code(),
            Code::BracketLeft
        );
        assert_eq!(
            chord(true, false, false, ChordKey::Equal).code(),
            Code::Equal
        );
        assert_eq!(
            chord(true, false, false, ChordKey::Backquote).code(),
            Code::Backquote
        );
        for key in all_keys() {
            let c = chord(true, true, true, key);
            assert!(!c.modifiers().contains(Modifiers::SUPER));
            assert_eq!(c.id(), HotKey::new(Some(c.modifiers()), c.code()).id());
        }
        let k = chord(true, false, true, ChordKey::Letter(b'K'));
        assert_eq!(k.modifiers(), Modifiers::CONTROL | Modifiers::ALT);
        assert_ne!(DEFAULT.id(), k.id());
    }

    #[test]
    fn key_from_event_rules() {
        assert_eq!(
            key_from_event(Key::Z, Some(Key::Z)),
            Some(ChordKey::Letter(b'Z'))
        );
        // AZERTY: logical letter wins over the physical position.
        assert_eq!(
            key_from_event(Key::A, Some(Key::Q)),
            Some(ChordKey::Letter(b'A'))
        );
        assert_eq!(
            key_from_event(Key::Plus, Some(Key::Equals)),
            Some(ChordKey::Equal)
        );
        assert_eq!(
            key_from_event(Key::Exclamationmark, Some(Key::Num1)),
            Some(ChordKey::Digit(1))
        );
        assert_eq!(key_from_event(Key::F5, None), Some(ChordKey::F(5)));
        assert_eq!(key_from_event(Key::F25, Some(Key::F25)), None);
        assert_eq!(key_from_event(Key::F35, None), None);
        assert_eq!(
            key_from_event(Key::IntlBackslash, Some(Key::IntlBackslash)),
            None
        );
        assert_eq!(key_from_event(Key::Escape, Some(Key::Escape)), None);
        assert_eq!(key_from_event(Key::Plus, None), None);
        for m in [
            Key::ShiftLeft,
            Key::ShiftRight,
            Key::ControlLeft,
            Key::ControlRight,
            Key::AltLeft,
            Key::AltRight,
            Key::SuperLeft,
            Key::SuperRight,
        ] {
            assert_eq!(key_from_event(m, Some(m)), None, "{m:?}");
        }
    }

    #[test]
    fn capture_ignores_modifier_presses_releases_and_repeats() {
        let mut st = CaptureState::default();
        let ev = [press(
            Key::ControlLeft,
            Some(Key::ControlLeft),
            mods(true, false, false),
        )];
        assert_eq!(
            capture_step(&mut st, &input(&ev, true, false, false)),
            CaptureStep::Continue
        );
        assert_eq!(st.rejected, None);
        let ev = [
            Event::Key {
                key: Key::Z,
                physical_key: Some(Key::Z),
                pressed: false,
                repeat: false,
                modifiers: mods(true, false, false),
            },
            Event::Key {
                key: Key::Z,
                physical_key: Some(Key::Z),
                pressed: true,
                repeat: true,
                modifiers: mods(true, false, false),
            },
        ];
        assert_eq!(
            capture_step(&mut st, &input(&ev, true, false, false)),
            CaptureStep::Continue
        );
        assert_eq!(st.rejected, None);
    }

    #[test]
    fn capture_escape_cancels() {
        let mut st = CaptureState::default();
        let ev = [press(Key::Escape, Some(Key::Escape), EMods::default())];
        assert_eq!(
            capture_step(&mut st, &input(&ev, false, false, false)),
            CaptureStep::Cancel
        );
        let ev = [press(
            Key::Escape,
            Some(Key::Escape),
            mods(true, true, false),
        )];
        assert_eq!(
            capture_step(&mut st, &input(&ev, true, true, false)),
            CaptureStep::Cancel
        );
    }

    #[test]
    fn capture_commits_and_rejects() {
        let mut st = CaptureState::default();
        let ev = [press(Key::K, Some(Key::K), mods(true, false, false))];
        assert_eq!(
            capture_step(&mut st, &input(&ev, true, false, false)),
            CaptureStep::Commit(chord(true, false, false, ChordKey::Letter(b'K')))
        );

        let mut st = CaptureState::default();
        let ev = [press(Key::Z, Some(Key::Z), EMods::default())];
        assert_eq!(
            capture_step(&mut st, &input(&ev, false, false, false)),
            CaptureStep::Continue
        );
        assert_eq!(st.rejected, Some(RejectReason::NoModifier));

        let mut st = CaptureState::default();
        let ev = [press(Key::F5, Some(Key::F5), EMods::default())];
        assert_eq!(
            capture_step(&mut st, &input(&ev, false, false, false)),
            CaptureStep::Commit(chord(false, false, false, ChordKey::F(5)))
        );

        let mut st = CaptureState::default();
        let ev = [press(Key::K, Some(Key::K), mods(true, false, false))];
        let mut inp = input(&ev, true, false, false);
        inp.windows_key = true;
        assert_eq!(capture_step(&mut st, &inp), CaptureStep::Continue);
        assert_eq!(st.rejected, Some(RejectReason::WindowsKey));

        let mut st = CaptureState::default();
        let ev = [press(Key::F30, Some(Key::F30), mods(true, false, false))];
        assert_eq!(
            capture_step(&mut st, &input(&ev, true, false, false)),
            CaptureStep::Continue
        );
        assert_eq!(st.rejected, Some(RejectReason::Unsupported));

        // Shifted punctuation resolves to its physical key.
        let mut st = CaptureState::default();
        let ev = [press(Key::Plus, Some(Key::Equals), mods(true, true, false))];
        assert_eq!(
            capture_step(&mut st, &input(&ev, true, true, false)),
            CaptureStep::Commit(chord(true, true, false, ChordKey::Equal))
        );
    }

    #[test]
    fn rejection_clears_on_modifier_change_or_next_key() {
        let mut st = CaptureState::default();
        let ev = [press(Key::Z, Some(Key::Z), EMods::default())];
        capture_step(&mut st, &input(&ev, false, false, false));
        assert_eq!(st.rejected, Some(RejectReason::NoModifier));
        // Same modifiers, no event: the caption stays.
        assert_eq!(
            capture_step(&mut st, &input(&[], false, false, false)),
            CaptureStep::Continue
        );
        assert_eq!(st.rejected, Some(RejectReason::NoModifier));
        // A modifier goes down: cleared.
        assert_eq!(
            capture_step(&mut st, &input(&[], false, true, false)),
            CaptureStep::Continue
        );
        assert_eq!(st.rejected, None);

        // A later key replaces the reason.
        let mut st = CaptureState::default();
        capture_step(&mut st, &input(&ev, false, false, false));
        let ev2 = [press(Key::F30, Some(Key::F30), EMods::default())];
        capture_step(&mut st, &input(&ev2, false, false, false));
        assert_eq!(st.rejected, Some(RejectReason::Unsupported));
    }

    #[test]
    fn copy_cut_paste_resolve_to_chords() {
        let commit = |ev: Event, ctrl: bool, shift: bool, insert: bool| {
            let mut st = CaptureState::default();
            let evs = [ev];
            let mut inp = input(&evs, ctrl, shift, false);
            inp.insert_down = insert;
            capture_step(&mut st, &inp)
        };
        let c = |ctrl, shift, key| CaptureStep::Commit(chord(ctrl, shift, false, key));
        // Ctrl+C, Ctrl+X and Ctrl+V resolve to their chords, which are reserved.
        let reserved = |ev: Event| {
            let mut st = CaptureState::default();
            let evs = [ev];
            let step = capture_step(&mut st, &input(&evs, true, false, false));
            (step, st.rejected)
        };
        assert_eq!(
            reserved(Event::Copy),
            (CaptureStep::Continue, Some(RejectReason::Reserved))
        );
        assert_eq!(
            reserved(Event::Cut),
            (CaptureStep::Continue, Some(RejectReason::Reserved))
        );
        assert_eq!(
            reserved(Event::Paste("x".into())),
            (CaptureStep::Continue, Some(RejectReason::Reserved))
        );
        assert_eq!(
            commit(Event::Copy, true, false, true),
            c(true, false, ChordKey::Insert)
        );
        assert_eq!(
            commit(Event::Copy, true, true, false),
            c(true, true, ChordKey::Letter(b'C'))
        );
        assert_eq!(
            commit(Event::Cut, false, true, false),
            c(false, true, ChordKey::Delete)
        );
        assert_eq!(
            commit(Event::Paste(String::new()), false, true, false),
            c(false, true, ChordKey::Insert)
        );
        assert_eq!(
            commit(Event::Paste(String::new()), true, true, false),
            c(true, true, ChordKey::Letter(b'V'))
        );
    }

    #[test]
    fn held_caption_text() {
        assert_eq!(held_caption(false, false, false), "Press a key…");
        assert_eq!(held_caption(true, true, false), "Ctrl+Shift+…");
        assert_eq!(held_caption(false, false, true), "Alt+…");
    }

    #[test]
    fn reject_captions() {
        assert_eq!(RejectReason::NoModifier.caption(), "Add Ctrl, Shift or Alt");
        assert_eq!(
            RejectReason::WindowsKey.caption(),
            "Windows key not allowed"
        );
        assert_eq!(RejectReason::Unsupported.caption(), "Key not supported");
        assert_eq!(
            RejectReason::Reserved.caption(),
            "Used by Windows or FerrisFilter"
        );
    }
}
