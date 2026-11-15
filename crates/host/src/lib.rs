//! The computer side shared by the game, the editor and the tools: where the cartridge is,
//! where saves go, loading a cartridge with its battery save, writing PNG pictures.
pub mod png_out;

use cart_model::custom_map::CustomMap;
use gb_core::Gb;
use std::path::{Path, PathBuf};

fn is_rom(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("gb") || e.eq_ignore_ascii_case("gbc"))
        .unwrap_or(false)
}

fn first_rom_in(dir: &Path) -> Option<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).filter(|p| is_rom(p)).collect();
    v.sort();
    v.into_iter().next()
}

/// Where to find the cartridge: the command line, a `rom` folder beside the
/// program (or its parent folders), then the person's Games folder.
pub fn find_rom(arg: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(p) = arg {
        if p.is_dir() {
            return first_rom_in(&p);
        }
        return Some(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut d = exe.parent().map(|p| p.to_path_buf());
        for _ in 0..5 {
            let Some(dir) = d else { break };
            for sub in ["rom", "roms", "."] {
                if let Some(r) = first_rom_in(&dir.join(sub)) {
                    return Some(r);
                }
            }
            d = dir.parent().map(|p| p.to_path_buf());
        }
    }
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).ok()?;
    let games = Path::new(&home).join("Games");
    if let Some(r) = first_rom_in(&games.join("Super Mario Land 2 - 6 Golden Coins (USA, Europe)")) {
        return Some(r);
    }
    for e in std::fs::read_dir(&games).ok()?.flatten() {
        if e.path().is_dir() {
            if let Some(r) = first_rom_in(&e.path()) {
                return Some(r);
            }
        } else if is_rom(&e.path()) {
            return Some(e.path());
        }
    }
    None
}

/// The folder the running program sits in.
pub fn exe_dir() -> PathBuf {
    std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())).unwrap_or_else(|| PathBuf::from("."))
}

pub fn save_path(rom: &Path) -> PathBuf {
    let dir = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.join("saves")))
        .unwrap_or_else(|| PathBuf::from("saves"));
    let name = rom.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "game".into());
    dir.join(format!("{name}.sav"))
}

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

/// The editor-made world map file that goes with a cartridge file (`<name>.points.txt`).
pub fn custom_map_path(rom: &Path) -> PathBuf {
    rom.with_extension("points.txt")
}

/// The editor-made world map for this cartridge file, if it has one with points.
pub fn load_custom_map(rom: &Path) -> Option<CustomMap> {
    let t = std::fs::read_to_string(custom_map_path(rom)).ok()?;
    let m = CustomMap::parse(&t);
    if m.points.is_empty() {
        None
    } else {
        Some(m)
    }
}

/// Save the map next to the cartridge file; a map with no points removes the file (the
/// game then uses its own map).
pub fn save_custom_map(map: &CustomMap, rom: &Path) -> std::io::Result<()> {
    let path = custom_map_path(rom);
    if map.points.is_empty() {
        return std::fs::remove_file(path).or(Ok(()));
    }
    std::fs::write(path, map.to_text())
}
