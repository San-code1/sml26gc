//! What every command needs: the cartridge on a Game Boy, a frame of a button script,
//! snapshots of the game's RAM, pokes, and printing results the way the tools always did.
pub use crate::probe::probed;
use gb_core::script::buttons_at;
use gb_core::{BusObserver, Buttons, GameBoy, Gb};
use std::path::Path;

static WITH_SAVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// `--with-save`: start from the player's battery save (read only; nothing is written back).
pub fn use_battery_save(on: bool) {
    WITH_SAVE.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// The cartridge on a Game Boy (with the player's save only under `--with-save`), or None
/// if it can't be loaded.
pub fn load(rom: &Path) -> Option<Gb> {
    host::load_gb(rom, 44100, WITH_SAVE.load(std::sync::atomic::Ordering::Relaxed)).ok()
}

/// Play frame `f` of a button script (sound dropped).
pub fn run_script<O: BusObserver>(gb: &mut GameBoy<O>, sc: &[(Buttons, u32, u32)], f: u32) {
    gb.set_buttons(buttons_at(sc, f));
    gb.run_frame();
    gb.clear_audio();
}

/// The memory the game keeps its state in: cartridge RAM, work RAM and high RAM.
pub const RAM_RANGES: [(u16, u16); 3] = [(0xA000, 0xBFFF), (0xC000, 0xDFFF), (0xFF80, 0xFFFE)];

pub fn ram_addresses() -> Vec<u16> {
    RAM_RANGES.iter().flat_map(|&(a, b)| a..=b).collect()
}

/// (address, value) for every byte of `RAM_RANGES`.
pub fn ram_snapshot(gb: &mut Gb) -> Vec<(u16, u8)> {
    ram_addresses().into_iter().map(|a| (a, gb.read(a))).collect()
}

/// Write `pokes` ("a299=00,ff9b=14", hex) into memory.
pub fn apply_pokes<O: BusObserver>(gb: &mut GameBoy<O>, pokes: &str) {
    for p in pokes.split(',') {
        if let Some((a, v)) = p.split_once('=') {
            gb.write(u16::from_str_radix(a, 16).unwrap_or(0), u8::from_str_radix(v, 16).unwrap_or(0));
        }
    }
}

/// A save result printed the way the tools always printed it: `Ok("...")` / `Err("...")`.
pub fn shown<T: std::fmt::Display, E: std::fmt::Display>(r: &Result<T, E>) -> String {
    match r {
        Ok(v) => format!("Ok({:?})", v.to_string()),
        Err(e) => format!("Err({:?})", e.to_string()),
    }
}
