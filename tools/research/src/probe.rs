//! A Game Boy on the bus with a `Probe`: logs writes to chosen addresses and data reads
//! from a ROM range, each with the instruction that made it.
use gb_core::{BusObserver, GameBoy};
use std::path::Path;

/// Watches memory for `watch` and `reads`: writes to chosen addresses, and data reads from a
/// ROM range (instruction bytes are left out), each with the instruction that made it.
#[derive(Clone, Default)]
pub struct Probe {
    pub watch: Vec<u16>,
    pub watch_log: Vec<(u16, usize, u16, u8)>, // (pc, bank, addr, value)
    /// (first address, last address, ROM bank or $FF for any)
    pub read_watch: Option<(u16, u16, usize)>,
    pub read_log: Vec<(u16, u16, u8)>, // (pc, addr, bank)
}

impl BusObserver for Probe {
    const ENABLED: bool = true;

    fn on_read(&mut self, pc: u16, bank: usize, addr: u16) {
        if let Some((lo, hi, want)) = self.read_watch {
            let fetch = addr >= pc && addr < pc.wrapping_add(3);
            if addr >= lo && addr <= hi && (want == 0xFF || bank == want) && !fetch && self.read_log.len() < 200_000 {
                self.read_log.push((pc, addr, bank as u8));
            }
        }
    }

    fn on_write(&mut self, pc: u16, bank: usize, addr: u16, value: u8) {
        if !self.watch.is_empty() && self.watch.contains(&addr) && self.watch_log.len() < 100_000 {
            self.watch_log.push((pc, bank, addr, value));
        }
    }
}

/// The cartridge on a Game Boy with a `Probe` on its bus (no battery save loaded).
pub fn probed(rom: &Path) -> Option<GameBoy<Probe>> {
    let data = std::fs::read(rom).map_err(|e| eprintln!("{e}")).ok()?;
    GameBoy::with_observer(data, 44100, Probe::default()).map_err(|e| eprintln!("{e}")).ok()
}
