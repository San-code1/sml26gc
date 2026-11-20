//! Keyboard keys as the game knows them, independent of any window library. Bindable keys
//! carry their name in `settings.cfg` and their label in the menu (English here; the other
//! languages' in `game/i18n/keys.txt`).
use crate::ports::Keyboard;
use gb_core::Buttons;

macro_rules! keys {
    ($($k:ident $id:literal $en:literal;)*) => {
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum Key {
            $($k,)*
            /// Not bindable: full screen on/off.
            F11,
        }

        /// Keys that can be bound: key, file id, English label.
        pub const BINDABLE: &[(Key, &str, &str)] = &[$((Key::$k, $id, $en),)*];
    };
}

keys! {
    Up "Up" "UP ARROW";
    Down "Down" "DOWN ARROW";
    Left "Left" "LEFT ARROW";
    Right "Right" "RIGHT ARROW";
    Space "Space" "SPACE";
    Enter "Enter" "ENTER";
    KpEnter "KpEnter" "NUM ENTER";
    Backspace "Backspace" "BACKSPACE";
    Tab "Tab" "TAB";
    LeftShift "LeftShift" "L SHIFT";
    RightShift "RightShift" "R SHIFT";
    LeftControl "LeftControl" "L CTRL";
    RightControl "RightControl" "R CTRL";
    LeftAlt "LeftAlt" "ALT";
    A "A" "A";
    B "B" "B";
    C "C" "C";
    D "D" "D";
    E "E" "E";
    F "F" "F";
    G "G" "G";
    H "H" "H";
    I "I" "I";
    J "J" "J";
    K "K" "K";
    L "L" "L";
    M "M" "M";
    N "N" "N";
    O "O" "O";
    P "P" "P";
    Q "Q" "Q";
    R "R" "R";
    S "S" "S";
    T "T" "T";
    U "U" "U";
    V "V" "V";
    W "W" "W";
    X "X" "X";
    Y "Y" "Y";
    Z "Z" "Z";
    Key0 "Key0" "0";
    Key1 "Key1" "1";
    Key2 "Key2" "2";
    Key3 "Key3" "3";
    Key4 "Key4" "4";
    Key5 "Key5" "5";
    Key6 "Key6" "6";
    Key7 "Key7" "7";
    Key8 "Key8" "8";
    Key9 "Key9" "9";
    Kp0 "Kp0" "NUM 0";
    Kp1 "Kp1" "NUM 1";
    Kp2 "Kp2" "NUM 2";
    Kp3 "Kp3" "NUM 3";
    Kp4 "Kp4" "NUM 4";
    Kp5 "Kp5" "NUM 5";
    Kp6 "Kp6" "NUM 6";
    Kp7 "Kp7" "NUM 7";
    Kp8 "Kp8" "NUM 8";
    Kp9 "Kp9" "NUM 9";
    Comma "Comma" "COMMA";
    Period "Period" "PERIOD";
    Minus "Minus" "MINUS";
    Slash "Slash" "SLASH";
    F1 "F1" "F1";
    F2 "F2" "F2";
    F3 "F3" "F3";
    F4 "F4" "F4";
    F5 "F5" "F5";
    F6 "F6" "F6";
    F7 "F7" "F7";
    F8 "F8" "F8";
    F9 "F9" "F9";
    F10 "F10" "F10";
    Escape "Escape" "ESC";
}

/// The bindings in order: the Game Boy buttons (Up, Down, Left, Right, A, B, Start, Select),
/// then the port's own save state, load state and rewind (cheats mode), fast-forward, the
/// pause menu and restart level.
pub const UP: usize = 0;
pub const DOWN: usize = 1;
pub const LEFT: usize = 2;
pub const RIGHT: usize = 3;
pub const A: usize = 4;
pub const B: usize = 5;
pub const START: usize = 6;
#[allow(dead_code)] // named for completeness: Select is never read on its own
pub const SELECT: usize = 7;
pub const SAVE_STATE: usize = 8;
pub const LOAD_STATE: usize = 9;
pub const REWIND: usize = 10;
/// Held: the game runs faster.
pub const FAST_FORWARD: usize = 11;
/// Opens the pause menu anywhere and goes back in it.
pub const MENU: usize = 12;
/// Restarts the level being played (cheats mode or a test game).
pub const RESTART: usize = 13;
/// Game Boy buttons, then all bindings.
pub const PAD_KEYS: usize = 8;
pub const KEYS: usize = 14;

pub const DEFAULT_KEYS: [Key; KEYS] = [Key::Up, Key::Down, Key::Left, Key::Right, Key::X, Key::Z, Key::Enter, Key::Backspace, Key::F6, Key::F7, Key::R, Key::LeftControl, Key::Escape, Key::F8];

impl Key {
    pub fn bindable(self) -> bool {
        BINDABLE.iter().any(|e| e.0 == self)
    }

    /// The name written in `settings.cfg`.
    pub fn id(self) -> Option<&'static str> {
        BINDABLE.iter().find(|e| e.0 == self).map(|e| e.1)
    }

    pub fn from_id(id: &str) -> Option<Key> {
        BINDABLE.iter().find(|e| e.1 == id).map(|e| e.0)
    }

    /// The name shown in the menu.
    pub fn label(self, lang: usize) -> &'static str {
        BINDABLE.iter().find(|e| e.0 == self).map(|e| super::i18n::key_label(lang, e.2)).unwrap_or("?")
    }
}

/// The Game Boy buttons of the first `PAD_KEYS` bindings, in order.
pub const PAD_BUTTONS: [Buttons; PAD_KEYS] = [Buttons::UP, Buttons::DOWN, Buttons::LEFT, Buttons::RIGHT, Buttons::A, Buttons::B, Buttons::START, Buttons::SELECT];

/// The Game Boy buttons held now, through the player's bindings.
pub fn buttons(keys: &dyn Keyboard, bindings: &[Key; KEYS]) -> Buttons {
    let mut b = Buttons::NONE;
    for (&k, bit) in bindings.iter().zip(PAD_BUTTONS) {
        if keys.held(k) {
            b |= bit;
        }
    }
    no_opposites(b)
}

/// A cartridge never sees opposite directions held together.
pub fn no_opposites(mut b: Buttons) -> Buttons {
    if b & (Buttons::LEFT | Buttons::RIGHT) == Buttons::LEFT | Buttons::RIGHT {
        b &= !(Buttons::LEFT | Buttons::RIGHT);
    }
    if b & (Buttons::UP | Buttons::DOWN) == Buttons::UP | Buttons::DOWN {
        b &= !(Buttons::UP | Buttons::DOWN);
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Held(Vec<Key>);
    impl Keyboard for Held {
        fn pressed(&self, _: Key) -> bool {
            false
        }
        fn held(&self, k: Key) -> bool {
            self.0.contains(&k)
        }
        fn last_pressed(&self) -> Option<Key> {
            None
        }
    }

    #[test]
    fn opposite_directions_cancel() {
        let b = buttons(&Held(vec![Key::Left, Key::Right, Key::Up, Key::X]), &DEFAULT_KEYS);
        assert_eq!(b, Buttons::UP | Buttons::A);
    }

    #[test]
    fn ids_round_trip() {
        for &(k, id, _) in BINDABLE {
            assert_eq!(Key::from_id(id), Some(k));
            assert_eq!(k.id(), Some(id));
        }
        assert!(Key::Escape.bindable() && !Key::F11.bindable());
        assert_eq!(Key::LeftShift.label(super::super::i18n::LANG_ES), "MAYUS IZQ");
    }
}
