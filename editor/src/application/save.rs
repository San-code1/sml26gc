//! Saving: every change goes into the cartridge image (`cart-model` lays out zones and
//! objects), then the editor-made map and the edited copy are written.
use crate::domain::document::{EditorDocument, Saved};
use crate::ports::EditedCopyStore;
use cart_model::CartError;

pub enum SaveError {
    /// Something doesn't fit in the cartridge.
    Layout(CartError),
    CustomMap(std::io::Error),
    Write(std::io::Error),
}

pub fn save(
    doc: &mut EditorDocument,
    store: &dyn EditedCopyStore,
) -> Result<Vec<Saved>, SaveError> {
    let mut parts = doc.commit().map_err(SaveError::Layout)?;
    if doc.custom_dirty {
        store
            .save_custom_map(&doc.custom)
            .map_err(SaveError::CustomMap)?;
        doc.custom_dirty = false;
        parts.push(Saved::CustomMap);
    }
    store.write(&doc.rom).map_err(SaveError::Write)?;
    Ok(parts)
}
