//! Driving this cartridge's program from outside: reading which screen it is on, and the
//! button sequences and memory writes that take it to the map, into a level or back.
use gb_core::Buttons;
use gb_core::Gb;

/// The game-specific actions on a running Game Boy with this cartridge.
pub trait GameDriver {
    /// Game mode byte ($FF9B): $1A file select, $0C map, $04 playing a level.
    fn mode(&mut self) -> u8;
    /// Leave the level for the map exactly as the game's own pause menu does when a cleared
    /// level is exited with Select (0:2774-0:278B): no life is lost.
    fn back_to_map(&mut self);
    /// Start the cartridge again (like switching the console off and on: battery saves stay)
    /// and press Start on the title screen until the A/B/C file select is up.
    fn back_to_file_select(&mut self);
    /// From power-on, press Start through the title and file A until the world map is up.
    fn boot_to_map(&mut self) -> bool;
    /// From the world map, enter level `level` (0..=31): press Start, and when the game
    /// copies the level's header (mode $0B, routine 01:5579 reads the level number at
    /// $A269) put the wanted number there first. Returns once the level is playing.
    fn enter_level(&mut self, level: u8) -> bool;
    /// Enter `level` from the world map whatever point the game's own Mario stands on:
    /// Start only works on a point that opens a level (0F:41B1), so first put him on one
    /// ($A840 = point number; the special points 0F:418F-0F:41A1 checks are avoided).
    fn enter_level_anywhere(&mut self, level: u8) -> bool;
}

impl GameDriver for Gb {
    fn mode(&mut self) -> u8 {
        self.read(0xFF9B)
    }

    fn back_to_map(&mut self) {
        for (addr, v) in [(0xA45E, 0x02), (0xA224, 0x02), (0xA2C8, 0), (0xA299, 0), (0xA2B4, 0), (0xA50D, 0)] {
            self.write(addr, v);
        }
        self.write(0xFF9B, 0x14);
    }

    fn back_to_file_select(&mut self) {
        let ram = self.cart_mut().take_ram();
        let rom = self.cart().rom().to_vec();
        let rate = self.sample_rate();
        if let Ok(mut g) = Gb::new(rom, rate) {
            g.cart_mut().set_ram(ram);
            for f in 0..1200u32 {
                g.run_frame_with(if f % 40 < 4 && f > 60 { Buttons::START } else { Buttons::NONE });
                if g.mode() == 0x1A {
                    break;
                }
            }
            g.set_buttons(Buttons::NONE);
            *self = g;
        } else {
            self.cart_mut().set_ram(ram);
        }
    }

    fn boot_to_map(&mut self) -> bool {
        for f in 0..3000u32 {
            let m = self.mode();
            if m == 0x0C {
                // let the map finish appearing
                for _ in 0..90 {
                    self.run_frame_with(Buttons::NONE);
                }
                return true;
            }
            self.run_frame_with(if f % 50 < 5 && f > 120 { Buttons::START } else { Buttons::NONE });
        }
        false
    }

    fn enter_level(&mut self, level: u8) -> bool {
        let mut set = false;
        for f in 0..900u32 {
            let m = self.mode();
            if m == 0x0B && !set {
                self.write(0xA269, level);
                set = true;
            }
            if set && m == 0x04 {
                for _ in 0..3 {
                    self.run_frame_with(Buttons::NONE);
                }
                return true;
            }
            let press = !set && m == 0x0C && f % 30 < 4;
            self.run_frame_with(if press { Buttons::START } else { Buttons::NONE });
        }
        false
    }

    fn enter_level_anywhere(&mut self, level: u8) -> bool {
        let rom = self.cart().rom().to_vec();
        let table = crate::world::point_levels(&rom);
        let special = [0x05, 0x09, 0x0E, 0x17, 0x1D, 0x11];
        if let Some(p) = (0..table.len()).find(|&p| table[p] < 32 && !special.contains(&(p as u8))) {
            self.write(0xA840, p as u8);
        }
        self.enter_level(level)
    }
}
