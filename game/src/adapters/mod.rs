//! The ports' implementations: macroquad (window, picture, keyboard), cpal (speakers),
//! files (through `host`) and the system around the window.
pub mod cpal_audio;
pub mod fs_stores;
pub mod gilrs_pad;
pub mod macroquad_display;
pub mod macroquad_keyboard;
pub mod macroquad_picker;
pub mod system_window;
pub mod title_preview;
