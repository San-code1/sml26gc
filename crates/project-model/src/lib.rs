//! A project of the editor, testable without a window or files: what is being edited and the
//! rules for editing it (`document`), its level, zone, world map and project files (`files`),
//! and the cartridge a project file makes over the base cartridge (`build`), which the editor
//! saves and the game plays.
pub mod build;
pub mod document;
pub mod files;
pub mod history;
pub mod stamp;
