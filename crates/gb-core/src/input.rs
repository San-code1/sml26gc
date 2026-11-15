//! The eight Game Boy buttons as a set.
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Buttons(u8);

impl Buttons {
    pub const NONE: Buttons = Buttons(0);
    pub const RIGHT: Buttons = Buttons(0x01);
    pub const LEFT: Buttons = Buttons(0x02);
    pub const UP: Buttons = Buttons(0x04);
    pub const DOWN: Buttons = Buttons(0x08);
    pub const A: Buttons = Buttons(0x10);
    pub const B: Buttons = Buttons(0x20);
    pub const SELECT: Buttons = Buttons(0x40);
    pub const START: Buttons = Buttons(0x80);

    /// Bit layout: d-pad in bits 0-3 (Right, Left, Up, Down), A, B, Select, Start in 4-7.
    pub const fn from_bits(bits: u8) -> Buttons {
        Buttons(bits)
    }
    pub const fn bits(self) -> u8 {
        self.0
    }
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
    pub const fn contains(self, other: Buttons) -> bool {
        self.0 & other.0 == other.0
    }
    pub const fn intersects(self, other: Buttons) -> bool {
        self.0 & other.0 != 0
    }
    /// Pressed in `self` but not in `before`.
    pub const fn newly_pressed(self, before: Buttons) -> Buttons {
        Buttons(self.0 & !before.0)
    }
}

impl BitOr for Buttons {
    type Output = Buttons;
    fn bitor(self, o: Buttons) -> Buttons {
        Buttons(self.0 | o.0)
    }
}
impl BitOrAssign for Buttons {
    fn bitor_assign(&mut self, o: Buttons) {
        self.0 |= o.0;
    }
}
impl BitAnd for Buttons {
    type Output = Buttons;
    fn bitand(self, o: Buttons) -> Buttons {
        Buttons(self.0 & o.0)
    }
}
impl BitAndAssign for Buttons {
    fn bitand_assign(&mut self, o: Buttons) {
        self.0 &= o.0;
    }
}
impl Not for Buttons {
    type Output = Buttons;
    fn not(self) -> Buttons {
        Buttons(!self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_operations() {
        let b = Buttons::RIGHT | Buttons::A;
        assert!(b.contains(Buttons::A) && !b.contains(Buttons::B));
        assert_eq!(b & !Buttons::RIGHT, Buttons::A);
        assert_eq!(
            (Buttons::A | Buttons::B).newly_pressed(Buttons::A),
            Buttons::B
        );
        assert_eq!(b.bits(), 0x11);
        assert!(Buttons::NONE.is_empty());
    }
}
