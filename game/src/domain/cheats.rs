//! The debug / cheats mode: which cheats there are, which of them stay on, and the info
//! panel drawn over the picture. What each one does to the game is `cart_model::cheats`.
use super::overlay::{text_width, Canvas};
use cart_model::cheats::{Active, Info};
use cart_model::level_start::{FEET, SPRITE_W};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cheat {
    Invincible,
    FinishLevel,
    SecretExit,
    GoToLevel,
    Summon,
    FreezeTimer,
    MaxLives,
    MaxCoins,
    GoldenCoins,
    OpenMap,
    Info,
    ObjectsView,
}

impl Cheat {
    /// In the menu's order.
    pub const ALL: [Cheat; 12] = [Cheat::Invincible, Cheat::FinishLevel, Cheat::SecretExit, Cheat::GoToLevel, Cheat::Summon, Cheat::FreezeTimer, Cheat::MaxLives, Cheat::MaxCoins, Cheat::GoldenCoins, Cheat::OpenMap, Cheat::Info, Cheat::ObjectsView];

    /// Only usable while playing a level.
    pub const fn needs_level(self) -> bool {
        matches!(self, Cheat::FinishLevel | Cheat::SecretExit | Cheat::Summon)
    }

    /// Turned on and off (the others happen once when chosen).
    pub const fn toggles(self) -> bool {
        matches!(self, Cheat::Invincible | Cheat::FreezeTimer | Cheat::Info | Cheat::ObjectsView)
    }
}

/// The cheats that stay on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cheats {
    pub invincible: bool,
    pub freeze_timer: bool,
    pub info: bool,
    pub objects: bool,
}

impl Cheats {
    pub fn is_on(&self, c: Cheat) -> bool {
        match c {
            Cheat::Invincible => self.invincible,
            Cheat::FreezeTimer => self.freeze_timer,
            Cheat::Info => self.info,
            Cheat::ObjectsView => self.objects,
            _ => false,
        }
    }

    pub fn toggle(&mut self, c: Cheat) {
        match c {
            Cheat::Invincible => self.invincible = !self.invincible,
            Cheat::FreezeTimer => self.freeze_timer = !self.freeze_timer,
            Cheat::Info => self.info = !self.info,
            Cheat::ObjectsView => self.objects = !self.objects,
            _ => {}
        }
    }
}

/// A 16x16 item picture in the screen's shades, `None` where it is see-through.
pub type Icon = [Option<u8>; 256];

/// `L05 X0123 Y0045 P07 M04` (hexadecimal) in a dark tag at the top left, and under it
/// `EDITOR 14,30`: the column and row of the block Mario stands on, as the level editor counts
/// them (the block Play here puts his feet on).
pub fn draw_info(px: &mut [u8], info: &Info) {
    let text = format!("L{:02X} X{:04X} Y{:04X} P{:02X} M{:02X}", info.level, info.x, info.y, info.point, info.mode);
    let (col, row) = editor_block(info);
    let block = format!("EDITOR {col},{row}");
    let mut c = Canvas { px };
    c.rect(1, 1, text_width(&text) + 5, 11, 3);
    c.text(3, 3, &text, 0);
    c.rect(1, 12, text_width(&block) + 5, 10, 3);
    c.text(3, 13, &block, 0);
}

/// The block under Mario's feet, below his middle: (column, row).
fn editor_block(info: &Info) -> (u16, u16) {
    ((info.x + SPRITE_W / 2) / 16, (info.y + FEET) / 16)
}

/// Each active object's type (hexadecimal) in a small tag above it.
pub fn draw_objects(px: &mut [u8], objects: &[Active]) {
    let mut c = Canvas { px };
    for o in objects {
        let t = format!("{:02X}", o.kind);
        let (x, y) = (o.x, o.y - 10);
        c.rect(x - 1, y - 1, text_width(&t) + 2, 9, 3);
        c.text(x, y, &t, 0);
    }
}

/// A short note at the bottom left (save states).
pub fn draw_note(px: &mut [u8], note: &str) {
    let mut c = Canvas { px };
    let y = gb_core::ppu::H as i32 - 12;
    c.rect(1, y, text_width(note) + 5, 11, 3);
    c.text(3, y + 2, note, 0);
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
        assert!(!c.invincible && c.freeze_timer && c.info && c.objects);
    }

    #[test]
    fn info_panel_is_drawn_top_left() {
        let mut px = vec![0u8; gb_core::ppu::W * gb_core::ppu::H];
        draw_info(&mut px, &Info { level: 5, x: 0x123, y: 0x45, point: 7, mode: 4, objects: 3 });
        let mut tags = vec![0u8; gb_core::ppu::W * gb_core::ppu::H];
        draw_objects(&mut tags, &[Active { kind: 0x1F, x: 40, y: 60 }]);
        assert_eq!(tags[gb_core::ppu::W * 49 + 39], 3);
        assert_eq!(px[gb_core::ppu::W + 1], 3);
        assert_eq!(px[gb_core::ppu::W * 50 + 50], 0);
    }
}
