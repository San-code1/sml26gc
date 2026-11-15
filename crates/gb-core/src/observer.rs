//! Watching the memory bus from outside, for research tools now and code hooks later.
//!
//! A `GameBoy` is generic over its observer. The default, `NoObserver`, has
//! `ENABLED = false`, so every call below is removed when the emulator is compiled:
//! watching costs nothing unless it is used.

/// Sees every access the program (or anyone else) makes through the memory bus.
///
/// `pc` is the address of the instruction being executed; `bank` is the ROM bank mapped at
/// $4000-$7FFF at that moment.
pub trait BusObserver: Clone {
    /// Must be `true` for the methods to be called at all.
    const ENABLED: bool;

    #[inline(always)]
    fn on_read(&mut self, _pc: u16, _bank: usize, _addr: u16) {}

    #[inline(always)]
    fn on_write(&mut self, _pc: u16, _bank: usize, _addr: u16, _value: u8) {}
}

/// Watches nothing (the normal case).
#[derive(Clone, Copy, Debug, Default)]
pub struct NoObserver;

impl BusObserver for NoObserver {
    const ENABLED: bool = false;
}
