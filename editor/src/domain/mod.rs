//! What is being edited and the rules for editing it, testable without a window or files: the
//! project itself (`project_model`) and what only the editor needs.
pub mod look_cache;
pub mod object_catalogue;
pub mod object_names;
pub mod place;
pub mod prefs;
pub mod tile_palette;
pub mod tools;

pub use project_model::{document, files, stamp};
