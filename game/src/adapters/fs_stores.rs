//! Settings, battery saves and new zone progress kept in files (where: `host::paths`), plus
//! stores that keep nothing, for unattended checks.
use crate::domain::settings::Settings;
use crate::domain::zone_progress::ZoneProgress;
use crate::ports::{SaveStore, SettingsStore, ZoneProgressStore};
use gb_core::Gb;
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

/// `saves/<cartridge>.zones`.
pub struct ZoneFile {
    pub rom: PathBuf,
}

impl ZoneProgressStore for ZoneFile {
    fn load(&self) -> Option<ZoneProgress> {
        ZoneProgress::from_text(&std::fs::read_to_string(host::zones_path(&self.rom)).ok()?)
    }
    fn save(&self, progress: &ZoneProgress) {
        let p = host::zones_path(&self.rom);
        let written = p.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&p, progress.to_text()));
        if let Err(e) = written {
            eprintln!("could not write the new zones' progress: {e}");
        }
    }
}

/// No progress kept (checks must not touch the player's files).
pub struct NoZoneProgress;

impl ZoneProgressStore for NoZoneProgress {
    fn load(&self) -> Option<ZoneProgress> {
        None
    }
    fn save(&self, _: &ZoneProgress) {}
}
