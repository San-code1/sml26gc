//! Keyboard keys as the game knows them, independent of any window library. Bindable keys
//! carry their name in `settings.cfg` and their label in the menu (Spanish, English; the
//! Japanese menu uses the English label).
use crate::ports::Keyboard;
use gb_core::Buttons;

macro_rules! keys {
    ($($k:ident $id:literal $es:literal $en:literal;)*) => {
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum Key {
            $($k,)*
            /// Not bindable: opens and leaves the menu.
            Escape,
            /// Not bindable: full screen on/off.
            F11,
        }

        /// Keys that can be bound: key, file id, Spanish label, English label.
        pub const BINDABLE: &[(Key, &str, &str, &str)] = &[$((Key::$k, $id, $es, $en),)*];
    };
}

keys! {
    Up "Up" "FLECHA ARR" "UP ARROW";
    Down "Down" "FLECHA ABA" "DOWN ARROW";
    Left "Left" "FLECHA IZQ" "LEFT ARROW";
    Right "Right" "FLECHA DER" "RIGHT ARROW";
    Space "Space" "ESPACIO" "SPACE";
    Enter "Enter" "INTRO" "ENTER";
    KpEnter "KpEnter" "INTRO NUM" "NUM ENTER";
    Backspace "Backspace" "RETROCESO" "BACKSPACE";
    Tab "Tab" "TAB" "TAB";
    LeftShift "LeftShift" "MAYUS IZQ" "L SHIFT";
    RightShift "RightShift" "MAYUS DER" "R SHIFT";
    LeftControl "LeftControl" "CTRL IZQ" "L CTRL";
    RightControl "RightControl" "CTRL DER" "R CTRL";
    LeftAlt "LeftAlt" "ALT" "ALT";
    A "A" "A" "A";
    B "B" "B" "B";
    C "C" "C" "C";
    D "D" "D" "D";
    E "E" "E" "E";
    F "F" "F" "F";
    G "G" "G" "G";
    H "H" "H" "H";
    I "I" "I" "I";
    J "J" "J" "J";
    K "K" "K" "K";
    L "L" "L" "L";
    M "M" "M" "M";
    N "N" "N" "N";
    O "O" "O" "O";
    P "P" "P" "P";
    Q "Q" "Q" "Q";
    R "R" "R" "R";
    S "S" "S" "S";
    T "T" "T" "T";
    U "U" "U" "U";
    V "V" "V" "V";
    W "W" "W" "W";
    X "X" "X" "X";
    Y "Y" "Y" "Y";
    Z "Z" "Z" "Z";
    Key0 "Key0" "0" "0";
    Key1 "Key1" "1" "1";
    Key2 "Key2" "2" "2";
    Key3 "Key3" "3" "3";
    Key4 "Key4" "4" "4";
    Key5 "Key5" "5" "5";
    Key6 "Key6" "6" "6";
    Key7 "Key7" "7" "7";
    Key8 "Key8" "8" "8";
    Key9 "Key9" "9" "9";
    Kp0 "Kp0" "NUM 0" "NUM 0";
    Kp1 "Kp1" "NUM 1" "NUM 1";
    Kp2 "Kp2" "NUM 2" "NUM 2";
    Kp3 "Kp3" "NUM 3" "NUM 3";
    Kp4 "Kp4" "NUM 4" "NUM 4";
    Kp5 "Kp5" "NUM 5" "NUM 5";
    Kp6 "Kp6" "NUM 6" "NUM 6";
    Kp7 "Kp7" "NUM 7" "NUM 7";
    Kp8 "Kp8" "NUM 8" "NUM 8";
    Kp9 "Kp9" "NUM 9" "NUM 9";
    Comma "Comma" "COMA" "COMMA";
    Period "Period" "PUNTO" "PERIOD";
    Minus "Minus" "MENOS" "MINUS";
    Slash "Slash" "BARRA" "SLASH";
    F1 "F1" "F1" "F1";
    F2 "F2" "F2" "F2";
    F3 "F3" "F3" "F3";
    F4 "F4" "F4" "F4";
    F5 "F5" "F5" "F5";
    F6 "F6" "F6" "F6";
    F7 "F7" "F7" "F7";
    F8 "F8" "F8" "F8";
    F9 "F9" "F9" "F9";
    F10 "F10" "F10" "F10";
}

/// The bindings in order: the Game Boy buttons (Up, Down, Left, Right, A, B, Start, Select),
/// then the port's own save state, load state and rewind (cheats mode).
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
/// Game Boy buttons, then all bindings.
pub const PAD_KEYS: usize = 8;
pub const KEYS: usize = 11;

pub const DEFAULT_KEYS: [Key; KEYS] = [Key::Up, Key::Down, Key::Left, Key::Right, Key::X, Key::Z, Key::Enter, Key::Backspace, Key::F6, Key::F7, Key::R];

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
        BINDABLE
            .iter()
            .find(|e| e.0 == self)
            .map(|e| if lang == super::i18n::LANG_ES { e.2 } else { e.3 })
            .unwrap_or("?")
    }
}

/// The Game Boy buttons held now, through the player's bindings.
pub fn buttons(keys: &dyn Keyboard, bindings: &[Key; KEYS]) -> Buttons {
    const BITS: [Buttons; PAD_KEYS] = [Buttons::UP, Buttons::DOWN, Buttons::LEFT, Buttons::RIGHT, Buttons::A, Buttons::B, Buttons::START, Buttons::SELECT];
    let mut b = Buttons::NONE;
    for (&k, bit) in bindings.iter().zip(BITS) {
        if keys.held(k) {
            b |= bit;
        }
    }
    // A cartridge never sees opposite directions held together.
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
        for &(k, id, _, _) in BINDABLE {
            assert_eq!(Key::from_id(id), Some(k));
            assert_eq!(k.id(), Some(id));
        }
        assert!(!Key::Escape.bindable());
        assert_eq!(Key::LeftShift.label(super::super::i18n::LANG_ES), "MAYUS IZQ");
    }
}
