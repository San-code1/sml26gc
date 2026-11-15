//! The editor-made world map file (`<cartridge>.points.txt`).
use crate::paths::custom_map_path;
use cart_model::custom_map::CustomMap;
use std::path::Path;

/// The editor-made world map for this cartridge file, if it has one with points.
pub fn load_custom_map(rom: &Path) -> Option<CustomMap> {
    let t = std::fs::read_to_string(custom_map_path(rom)).ok()?;
    let m = CustomMap::parse(&t);
    if m.points.is_empty() {
        None
    } else {
        Some(m)
    }
}

/// Save the map next to the cartridge file; a map with no points removes the file (the
/// game then uses its own map).
pub fn save_custom_map(map: &CustomMap, rom: &Path) -> std::io::Result<()> {
    let path = custom_map_path(rom);
    if map.points.is_empty() {
        return std::fs::remove_file(path).or(Ok(()));
    }
    std::fs::write(path, map.to_text())
}
