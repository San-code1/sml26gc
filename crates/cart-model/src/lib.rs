//! Everything known about this cartridge from reverse engineering it: where and how it
//! keeps its levels, objects and world map, the patches that give edited levels more room,
//! and how to drive its program (see `driver`). Works on ROM bytes and on a running `Gb`;
//! no files, no windows.
pub mod custom_map;
pub mod driver;
pub mod levels;
pub mod objects;
pub mod world;

pub use driver::GameDriver;
