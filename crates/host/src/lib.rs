//! The computer side shared by the game, the editor and the tools: finding the cartridge,
//! where every file lives, battery saves, settings and PNG pictures. Nothing here knows about windows, sound or keyboards.
pub mod battery;
pub mod paths;
pub mod png_out;
pub mod rom_locator;
pub mod settings_file;

pub use battery::{load_gb, write_battery_save};
pub use paths::{edited_copy_path, editor_settings_path, exe_dir, exports_dir, game_exe, save_path, settings_path};
pub use rom_locator::{find_rom, last_rom, last_rom_named, remember_rom, require_rom, NO_CARTRIDGE};
