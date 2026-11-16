//! The cartridge files: the original (read only) and the edited copy in `edits\`
//! (paths: `host::paths`).
use crate::domain::document::EditorDocument;
use crate::domain::prefs::Prefs;
use crate::ports::{EditedCopyStore, LookStore, PrefsStore, TextFiles};
use std::path::{Path, PathBuf};

pub struct EditedCopyFile {
    path: PathBuf,
}

impl EditedCopyFile {
    fn make_dir(&self) {
        if let Some(d) = self.path.parent() {
            let _ = std::fs::create_dir_all(d);
        }
    }
}

impl EditedCopyStore for EditedCopyFile {
    fn path(&self) -> &Path {
        &self.path
    }
    fn write(&self, rom: &[u8]) -> std::io::Result<()> {
        self.make_dir();
        std::fs::write(&self.path, rom)
    }
}

/// What opening a cartridge gives: the document, where it saves, and whether an edited
/// copy already existed.
pub struct Opened {
    pub doc: EditorDocument,
    pub store: EditedCopyFile,
    pub had_copy: bool,
}

pub fn open(original_path: &Path) -> Result<Opened, String> {
    let original = std::fs::read(original_path).map_err(|e| e.to_string())?;
    let path = host::edited_copy_path(original_path);
    let edited = std::fs::read(&path).ok();
    let had_copy = path.exists();
    Ok(Opened { doc: EditorDocument::open(original, edited), store: EditedCopyFile { path }, had_copy })
}

/// `editor.cfg` next to the editor; a missing or unreadable file gives the defaults.
pub struct PrefsFile;

impl PrefsStore for PrefsFile {
    fn load(&self) -> Prefs {
        std::fs::read_to_string(host::editor_settings_path()).map(|t| Prefs::from_text(&t)).unwrap_or_default()
    }
    fn save(&self, prefs: &Prefs) {
        let _ = std::fs::write(host::editor_settings_path(), prefs.to_text());
    }
}

/// The object pictures captured for one cartridge: `cache\looks-<hash>.txt` next to the
/// editor (`hash` of the original file, so another cartridge has its own).
pub struct LookFile {
    path: PathBuf,
}

impl LookFile {
    pub fn for_cartridge(original: &[u8]) -> LookFile {
        // FNV-1a
        let hash = original.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3));
        LookFile { path: host::exe_dir().join("cache").join(format!("looks-{hash:016x}.txt")) }
    }
}

impl LookStore for LookFile {
    fn load(&self) -> Option<String> {
        std::fs::read_to_string(&self.path).ok()
    }
    fn save(&self, text: &str) {
        if let Some(d) = self.path.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(&self.path, text);
    }
}

/// Language from the game's settings file (English unless Spanish was chosen).
pub fn spanish() -> bool {
    host::settings_file::language().as_deref() == Some("es")
}

/// Level and project files, wherever the person chose.
pub struct TextFile;

impl TextFiles for TextFile {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }
    fn write(&self, path: &Path, text: &str) -> std::io::Result<()> {
        std::fs::write(path, text)
    }
}
