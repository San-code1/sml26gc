//! Where every file lives. The game and the editor both use these, so they always agree
//! (this is part of the contract between them, see docs/ARCHITECTURE.md §6).
//!
//! Next to the executables:
//!   settings.cfg                      the game's settings (the editor reads the language)
//!   saves\<cartridge>.sav             battery save
//!   saves\<cartridge>.progress.txt    levels cleared on the editor-made map
//!   edits\<cartridge> (edit).gb       the editor's copy of the cartridge
//! Next to a cartridge file:
//!   <cartridge>.points.txt            the editor-made world map
use std::path::{Path, PathBuf};

/// The folder the running program sits in.
pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn stem(rom: &Path) -> String {
    rom.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "game".into())
}

pub fn settings_path() -> PathBuf {
    exe_dir().join("settings.cfg")
}

/// Battery save of a cartridge file.
pub fn save_path(rom: &Path) -> PathBuf {
    exe_dir().join("saves").join(format!("{}.sav", stem(rom)))
}

/// Levels cleared on the editor-made map, for a cartridge file.
pub fn progress_path(rom: &Path) -> PathBuf {
    save_path(rom).with_extension("progress.txt")
}

/// The editor's edited copy of an original cartridge file.
pub fn edited_copy_path(original: &Path) -> PathBuf {
    exe_dir()
        .join("edits")
        .join(format!("{} (edit).gb", stem(original)))
}

/// The editor-made world map file that goes with a cartridge file.
pub fn custom_map_path(rom: &Path) -> PathBuf {
    rom.with_extension("points.txt")
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
        assert!(progress_path(rom).ends_with("saves/Some Game (USA).progress.txt"));
        assert!(edited_copy_path(rom).ends_with("edits/Some Game (USA) (edit).gb"));
        assert_eq!(
            custom_map_path(Path::new("x/a (edit).gb")),
            Path::new("x/a (edit).points.txt")
        );
        // the edited copy has its own save and progress
        let copy = edited_copy_path(rom);
        assert!(save_path(&copy).ends_with("saves/Some Game (USA) (edit).sav"));
    }
}
