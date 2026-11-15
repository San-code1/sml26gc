//! Settings, battery saves and map progress kept in files (where: `host::paths`), plus
//! stores that keep nothing, for unattended checks.
use crate::domain::settings::Settings;
use crate::ports::{ProgressStore, SaveStore, SettingsStore};
use gb_core::Gb;
use std::collections::HashSet;
use std::path::PathBuf;

/// `settings.cfg` next to the programs.
pub struct SettingsFile;

impl SettingsStore for SettingsFile {
    fn load(&self) -> Settings {
        host::settings_file::read().map(|t| Settings::from_text(&t)).unwrap_or_default()
    }
    fn save(&self, settings: &Settings) {
        let _ = host::settings_file::write(&settings.to_text());
    }
}

/// Default settings, never written (checks must not touch the player's file).
pub struct DefaultSettings;

impl SettingsStore for DefaultSettings {
    fn load(&self) -> Settings {
        Settings::default()
    }
    fn save(&self, _: &Settings) {}
}

/// `saves/<cartridge>.sav`.
pub struct BatteryFile {
    pub rom: PathBuf,
}

impl SaveStore for BatteryFile {
    fn save(&self, gb: &mut Gb) {
        if let Err(e) = host::write_battery_save(gb, &self.rom) {
            eprintln!("could not write the save file: {e}");
        }
    }
}

/// `saves/<cartridge>.progress.txt`.
pub struct ProgressFile {
    pub path: PathBuf,
}

impl ProgressFile {
    pub fn for_rom(rom: &std::path::Path) -> ProgressFile {
        ProgressFile { path: host::progress_path(rom) }
    }
    pub fn load(&self) -> HashSet<u8> {
        host::progress::load(&self.path)
    }
}

impl ProgressStore for ProgressFile {
    fn save(&self, cleared: &HashSet<u8>) {
        let _ = host::progress::save(&self.path, cleared);
    }
}
