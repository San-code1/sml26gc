//! Where every file lives. The game and the editor both use these, so they always agree
//! (this is part of the contract between them, see docs/ARCHITECTURE.md, "Contract between game and editor").
//!
//! In the data folder (`data_dir`, the executables' own folder: the programs are portable):
//!   base_cartridge.txt                the original cartridge file both programs use
//!   settings.cfg                      the game's settings
//!   editor.cfg                        the editor's own settings
//!   editor\projects\<name>\           a project: its file `<name>.nmbproject` and the cartridge
//!                                     made from it over the base cartridge, `<base> - <name>.gb`,
//!                                     editor_place.txt, where the editor was when it saved, and
//!                                     restore_points.txt, what imported levels and zones restore to
//!   saves\vanilla\, saves\<project>\  battery save (game.sav) and new zone progress (zones.txt)
//!   exports\                          level, zone, world map and project files the editor writes
use std::path::{Path, PathBuf};

/// The folder the running program sits in (where the other program is found).
pub fn exe_dir() -> PathBuf {
    std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())).unwrap_or_else(|| PathBuf::from("."))
}

/// The folder every data file lives in: the executables' folder on every platform so far, so
/// the unpacked package is self-contained.
pub fn data_dir() -> PathBuf {
    exe_dir()
}

/// Why the data folder is of no use, in `lang`: None when it can be written. Tried by making and
/// removing a file.
pub fn data_dir_problem(lang: i18n::Lang) -> Option<&'static str> {
    (!data_dir_writable()).then(|| crate::texts::tr(lang, "This folder cannot be written: settings, projects and saves will not be kept. Move the program's folder somewhere you can write, such as Documents."))
}

fn data_dir_writable() -> bool {
    let probe = data_dir().join(format!(".write-test-{}", std::process::id()));
    let ok = std::fs::write(&probe, b"").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

fn stem(rom: &Path) -> String {
    rom.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "game".into())
}

/// The file naming the base cartridge (`base_cartridge`).
pub fn base_cartridge_path() -> PathBuf {
    data_dir().join("base_cartridge.txt")
}

pub fn settings_path() -> PathBuf {
    data_dir().join("settings.cfg")
}

pub fn editor_settings_path() -> PathBuf {
    data_dir().join("editor.cfg")
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
    data_dir().join("editor").join("projects")
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

/// What project `name`'s imported levels and zones restore to (the editor's only).
pub fn project_restore_points(name: &str) -> PathBuf {
    project_dir(name).join("restore_points.txt")
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
    data_dir().join("saves").join(project.unwrap_or("vanilla"))
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
    data_dir().join("exports")
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
    exe_dir().join(format!("{GAME_PROGRAM}{}", std::env::consts::EXE_SUFFIX))
}

/// The editor program, next to the game.
pub fn editor_exe() -> PathBuf {
    exe_dir().join(format!("{EDITOR_PROGRAM}{}", std::env::consts::EXE_SUFFIX))
}

/// The programs' names (the `[[bin]]` names of `game` and `editor`), without the system's suffix.
pub const GAME_PROGRAM: &str = "sml26gc";
pub const EDITOR_PROGRAM: &str = "sml26gc_editor";

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
    fn the_programs_find_each_other_by_their_bin_names() {
        let manifest = |crate_dir: &str| std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(crate_dir).join("Cargo.toml")).unwrap().replace('\r', "");
        assert!(manifest("game").contains(&format!("[[bin]]\nname = \"{GAME_PROGRAM}\"")));
        assert!(manifest("editor").contains(&format!("[[bin]]\nname = \"{EDITOR_PROGRAM}\"")));
        assert!(game_exe().file_stem().is_some_and(|s| s == GAME_PROGRAM));
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
