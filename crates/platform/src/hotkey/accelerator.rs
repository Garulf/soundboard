use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use std::fmt;
use std::str::FromStr;

struct KeyDef {
    name: &'static str,
    aliases: &'static [&'static str],
    code: Code,
    xkb: &'static str,
}

#[rustfmt::skip]
static KEYS: &[KeyDef] = &[
    KeyDef { name: "A", aliases: &[], code: Code::KeyA, xkb: "a" },
    KeyDef { name: "B", aliases: &[], code: Code::KeyB, xkb: "b" },
    KeyDef { name: "C", aliases: &[], code: Code::KeyC, xkb: "c" },
    KeyDef { name: "D", aliases: &[], code: Code::KeyD, xkb: "d" },
    KeyDef { name: "E", aliases: &[], code: Code::KeyE, xkb: "e" },
    KeyDef { name: "F", aliases: &[], code: Code::KeyF, xkb: "f" },
    KeyDef { name: "G", aliases: &[], code: Code::KeyG, xkb: "g" },
    KeyDef { name: "H", aliases: &[], code: Code::KeyH, xkb: "h" },
    KeyDef { name: "I", aliases: &[], code: Code::KeyI, xkb: "i" },
    KeyDef { name: "J", aliases: &[], code: Code::KeyJ, xkb: "j" },
    KeyDef { name: "K", aliases: &[], code: Code::KeyK, xkb: "k" },
    KeyDef { name: "L", aliases: &[], code: Code::KeyL, xkb: "l" },
    KeyDef { name: "M", aliases: &[], code: Code::KeyM, xkb: "m" },
    KeyDef { name: "N", aliases: &[], code: Code::KeyN, xkb: "n" },
    KeyDef { name: "O", aliases: &[], code: Code::KeyO, xkb: "o" },
    KeyDef { name: "P", aliases: &[], code: Code::KeyP, xkb: "p" },
    KeyDef { name: "Q", aliases: &[], code: Code::KeyQ, xkb: "q" },
    KeyDef { name: "R", aliases: &[], code: Code::KeyR, xkb: "r" },
    KeyDef { name: "S", aliases: &[], code: Code::KeyS, xkb: "s" },
    KeyDef { name: "T", aliases: &[], code: Code::KeyT, xkb: "t" },
    KeyDef { name: "U", aliases: &[], code: Code::KeyU, xkb: "u" },
    KeyDef { name: "V", aliases: &[], code: Code::KeyV, xkb: "v" },
    KeyDef { name: "W", aliases: &[], code: Code::KeyW, xkb: "w" },
    KeyDef { name: "X", aliases: &[], code: Code::KeyX, xkb: "x" },
    KeyDef { name: "Y", aliases: &[], code: Code::KeyY, xkb: "y" },
    KeyDef { name: "Z", aliases: &[], code: Code::KeyZ, xkb: "z" },
    KeyDef { name: "0", aliases: &[], code: Code::Digit0, xkb: "0" },
    KeyDef { name: "1", aliases: &[], code: Code::Digit1, xkb: "1" },
    KeyDef { name: "2", aliases: &[], code: Code::Digit2, xkb: "2" },
    KeyDef { name: "3", aliases: &[], code: Code::Digit3, xkb: "3" },
    KeyDef { name: "4", aliases: &[], code: Code::Digit4, xkb: "4" },
    KeyDef { name: "5", aliases: &[], code: Code::Digit5, xkb: "5" },
    KeyDef { name: "6", aliases: &[], code: Code::Digit6, xkb: "6" },
    KeyDef { name: "7", aliases: &[], code: Code::Digit7, xkb: "7" },
    KeyDef { name: "8", aliases: &[], code: Code::Digit8, xkb: "8" },
    KeyDef { name: "9", aliases: &[], code: Code::Digit9, xkb: "9" },
    KeyDef { name: "F1", aliases: &[], code: Code::F1, xkb: "F1" },
    KeyDef { name: "F2", aliases: &[], code: Code::F2, xkb: "F2" },
    KeyDef { name: "F3", aliases: &[], code: Code::F3, xkb: "F3" },
    KeyDef { name: "F4", aliases: &[], code: Code::F4, xkb: "F4" },
    KeyDef { name: "F5", aliases: &[], code: Code::F5, xkb: "F5" },
    KeyDef { name: "F6", aliases: &[], code: Code::F6, xkb: "F6" },
    KeyDef { name: "F7", aliases: &[], code: Code::F7, xkb: "F7" },
    KeyDef { name: "F8", aliases: &[], code: Code::F8, xkb: "F8" },
    KeyDef { name: "F9", aliases: &[], code: Code::F9, xkb: "F9" },
    KeyDef { name: "F10", aliases: &[], code: Code::F10, xkb: "F10" },
    KeyDef { name: "F11", aliases: &[], code: Code::F11, xkb: "F11" },
    KeyDef { name: "F12", aliases: &[], code: Code::F12, xkb: "F12" },
    KeyDef { name: "F13", aliases: &[], code: Code::F13, xkb: "F13" },
    KeyDef { name: "F14", aliases: &[], code: Code::F14, xkb: "F14" },
    KeyDef { name: "F15", aliases: &[], code: Code::F15, xkb: "F15" },
    KeyDef { name: "F16", aliases: &[], code: Code::F16, xkb: "F16" },
    KeyDef { name: "F17", aliases: &[], code: Code::F17, xkb: "F17" },
    KeyDef { name: "F18", aliases: &[], code: Code::F18, xkb: "F18" },
    KeyDef { name: "F19", aliases: &[], code: Code::F19, xkb: "F19" },
    KeyDef { name: "F20", aliases: &[], code: Code::F20, xkb: "F20" },
    KeyDef { name: "F21", aliases: &[], code: Code::F21, xkb: "F21" },
    KeyDef { name: "F22", aliases: &[], code: Code::F22, xkb: "F22" },
    KeyDef { name: "F23", aliases: &[], code: Code::F23, xkb: "F23" },
    KeyDef { name: "F24", aliases: &[], code: Code::F24, xkb: "F24" },
    KeyDef { name: "Space", aliases: &[], code: Code::Space, xkb: "space" },
    KeyDef { name: "Enter", aliases: &["Return"], code: Code::Enter, xkb: "Return" },
    KeyDef { name: "Tab", aliases: &[], code: Code::Tab, xkb: "Tab" },
    KeyDef { name: "Escape", aliases: &["Esc"], code: Code::Escape, xkb: "Escape" },
    KeyDef { name: "Backspace", aliases: &[], code: Code::Backspace, xkb: "BackSpace" },
    KeyDef { name: "Delete", aliases: &["Del"], code: Code::Delete, xkb: "Delete" },
    KeyDef { name: "Insert", aliases: &["Ins"], code: Code::Insert, xkb: "Insert" },
    KeyDef { name: "Home", aliases: &[], code: Code::Home, xkb: "Home" },
    KeyDef { name: "End", aliases: &[], code: Code::End, xkb: "End" },
    KeyDef { name: "PageUp", aliases: &["PgUp"], code: Code::PageUp, xkb: "Prior" },
    KeyDef { name: "PageDown", aliases: &["PgDown", "PgDn"], code: Code::PageDown, xkb: "Next" },
    KeyDef { name: "Up", aliases: &["ArrowUp"], code: Code::ArrowUp, xkb: "Up" },
    KeyDef { name: "Down", aliases: &["ArrowDown"], code: Code::ArrowDown, xkb: "Down" },
    KeyDef { name: "Left", aliases: &["ArrowLeft"], code: Code::ArrowLeft, xkb: "Left" },
    KeyDef { name: "Right", aliases: &["ArrowRight"], code: Code::ArrowRight, xkb: "Right" },
    KeyDef { name: "Minus", aliases: &["-"], code: Code::Minus, xkb: "minus" },
    KeyDef { name: "Equal", aliases: &["Equals", "="], code: Code::Equal, xkb: "equal" },
    KeyDef { name: "Comma", aliases: &[","], code: Code::Comma, xkb: "comma" },
    KeyDef { name: "Period", aliases: &["."], code: Code::Period, xkb: "period" },
    KeyDef { name: "Slash", aliases: &["/"], code: Code::Slash, xkb: "slash" },
    KeyDef { name: "Semicolon", aliases: &[";"], code: Code::Semicolon, xkb: "semicolon" },
    KeyDef { name: "Quote", aliases: &["'"], code: Code::Quote, xkb: "apostrophe" },
    KeyDef { name: "BracketLeft", aliases: &["["], code: Code::BracketLeft, xkb: "bracketleft" },
    KeyDef { name: "BracketRight", aliases: &["]"], code: Code::BracketRight, xkb: "bracketright" },
    KeyDef { name: "Backslash", aliases: &["\\"], code: Code::Backslash, xkb: "backslash" },
    KeyDef { name: "Backquote", aliases: &["`", "Grave"], code: Code::Backquote, xkb: "grave" },
    KeyDef { name: "Num0", aliases: &["Numpad0"], code: Code::Numpad0, xkb: "KP_0" },
    KeyDef { name: "Num1", aliases: &["Numpad1"], code: Code::Numpad1, xkb: "KP_1" },
    KeyDef { name: "Num2", aliases: &["Numpad2"], code: Code::Numpad2, xkb: "KP_2" },
    KeyDef { name: "Num3", aliases: &["Numpad3"], code: Code::Numpad3, xkb: "KP_3" },
    KeyDef { name: "Num4", aliases: &["Numpad4"], code: Code::Numpad4, xkb: "KP_4" },
    KeyDef { name: "Num5", aliases: &["Numpad5"], code: Code::Numpad5, xkb: "KP_5" },
    KeyDef { name: "Num6", aliases: &["Numpad6"], code: Code::Numpad6, xkb: "KP_6" },
    KeyDef { name: "Num7", aliases: &["Numpad7"], code: Code::Numpad7, xkb: "KP_7" },
    KeyDef { name: "Num8", aliases: &["Numpad8"], code: Code::Numpad8, xkb: "KP_8" },
    KeyDef { name: "Num9", aliases: &["Numpad9"], code: Code::Numpad9, xkb: "KP_9" },
    KeyDef { name: "NumAdd", aliases: &["NumpadAdd"], code: Code::NumpadAdd, xkb: "KP_Add" },
    KeyDef { name: "NumSubtract", aliases: &["NumpadSubtract"], code: Code::NumpadSubtract, xkb: "KP_Subtract" },
    KeyDef { name: "NumMultiply", aliases: &["NumpadMultiply"], code: Code::NumpadMultiply, xkb: "KP_Multiply" },
    KeyDef { name: "NumDivide", aliases: &["NumpadDivide"], code: Code::NumpadDivide, xkb: "KP_Divide" },
    KeyDef { name: "NumDecimal", aliases: &["NumpadDecimal"], code: Code::NumpadDecimal, xkb: "KP_Decimal" },
    KeyDef { name: "NumEnter", aliases: &["NumpadEnter"], code: Code::NumpadEnter, xkb: "KP_Enter" },
    KeyDef { name: "PrintScreen", aliases: &["Print"], code: Code::PrintScreen, xkb: "Print" },
    KeyDef { name: "ScrollLock", aliases: &[], code: Code::ScrollLock, xkb: "Scroll_Lock" },
    KeyDef { name: "Pause", aliases: &[], code: Code::Pause, xkb: "Pause" },
    KeyDef { name: "MediaPlayPause", aliases: &[], code: Code::MediaPlayPause, xkb: "XF86AudioPlay" },
    KeyDef { name: "MediaStop", aliases: &[], code: Code::MediaStop, xkb: "XF86AudioStop" },
    KeyDef { name: "MediaNext", aliases: &["MediaTrackNext"], code: Code::MediaTrackNext, xkb: "XF86AudioNext" },
    KeyDef { name: "MediaPrev", aliases: &["MediaTrackPrevious"], code: Code::MediaTrackPrevious, xkb: "XF86AudioPrev" },
];

