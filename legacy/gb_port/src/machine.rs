//! The Game Boy as a whole (processor + memory bus), finding the cartridge file and
//! loading it with its battery save. Shared by the game and the level editor.
use crate::bus::{self, Bus};
use crate::{cart, cpu};
use std::path::{Path, PathBuf};

pub const FRAME_CYCLES: u32 = 70224;
pub const FRAME_SECONDS: f64 = FRAME_CYCLES as f64 / 4_194_304.0; // 59.73 Hz

#[derive(Clone)]
pub struct Gb {
    pub cpu: cpu::Cpu,
    pub bus: Bus,
}

impl Gb {
    pub fn new(rom: Vec<u8>, sample_rate: u32) -> Result<Gb, String> {
        let cart = cart::Cart::new(rom)?;
        Ok(Gb { cpu: cpu::Cpu::new(), bus: Bus::new(cart, sample_rate) })
    }

    /// Run until the next vertical blank (one video frame).
    pub fn run_frame(&mut self) {
        let mut total = 0u32;
        loop {
            let c = self.cpu.step(&mut self.bus);
            self.bus.tick(c);
            total += c;
            if self.bus.ppu.frame_ready {
                self.bus.ppu.frame_ready = false;
                break;
            }
            if total >= FRAME_CYCLES * 2 || (!self.bus.ppu.lcd_on() && total >= FRAME_CYCLES) {
                break;
            }
        }
    }

    /// Run one frame with these buttons held, throwing the sound away.
    pub fn run_frame_with(&mut self, buttons: u8) {
        self.bus.set_buttons(buttons);
        self.run_frame();
        self.bus.apu.out.clear();
    }

    /// Game mode byte ($FF9B): $1A file select, $0C map, $04 playing a level.
    pub fn mode(&mut self) -> u8 {
        self.bus.read(0xFF9B)
    }

    /// Leave the level for the map exactly as the game's own pause menu does when a cleared
    /// level is exited with Select (0:2774-0:278B): no life is lost.
    pub fn back_to_map(&mut self) {
        for (addr, v) in [(0xA45E, 0x02), (0xA224, 0x02), (0xA2C8, 0), (0xA299, 0), (0xA2B4, 0), (0xA50D, 0)] {
            self.bus.write(addr, v);
        }
        self.bus.write(0xFF9B, 0x14);
    }

    /// Start the cartridge again (like switching the console off and on: battery saves stay)
    /// and press Start on the title screen until the A/B/C file select is up.
    pub fn back_to_file_select(&mut self) {
        let ram = std::mem::take(&mut self.bus.cart.ram);
        let rom = self.bus.cart.rom_copy();
        let rate = self.bus.apu.sample_rate();
        if let Ok(mut g) = Gb::new(rom, rate) {
            g.bus.cart.ram = ram;
            for f in 0..1200u32 {
                g.run_frame_with(if f % 40 < 4 && f > 60 { bus::BTN_START } else { 0 });
                if g.mode() == 0x1A {
                    break;
                }
            }
            g.bus.set_buttons(0);
            *self = g;
        } else {
            self.bus.cart.ram = ram;
        }
    }

    /// From power-on, press Start through the title and file A until the world map is up.
    pub fn boot_to_map(&mut self) -> bool {
        for f in 0..3000u32 {
            let m = self.mode();
            if m == 0x0C {
                // let the map finish appearing
                for _ in 0..90 {
                    self.run_frame_with(0);
                }
                return true;
            }
            self.run_frame_with(if f % 50 < 5 && f > 120 { bus::BTN_START } else { 0 });
        }
        false
    }

    /// From the world map, enter level `level` (0..=31): press Start, and when the game
    /// copies the level's header (mode $0B, routine 01:5579 reads the level number at
    /// $A269) put the wanted number there first. Returns once the level is playing.
    pub fn enter_level(&mut self, level: u8) -> bool {
        let mut set = false;
        for f in 0..900u32 {
            let m = self.mode();
            if m == 0x0B && !set {
                self.bus.write(0xA269, level);
                set = true;
            }
            if set && m == 0x04 {
                for _ in 0..3 {
                    self.run_frame_with(0);
                }
                return true;
            }
            let press = !set && m == 0x0C && f % 30 < 4;
            self.run_frame_with(if press { bus::BTN_START } else { 0 });
        }
        false
    }
}

impl Gb {
    /// Enter `level` from the world map whatever point the game's own Mario stands on:
    /// Start only works on a point that opens a level (0F:41B1), so first put him on one
    /// ($A840 = point number; the special points 0F:418F-0F:41A1 checks are avoided).
    pub fn enter_level_anywhere(&mut self, level: u8) -> bool {
        let rom = self.bus.cart.rom_copy();
        let table = crate::world::point_levels(&rom);
        let special = [0x05, 0x09, 0x0E, 0x17, 0x1D, 0x11];
        if let Some(p) = (0..table.len()).find(|&p| table[p] < 32 && !special.contains(&(p as u8))) {
            self.bus.write(0xA840, p as u8);
        }
        self.enter_level(level)
    }
}

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
    if load_save && gb.bus.cart.battery {
        if let Ok(data) = std::fs::read(save_path(rom_path)) {
            let n = data.len().min(gb.bus.cart.ram.len());
            gb.bus.cart.ram[..n].copy_from_slice(&data[..n]);
        }
    }
    Ok(gb)
}
