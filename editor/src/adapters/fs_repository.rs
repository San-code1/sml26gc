//! The cartridge files: the original (read only), the edited copy in `edits\` and its
//! editor-made map file next to it (paths: `host::paths`).
use crate::domain::document::EditorDocument;
use crate::ports::EditedCopyStore;
use cart_model::custom_map::CustomMap;
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
    fn save_custom_map(&self, map: &CustomMap) -> std::io::Result<()> {
        self.make_dir();
        host::save_custom_map(map, &self.path)
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
    let custom = host::load_custom_map(&path).unwrap_or_default();
    let had_copy = path.exists();
    Ok(Opened {
        doc: EditorDocument::open(original, edited, custom),
        store: EditedCopyFile { path },
        had_copy,
    })
}

/// Language from the game's settings file (English unless Spanish was chosen).
pub fn spanish() -> bool {
    host::settings_file::language().as_deref() == Some("es")
}
