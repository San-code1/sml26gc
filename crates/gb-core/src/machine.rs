//! The Game Boy as a whole: processor + memory bus, run a video frame at a time.
use crate::bus::Bus;
use crate::cart::Cart;
use crate::cpu::Cpu;
use crate::input::Buttons;
use crate::observer::{BusObserver, NoObserver};
use crate::ppu::{Ppu, Screen};

pub const FRAME_CYCLES: u32 = 70224;
pub const FRAME_SECONDS: f64 = FRAME_CYCLES as f64 / 4_194_304.0; // 59.73 Hz

/// A Game Boy with a cartridge in it. `O` watches its memory bus (nothing by default).
#[derive(Clone)]
pub struct GameBoy<O: BusObserver = NoObserver> {
    cpu: Cpu,
    bus: Bus<O>,
}

/// The usual Game Boy, with nothing watching it.
pub type Gb = GameBoy<NoObserver>;

impl GameBoy<NoObserver> {
    /// Power on with this ROM image; `sample_rate` is the sound output rate in Hz.
    pub fn new(rom: Vec<u8>, sample_rate: u32) -> Result<Gb, String> {
        GameBoy::with_observer(rom, sample_rate, NoObserver)
    }
}

impl<O: BusObserver> GameBoy<O> {
    pub fn with_observer(
        rom: Vec<u8>,
        sample_rate: u32,
        observer: O,
    ) -> Result<GameBoy<O>, String> {
        let cart = Cart::new(rom)?;
        Ok(GameBoy {
            cpu: Cpu::new(),
            bus: Bus::new(cart, sample_rate, observer),
        })
    }

    /// Run until the next vertical blank (one video frame).
    pub fn run_frame(&mut self) {
        let mut total = 0u32;
        loop {
            let c = self.cpu.step(&mut self.bus);
            self.bus.tick(c);
            total += c;
            if self.bus.ppu.frame_ready {
                self.bus.ppu.frame_ready = false;
                break;
            }
            if total >= FRAME_CYCLES * 2 || (!self.bus.ppu.lcd_on() && total >= FRAME_CYCLES) {
                break;
            }
        }
    }

    /// Run one frame with these buttons held, throwing the sound away.
    pub fn run_frame_with(&mut self, buttons: Buttons) {
        self.set_buttons(buttons);
        self.run_frame();
        self.clear_audio();
    }

    /// The buttons held from now on (until changed).
    pub fn set_buttons(&mut self, buttons: Buttons) {
        self.bus.set_buttons(buttons);
    }

    /// Read a byte through the memory bus, as the program would.
    pub fn read(&mut self, addr: u16) -> u8 {
        self.bus.read(addr)
    }

    /// Write a byte through the memory bus, as the program would.
    pub fn write(&mut self, addr: u16, value: u8) {
        self.bus.write(addr, value)
    }

    /// The last picture drawn.
    pub fn screen(&self) -> &Screen {
        &self.bus.ppu.fb
    }

    /// Interleaved stereo samples made since the last `clear_audio`.
    pub fn audio(&self) -> &[f32] {
        &self.bus.apu.out
    }

    pub fn clear_audio(&mut self) {
        self.bus.apu.out.clear();
    }

    pub fn sample_rate(&self) -> u32 {
        self.bus.apu.sample_rate()
    }

    /// Video memory, sprite table and display registers.
    pub fn video(&self) -> &Ppu {
        &self.bus.ppu
    }

    /// Work RAM ($C000-$DFFF).
    pub fn wram(&self) -> &[u8; 0x2000] {
        &self.bus.wram
    }

    pub fn cart(&self) -> &Cart {
        &self.bus.cart
    }

    pub fn cart_mut(&mut self) -> &mut Cart {
        &mut self.bus.cart
    }

    /// Address of the next instruction.
    pub fn pc(&self) -> u16 {
        self.cpu.pc
    }

    pub fn observer(&self) -> &O {
        &self.bus.observer
    }

    pub fn observer_mut(&mut self) -> &mut O {
        &mut self.bus.observer
    }
}
