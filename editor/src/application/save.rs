//! Saving: every change goes into the cartridge image (`cart-model` lays out zones and
//! objects), then the edited copy is written.
use crate::domain::document::{EditorDocument, Saved};
use crate::ports::EditedCopyStore;
use cart_model::CartError;

pub enum SaveError {
    /// Something doesn't fit in the cartridge.
    Layout(CartError),
    Write(std::io::Error),
}

pub fn save(doc: &mut EditorDocument, store: &dyn EditedCopyStore) -> Result<Vec<Saved>, SaveError> {
    let parts = doc.commit().map_err(SaveError::Layout)?;
    store.write(&doc.rom).map_err(SaveError::Write)?;
    Ok(parts)
}
