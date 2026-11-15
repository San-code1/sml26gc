//! Loading a cartridge file with its battery save, and writing the save back.
use crate::paths::save_path;
use gb_core::Gb;
use std::path::Path;

/// Power on a Game Boy with this cartridge file; with `load_save`, its battery RAM comes
/// from the save file (if there is one).
pub fn load_gb(rom_path: &Path, sample_rate: u32, load_save: bool) -> Result<Gb, String> {
    let rom = std::fs::read(rom_path).map_err(|e| format!("could not read {}: {e}", rom_path.display()))?;
    let mut gb = Gb::new(rom, sample_rate)?;
    if load_save && gb.cart().has_battery() {
        if let Ok(data) = std::fs::read(save_path(rom_path)) {
            let ram = gb.cart_mut().ram_mut();
            let n = data.len().min(ram.len());
            ram[..n].copy_from_slice(&data[..n]);
        }
    }
    Ok(gb)
}

/// Write the battery RAM to the save file if the game changed it since the last write.
/// Returns whether it wrote.
pub fn write_battery_save(gb: &mut Gb, rom_path: &Path) -> std::io::Result<bool> {
    if !gb.cart().has_battery() || !gb.cart().needs_saving() {
        return Ok(false);
    }
    let p = save_path(rom_path);
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::write(&p, gb.cart().ram())?;
    gb.cart_mut().mark_saved();
    Ok(true)
}
