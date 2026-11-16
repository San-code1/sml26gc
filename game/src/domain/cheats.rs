//! The debug / cheats mode: which cheats there are, which of them stay on, and the info
//! panel drawn over the picture. What each one does to the game is `cart_model::cheats`.
use super::overlay::{text_width, Canvas};
use cart_model::cheats::Info;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cheat {
    Invincible,
    FinishLevel,
    SecretExit,
    Summon,
    FreezeTimer,
    MaxLives,
    MaxCoins,
    GoldenCoins,
    Info,
}

impl Cheat {
    /// In the menu's order.
    pub const ALL: [Cheat; 9] = [Cheat::Invincible, Cheat::FinishLevel, Cheat::SecretExit, Cheat::Summon, Cheat::FreezeTimer, Cheat::MaxLives, Cheat::MaxCoins, Cheat::GoldenCoins, Cheat::Info];

    /// Only usable while playing a level.
    pub const fn needs_level(self) -> bool {
        matches!(self, Cheat::FinishLevel | Cheat::SecretExit | Cheat::Summon)
    }

    /// Turned on and off (the others happen once when chosen).
    pub const fn toggles(self) -> bool {
        matches!(self, Cheat::Invincible | Cheat::FreezeTimer | Cheat::Info)
    }
}

/// The cheats that stay on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cheats {
    pub invincible: bool,
    pub freeze_timer: bool,
    pub info: bool,
}

impl Cheats {
    pub fn is_on(&self, c: Cheat) -> bool {
        match c {
            Cheat::Invincible => self.invincible,
            Cheat::FreezeTimer => self.freeze_timer,
            Cheat::Info => self.info,
            _ => false,
        }
    }

    pub fn toggle(&mut self, c: Cheat) {
        match c {
            Cheat::Invincible => self.invincible = !self.invincible,
            Cheat::FreezeTimer => self.freeze_timer = !self.freeze_timer,
            Cheat::Info => self.info = !self.info,
            _ => {}
        }
    }
}

/// A 16x16 item picture in the screen's shades, `None` where it is see-through.
pub type Icon = [Option<u8>; 256];

/// `L05 X0123 Y0045 P07 M04` (hexadecimal) in a dark tag at the top left.
pub fn draw_info(px: &mut [u8], info: &Info) {
    let text = format!("L{:02X} X{:04X} Y{:04X} P{:02X} M{:02X}", info.level, info.x, info.y, info.point, info.mode);
    let mut c = Canvas { px };
    c.rect(1, 1, text_width(&text) + 5, 11, 3);
    c.text(3, 3, &text, 0);
}

pub fn draw_icon(c: &mut Canvas, x: i32, y: i32, icon: &Icon) {
    for (i, s) in icon.iter().enumerate() {
        if let Some(s) = s {
            c.rect(x + (i % 16) as i32, y + (i / 16) as i32, 1, 1, *s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_lasting_cheats_toggle() {
        let mut c = Cheats::default();
        for cheat in Cheat::ALL {
            c.toggle(cheat);
            assert_eq!(c.is_on(cheat), cheat.toggles());
        }
        c.toggle(Cheat::Invincible);
        assert!(!c.invincible && c.freeze_timer && c.info);
    }

    #[test]
    fn info_panel_is_drawn_top_left() {
        let mut px = vec![0u8; gb_core::ppu::W * gb_core::ppu::H];
        draw_info(&mut px, &Info { level: 5, x: 0x123, y: 0x45, point: 7, mode: 4 });
        assert_eq!(px[gb_core::ppu::W + 1], 3);
        assert_eq!(px[gb_core::ppu::W * 50 + 50], 0);
    }
}