/// A key that can be bound, identified by its index in the key table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key(usize);

impl Key {
    pub fn from_name(name: &str) -> Option<Self> {
        KEYS.iter()
            .position(|k| {
                k.name.eq_ignore_ascii_case(name)
                    || k.aliases.iter().any(|a| a.eq_ignore_ascii_case(name))
            })
            .map(Key)
    }

    pub fn name(self) -> &'static str {
        KEYS[self.0].name
    }

    fn def(self) -> &'static KeyDef {
        &KEYS[self.0]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Accelerator {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub logo: bool,
    pub key: Key,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AcceleratorError {
    #[error("empty shortcut")]
    Empty,
    #[error("unknown key \"{0}\"")]
    UnknownKey(String),
    #[error("a shortcut needs exactly one non-modifier key")]
    KeyCount,
}

impl Accelerator {
    pub fn new(key: Key) -> Self {
        Self {
            ctrl: false,
            shift: false,
            alt: false,
            logo: false,
            key,
        }
    }

    fn modifier_names(&self, names: [&'static str; 4]) -> impl Iterator<Item = &'static str> {
        [self.ctrl, self.shift, self.alt, self.logo]
            .into_iter()
            .zip(names)
            .filter_map(|(on, name)| on.then_some(name))
    }

    /// Trigger string in the XDG shortcuts format used by the GlobalShortcuts portal.
    pub fn to_portal_trigger(&self) -> String {
        self.modifier_names(["CTRL", "SHIFT", "ALT", "LOGO"])
            .chain([self.key.def().xkb])
            .collect::<Vec<_>>()
            .join("+")
    }

    pub fn to_global_hotkey(&self) -> HotKey {
        let mut mods = Modifiers::empty();
        mods.set(Modifiers::CONTROL, self.ctrl);
        mods.set(Modifiers::SHIFT, self.shift);
        mods.set(Modifiers::ALT, self.alt);
        mods.set(Modifiers::SUPER, self.logo);
        HotKey::new(Some(mods), self.key.def().code)
    }
}

impl fmt::Display for Accelerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<_> = self
            .modifier_names(["Ctrl", "Shift", "Alt", "Super"])
            .chain([self.key.name()])
            .collect();
        f.write_str(&parts.join("+"))
    }
}

impl FromStr for Accelerator {
    type Err = AcceleratorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(AcceleratorError::Empty);
        }
        let (mut ctrl, mut shift, mut alt, mut logo) = (false, false, false, false);
        let mut key = None;
        for part in s.split('+').map(str::trim) {
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => ctrl = true,
                "shift" => shift = true,
                "alt" | "option" => alt = true,
                "super" | "logo" | "meta" | "win" | "cmd" | "command" => logo = true,
                "" => return Err(AcceleratorError::KeyCount),
                _ => {
                    if key.is_some() {
                        return Err(AcceleratorError::KeyCount);
                    }
                    key = Some(
                        Key::from_name(part)
                            .ok_or_else(|| AcceleratorError::UnknownKey(part.to_string()))?,
                    );
                }
            }
        }
        let key = key.ok_or(AcceleratorError::KeyCount)?;
        Ok(Self {
            ctrl,
            shift,
            alt,
            logo,
            key,
        })
    }
}

#[cfg(test)]
#[path = "accelerator_tests.rs"]
mod tests;
