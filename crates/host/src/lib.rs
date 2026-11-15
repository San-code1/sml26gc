//! The computer side shared by the game, the editor and the tools: finding the cartridge,
//! where every file lives, battery saves, settings, progress, the editor-made map file and
//! PNG pictures. Nothing here knows about windows, sound or keyboards.
pub mod battery;
pub mod custom_map_file;
pub mod paths;
pub mod png_out;
pub mod progress;
pub mod rom_locator;
pub mod settings_file;

pub use battery::{load_gb, write_battery_save};
pub use custom_map_file::{load_custom_map, save_custom_map};
pub use paths::{
    custom_map_path, edited_copy_path, exe_dir, game_exe, progress_path, save_path, settings_path,
};
pub use rom_locator::{find_rom, require_rom, NO_CARTRIDGE};
