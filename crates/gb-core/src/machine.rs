//! The Game Boy as a whole: processor + memory bus, run a video frame at a time.
use crate::bus::Bus;
use crate::{cart, cpu};

pub const FRAME_CYCLES: u32 = 70224;
pub const FRAME_SECONDS: f64 = FRAME_CYCLES as f64 / 4_194_304.0; // 59.73 Hz

#[derive(Clone)]
pub struct Gb {
    pub cpu: cpu::Cpu,
    pub bus: Bus,
}

impl Gb {
    pub fn new(rom: Vec<u8>, sample_rate: u32) -> Result<Gb, String> {
        let cart = cart::Cart::new(rom)?;
        Ok(Gb { cpu: cpu::Cpu::new(), bus: Bus::new(cart, sample_rate) })
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
    pub fn run_frame_with(&mut self, buttons: u8) {
        self.bus.set_buttons(buttons);
        self.run_frame();
        self.bus.apu.out.clear();
    }
}
