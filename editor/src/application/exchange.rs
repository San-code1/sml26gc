//! Level, zone and project files: the document gives or takes the data (`domain::files` reads and
//! writes the text), the person picks the file through `FileDialogs`.
use crate::domain::document::{EditorDocument, ZoneRefusal};
use crate::domain::files::{self, FileError};
use crate::ports::{FileDialogs, FileKind, TextFiles};
use cart_model::levels::LEVELS;
use cart_model::original_points::OnMap;
use std::path::PathBuf;

#[derive(Debug)]
pub enum ExchangeError {
    Io(std::io::Error),
    File(FileError),
    /// As many new levels as there can be.
    NoRoom,
    /// One of the 32 levels: never replaced from a file.
    Protected,
    /// A new zone was not made.
    Zone(ZoneRefusal),
    /// One of the game's zones: never replaced from a file.
    ProtectedZone,
    /// A world map file imported as a new zone.
    WorldAsZone,
}

/// What a finished action did; None from the functions below means the dialog was cancelled.
pub type Done<T> = Result<Option<T>, ExchangeError>;

fn write(dialogs: &dyn FileDialogs, out: &dyn TextFiles, kind: FileKind, name: &str, text: impl FnOnce() -> String) -> Done<PathBuf> {
    let Some(path) = dialogs.save_as(kind, name) else { return Ok(None) };
    out.write(&path, &text()).map_err(ExchangeError::Io)?;
    Ok(Some(path))
}

fn read(dialogs: &dyn FileDialogs, input: &dyn TextFiles, kind: FileKind) -> Done<String> {
    let Some(path) = dialogs.open(kind) else { return Ok(None) };
    input.read(&path).map(Some).map_err(ExchangeError::Io)
}

pub fn export_level(doc: &EditorDocument, l: usize, name: &str, dialogs: &dyn FileDialogs, out: &dyn TextFiles) -> Done<PathBuf> {
    write(dialogs, out, FileKind::Level, name, || files::level_text(&doc.level_file(l)))
}

/// A level file as a new level: its number.
pub fn import_level(doc: &mut EditorDocument, dialogs: &dyn FileDialogs, input: &dyn TextFiles) -> Done<usize> {
    if doc.new_levels() >= cart_model::extra_levels::MAX {
        return Err(ExchangeError::NoRoom);
    }
    let Some(text) = read(dialogs, input, FileKind::Level)? else { return Ok(None) };
    let f = files::read_level(&text).map_err(ExchangeError::File)?;
    doc.import_level(f).map(Some).ok_or(ExchangeError::NoRoom)
}

/// A level file in place of new level `l`.
pub fn replace_level(doc: &mut EditorDocument, l: usize, dialogs: &dyn FileDialogs, input: &dyn TextFiles) -> Done<()> {
    if l < LEVELS {
        return Err(ExchangeError::Protected);
    }
    let Some(text) = read(dialogs, input, FileKind::Level)? else { return Ok(None) };
    let f = files::read_level(&text).map_err(ExchangeError::File)?;
    Ok(doc.replace_level(l, f).then_some(()))
}

/// Map `map` as a zone file (a new zone, or a zone a door enters).
pub fn export_zone(doc: &EditorDocument, map: OnMap, name: &str, dialogs: &dyn FileDialogs, out: &dyn TextFiles) -> Done<PathBuf> {
    let z = doc.zone_file(map).ok_or(ExchangeError::Zone(ZoneRefusal::NotABase))?;
    write(dialogs, out, FileKind::Zone, name, || files::zone_text(&z, doc.map_start(map)))
}

/// A zone file as a new zone: its number, and Mario's start if the file has it. A world map
/// file is refused (the game has one world map).
pub fn import_zone(doc: &mut EditorDocument, dialogs: &dyn FileDialogs, input: &dyn TextFiles) -> Done<(usize, Option<u8>)> {
    if doc.new_zones.len() >= cart_model::new_zones::MAX {
        return Err(ExchangeError::Zone(ZoneRefusal::TooManyZones(cart_model::new_zones::MAX)));
    }
    let Some(text) = read(dialogs, input, FileKind::Zone)? else { return Ok(None) };
    if files::read_world(&text).is_ok() {
        return Err(ExchangeError::WorldAsZone);
    }
    let (f, start) = files::read_zone(&text).map_err(ExchangeError::File)?;
    doc.import_zone(f).map(|k| Some((k, start))).map_err(ExchangeError::Zone)
}

/// A zone file in place of the new zone shown on `map`.
/// A zone file in place of the new zone shown on `map`: Mario's start if the file has it.
pub fn replace_zone(doc: &mut EditorDocument, map: OnMap, dialogs: &dyn FileDialogs, input: &dyn TextFiles) -> Done<Option<u8>> {
    let OnMap::NewZone(k) = map else { return Err(ExchangeError::ProtectedZone) };
    let Some(text) = read(dialogs, input, FileKind::Zone)? else { return Ok(None) };
    let (f, start) = files::read_zone(&text).map_err(ExchangeError::File)?;
    Ok(doc.replace_zone(k, f).then_some(start))
}

pub fn export_project(doc: &EditorDocument, name: &str, dialogs: &dyn FileDialogs, out: &dyn TextFiles) -> Done<PathBuf> {
    write(dialogs, out, FileKind::Project, name, || files::project_text(&doc.project_file()))
}

pub fn export_world(doc: &EditorDocument, name: &str, dialogs: &dyn FileDialogs, out: &dyn TextFiles) -> Done<PathBuf> {
    write(dialogs, out, FileKind::World, name, || files::world_text(&doc.world_file()))
}

/// A world map file in place of the world map: how many points and exits lost their target.
/// Also Mario's start, if the file has it.
pub fn import_world(doc: &mut EditorDocument, dialogs: &dyn FileDialogs, input: &dyn TextFiles) -> Done<(usize, usize, Option<u8>)> {
    let Some(text) = read(dialogs, input, FileKind::Zone)? else { return Ok(None) };
    let w = files::read_world(&text).map_err(ExchangeError::File)?;
    let start = w.start;
    doc.replace_world(w).map(|(p, e)| Some((p, e, start))).map_err(ExchangeError::File)
}
