//! The Game Boy: processor, memory bus, picture, sound, timer, buttons and cartridge
//! memory banks. Knows nothing about any particular game, files or windows.
//!
//! Everything goes through [`GameBoy`] (usually the alias [`Gb`]): run frames, hold
//! buttons, read the picture and sound, look at video memory and the cartridge, and read or
//! write memory as the program would. A [`BusObserver`] can watch every memory access.
mod apu;
mod bus;
pub mod cart;
mod cpu;
pub mod input;
pub mod machine;
pub mod observer;
pub mod palette;
pub mod ppu;
pub mod script;

pub use cart::Cart;
pub use input::Buttons;
pub use machine::{GameBoy, Gb, State, FRAME_CYCLES, FRAME_SECONDS};
pub use observer::{BusObserver, NoObserver};
pub use ppu::Screen;
