//! Everything known about this cartridge from reverse engineering it. Works on ROM bytes
//! and on a running `Gb`; no files, no windows.
//!
//! | Module | What |
//! |---|---|
//! | `rom` | Banks and words in the ROM file |
//! | `levels` | Level headers, map locations, the map compression (decode/encode) |
//! | `objects` | Object lists per level: 3-byte format, reading, re-packing |
//! | `world` | World map picture and which level each map point opens |
//! | `patch` | The 1 MB edited copy that gives levels and objects more room |
//! | `layout` | Each zone's space for its maps and packing edited maps back |
//! | `ram_map` | Where the running game keeps things; `GameMode` |
//! | `live` | Reading what the running game has loaded |
//! | `art` | Pictures captured from the running game (blocks, map tiles, object looks) |
//! | `driver` | `GameDriver`: boot to the map, enter a level, back to the map... |
//! | `custom_map` | The editor-made world map (points, paths) as text |
//! | `error` | `CartError` and what a save used |
pub mod art;
pub mod custom_map;
pub mod driver;
pub mod error;
pub mod layout;
pub mod levels;
pub mod live;
pub mod objects;
pub mod patch;
pub mod ram_map;
pub mod rom;
pub mod world;

pub use driver::GameDriver;
pub use error::{BankUsage, CartError, ObjectsUsage};
pub use ram_map::GameMode;
