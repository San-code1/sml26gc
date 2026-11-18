//! Saving: every change goes into the cartridge image (`cart-model` lays out zones and
//! objects), then the project's file (what the project is) and its cartridge are written.
use crate::domain::document::{EditorDocument, Saved};
use crate::domain::files;
use crate::ports::EditedCopyStore;
use cart_model::CartError;

pub enum SaveError {
    /// Something doesn't fit in the cartridge.
    Layout(CartError),
    Write(std::io::Error),
}

pub fn save(doc: &mut EditorDocument, store: &dyn EditedCopyStore) -> Result<Vec<Saved>, SaveError> {
    let parts = doc.commit().map_err(SaveError::Layout)?;
    store.write_project(&files::project_text(&doc.project_file())).map_err(SaveError::Write)?;
    store.write(&doc.rom).map_err(SaveError::Write)?;
    Ok(parts)
}
