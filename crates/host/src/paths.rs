//! Where every file lives. The game and the editor both use these, so they always agree
//! (this is part of the contract between them, see docs/ARCHITECTURE.md, "Contract between game and editor").
//!
//! Next to the executables:
//!   settings.cfg                      the game's settings (the editor reads the language)
//!   editor.cfg                        the editor's own settings
//!   last_cartridge.txt                the cartridge file chosen last at the game's start
//!   saves\<cartridge>.sav             battery save
//!   saves\<cartridge>.zones           progress in the editor's new zones
//!   edits\<cartridge> (edit).gb       the editor's copy of the cartridge
//!   exports\                          level and project files the editor writes
use std::path::{Path, PathBuf};

/// The folder the running program sits in.
pub fn exe_dir() -> PathBuf {
    std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())).unwrap_or_else(|| PathBuf::from("."))
}

fn stem(rom: &Path) -> String {
    rom.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "game".into())
}

/// The cartridge file chosen last at the game's start screen.
pub fn last_cartridge_path() -> PathBuf {
    exe_dir().join("last_cartridge.txt")
}

pub fn settings_path() -> PathBuf {
    exe_dir().join("settings.cfg")
}

pub fn editor_settings_path() -> PathBuf {
    exe_dir().join("editor.cfg")
}

/// Battery save of a cartridge file.
pub fn save_path(rom: &Path) -> PathBuf {
    exe_dir().join("saves").join(format!("{}.sav", stem(rom)))
}

/// Progress in the new zones of a cartridge file (the game keeps it next to the battery save).
pub fn zones_path(rom: &Path) -> PathBuf {
    exe_dir().join("saves").join(format!("{}.zones", stem(rom)))
}

/// The editor's edited copy of an original cartridge file.
pub fn edited_copy_path(original: &Path) -> PathBuf {
    exe_dir().join("edits").join(format!("{} (edit).gb", stem(original)))
}

/// Where the editor's Save and Open dialogs for level and project files start.
pub fn exports_dir() -> PathBuf {
    exe_dir().join("exports")
}

/// The game program, next to the editor.
pub fn game_exe() -> PathBuf {
    exe_dir().join(if cfg!(windows) { "game.exe" } else { "game" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_the_cartridge() {
        let rom = Path::new("C:/Games/Some Game (USA).gb");
        assert!(save_path(rom).ends_with("saves/Some Game (USA).sav"));
        assert!(zones_path(rom).ends_with("saves/Some Game (USA).zones"));
        assert!(edited_copy_path(rom).ends_with("edits/Some Game (USA) (edit).gb"));
        // the edited copy has its own save
        let copy = edited_copy_path(rom);
        assert!(save_path(&copy).ends_with("saves/Some Game (USA) (edit).sav"));
    }
}
