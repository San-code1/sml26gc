//! What can go wrong when changing the cartridge image, and what a successful save used.
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CartError {
    /// The cartridge is not the version whose code the patches were made for.
    UnknownVersion,
    /// A zone's levels don't fit in the space its bank has for them.
    ZoneFull { bank: u8, need: usize, capacity: usize },
    /// The objects of all levels don't fit in their space.
    ObjectsFull { need: usize, capacity: usize },
    /// More new levels than there are numbers for.
    TooManyLevels { max: usize },
    /// A new level's map or objects don't fit in the room it gets while it is played.
    LevelTooBig { level: usize, need: usize, capacity: usize },
    /// The new levels don't fit in the space kept for them.
    LevelsFull { need: usize, capacity: usize },
    /// The map's changed ways don't fit in the space kept for them.
    RoutesFull { need: usize, capacity: usize },
}

impl fmt::Display for CartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            CartError::UnknownVersion => {
                write!(f, "this cartridge isn't the version the editor knows; levels can only be saved if they fit in place")
            }
            CartError::ZoneFull { need, capacity, .. } => write!(
                f,
                "this zone's levels need {need} bytes but the cartridge has {capacity} for them ({} too many); use longer rows of the same block or simplify another level of the zone",
                need - capacity
            ),
            CartError::ObjectsFull { need, capacity } => write!(
                f,
                "the objects need {need} bytes but the cartridge has {capacity} for them; remove {} objects somewhere",
                (need - capacity + 2) / 3
            ),
            CartError::TooManyLevels { max } => write!(f, "there can be {max} levels at most"),
            CartError::LevelTooBig { level, need, capacity } => write!(f, "level {level} needs {need} bytes but has {capacity}; use longer rows of the same block or fewer objects"),
            CartError::LevelsFull { need, capacity } => write!(f, "the new levels need {need} bytes but the cartridge has {capacity} for them; remove a level"),
            CartError::RoutesFull { need, capacity } => write!(f, "the map's paths need {need} bytes but the cartridge has {capacity} for them; shorten or remove a path"),
        }
    }
}

impl std::error::Error for CartError {}

/// Bytes a zone's maps take after saving, against the bytes it has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BankUsage {
    pub bank: u8,
    pub used: usize,
    pub capacity: usize,
}

impl fmt::Display for BankUsage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bank {:02X}: {}/{} bytes", self.bank, self.used, self.capacity)
    }
}

/// Bytes the object lists take after saving, against the bytes they have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectsUsage {
    pub used: usize,
    pub capacity: usize,
}

impl fmt::Display for ObjectsUsage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "objects {}/{} bytes", self.used, self.capacity)
    }
}
