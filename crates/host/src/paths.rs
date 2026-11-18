//! Where every file lives. The game and the editor both use these, so they always agree
//! (this is part of the contract between them, see docs/ARCHITECTURE.md, "Contract between game and editor").
//!
//! Next to the executables:
//!   base_cartridge.txt                the original cartridge file both programs use
//!   settings.cfg                      the game's settings
//!   editor.cfg                        the editor's own settings
//!   editor\projects\<name>\           a project: its file `<name>.nmbproject` and the cartridge
//!                                     made from it over the base cartridge, `<base> - <name>.gb`,
//!                                     and editor_place.txt, where the editor was when it saved
//!   saves\vanilla\, saves\<project>\  battery save (game.sav) and new zone progress (zones.txt)
//!   exports\                          level, zone, world map and project files the editor writes
use std::path::{Path, PathBuf};

/// The folder the running program sits in.
pub fn exe_dir() -> PathBuf {
    std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())).unwrap_or_else(|| PathBuf::from("."))
}

fn stem(rom: &Path) -> String {
    rom.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "game".into())
}

/// The file naming the base cartridge (`base_cartridge`).
pub fn base_cartridge_path() -> PathBuf {
    exe_dir().join("base_cartridge.txt")
}

pub fn settings_path() -> PathBuf {
    exe_dir().join("settings.cfg")
}

pub fn editor_settings_path() -> PathBuf {
    exe_dir().join("editor.cfg")
}

/// The longest project name.
pub const PROJECT_NAME_MAX: usize = 40;
/// The game's start screen name for the original cartridge, which no project may take.
pub const VANILLA: &str = "VANILLA";

/// Can `name` name a project (and its folder)? Printable ASCII (the editor's font has no
/// accents), no path characters, not VANILLA.
pub fn valid_project_name(name: &str) -> bool {
    let t = name.trim();
    !t.is_empty()
        && t == name
        && t.chars().count() <= PROJECT_NAME_MAX
        && t.chars().all(|c| c.is_ascii() && !c.is_ascii_control() && !r#"\/:*?"<>|"#.contains(c))
        && !t.ends_with('.')
        && !t.eq_ignore_ascii_case(VANILLA)
}

pub fn projects_dir() -> PathBuf {
    exe_dir().join("editor").join("projects")
}

pub fn project_dir(name: &str) -> PathBuf {
    projects_dir().join(name)
}

/// The edited cartridge of project `name`, made from the base cartridge `base`.
pub fn project_rom(name: &str, base: &Path) -> PathBuf {
    project_dir(name).join(format!("{} - {name}.gb", stem(base)))
}

/// Project `name`'s file, what the project is (its cartridge is made from it).
pub fn project_file(name: &str) -> PathBuf {
    project_dir(name).join(format!("{name}.nmbproject"))
}

/// Where the editor was in project `name` when it was last saved.
pub fn project_place(name: &str) -> PathBuf {
    project_dir(name).join("editor_place.txt")
}

/// The cartridge file in project `name`'s folder, whatever it is called.
pub fn project_rom_in(name: &str) -> Option<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(project_dir(name)).ok()?.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("gb"))).collect();
    v.sort();
    v.into_iter().next()
}

/// The projects (folders of `projects_dir`), sorted by name.
pub fn projects() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(projects_dir()).into_iter().flatten().flatten().filter(|e| e.path().is_dir()).filter_map(|e| e.file_name().to_str().map(String::from)).collect();
    v.sort_by_key(|n| n.to_lowercase());
    v
}

/// Where a game's saves go: `saves/vanilla` for the original cartridge, else the project's.
pub fn saves_dir(project: Option<&str>) -> PathBuf {
    exe_dir().join("saves").join(project.unwrap_or("vanilla"))
}

/// The battery save in a saves folder.
pub fn battery_file(saves: &Path) -> PathBuf {
    saves.join("game.sav")
}

/// The progress in new zones and added points in a saves folder.
pub fn zones_file(saves: &Path) -> PathBuf {
    saves.join("zones.txt")
}

/// Where the editor's Save and Open dialogs start, by kind of file.
pub fn exports_dir() -> PathBuf {
    exe_dir().join("exports")
}

pub fn project_exports(name: &str) -> PathBuf {
    exports_dir().join("projects").join(name)
}

pub fn level_exports() -> PathBuf {
    exports_dir().join("levels")
}

pub fn zone_exports() -> PathBuf {
    exports_dir().join("zones")
}

/// The game program, next to the editor.
pub fn game_exe() -> PathBuf {
    exe_dir().join(if cfg!(windows) { "game.exe" } else { "game" })
}

/// The editor program, next to the game.
pub fn editor_exe() -> PathBuf {
    exe_dir().join(if cfg!(windows) { "editor.exe" } else { "editor" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_projects_files() {
        let rom = Path::new("C:/Games/Some Game (USA).gb");
        assert!(project_rom("Islands", rom).ends_with("editor/projects/Islands/Some Game (USA) - Islands.gb"));
        assert!(project_file("Islands").ends_with("editor/projects/Islands/Islands.nmbproject"));
        assert!(battery_file(&saves_dir(Some("Islands"))).ends_with("saves/Islands/game.sav"));
        assert!(zones_file(&saves_dir(None)).ends_with("saves/vanilla/zones.txt"));
        assert!(project_exports("Islands").ends_with("exports/projects/Islands"));
    }

    #[test]
    fn project_names() {
        assert!(valid_project_name("Sky Islands 2"));
        assert!(valid_project_name("Mundo (v2) - copia_1"));
        for bad in ["", " lead", "trail ", "a/b", "a:b", "dot.", "vanilla", "VANILLA", "Mundo ñ", &"x".repeat(41)] {
            assert!(!valid_project_name(bad), "{bad:?}");
        }
    }
}
