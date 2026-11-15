//! Memory map, timer, joypad and interrupt flags.
use crate::apu::Apu;
use crate::cart::Cart;
use crate::ppu::Ppu;

pub const BTN_RIGHT: u8 = 0x01;
pub const BTN_LEFT: u8 = 0x02;
pub const BTN_UP: u8 = 0x04;
pub const BTN_DOWN: u8 = 0x08;
pub const BTN_A: u8 = 0x10;
pub const BTN_B: u8 = 0x20;
pub const BTN_SELECT: u8 = 0x40;
pub const BTN_START: u8 = 0x80;

#[derive(Clone)]
pub struct Bus {
    pub cart: Cart,
    pub ppu: Ppu,
    pub apu: Apu,
    pub wram: [u8; 0x2000],
    hram: [u8; 0x7F],
    pub ie: u8,
    pub iflag: u8,
    // timer
    div: u16,
    tima: u8,
    tma: u8,
    tac: u8,
    // joypad
    buttons: u8,
    joy_sel: u8,
    sb: u8,
    sc: u8,
    // research: log writes to watched addresses with the instruction that made them
    pub cur_pc: u16,
    pub watch: Vec<u16>,
    pub watch_log: Vec<(u16, usize, u16, u8)>, // (pc, bank, addr, value)
    pub read_watch: Option<(u16, u16, usize)>,
    pub read_log: Vec<(u16, u16, u8)>, // (pc, addr, bank)
}

impl Bus {
    pub fn new(cart: Cart, sample_rate: u32) -> Bus {
        Bus {
            cart,
            ppu: Ppu::new(),
            apu: Apu::new(sample_rate),
            wram: [0; 0x2000],
            hram: [0; 0x7F],
            ie: 0,
            iflag: 0xE1,
            div: 0xABCC,
            tima: 0,
            tma: 0,
            tac: 0xF8,
            buttons: 0,
            joy_sel: 0x30,
            sb: 0,
            sc: 0,
            cur_pc: 0,
            watch: Vec::new(),
            watch_log: Vec::new(),
            read_watch: None,
            read_log: Vec::new(),
        }
    }

    pub fn set_buttons(&mut self, b: u8) {
        if b & !self.buttons != 0 {
            self.iflag |= 0x10;
        }
        self.buttons = b;
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        if let Some((lo, hi, bank)) = self.read_watch {
            // research only: data reads (not instruction bytes) from one ROM bank range
            if addr >= lo && addr <= hi && (bank == 0xFF || self.cart.upper_bank() == bank) && !(addr >= self.cur_pc && addr < self.cur_pc.wrapping_add(3)) && self.read_log.len() < 200_000 {
                self.read_log.push((self.cur_pc, addr, self.cart.upper_bank() as u8));
            }
        }
        match addr {
            0x0000..=0x7FFF | 0xA000..=0xBFFF => self.cart.read(addr),
            0x8000..=0x9FFF => self.ppu.vram[(addr - 0x8000) as usize],
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize],
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize],
            0xFE00..=0xFE9F => self.ppu.oam[(addr - 0xFE00) as usize],
            0xFEA0..=0xFEFF => 0xFF,
            0xFF00 => {
                let mut r = 0xC0 | (self.joy_sel & 0x30) | 0x0F;
                if self.joy_sel & 0x10 == 0 {
                    r &= !(self.buttons & 0x0F);
                }
                if self.joy_sel & 0x20 == 0 {
                    r &= !((self.buttons >> 4) & 0x0F);
                }
                r
            }
            0xFF01 => self.sb,
            0xFF02 => self.sc | 0x7E,
            0xFF04 => (self.div >> 8) as u8,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac | 0xF8,
            0xFF0F => self.iflag | 0xE0,
            0xFF10..=0xFF3F => self.apu.read(addr),
            0xFF40..=0xFF4B => self.ppu.read(addr),
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],
            0xFFFF => self.ie,
            _ => 0xFF,
        }
    }

    pub fn write(&mut self, addr: u16, v: u8) {
        if !self.watch.is_empty() && self.watch.contains(&addr) && self.watch_log.len() < 100_000 {
            // the bank mapped at 4000-7FFF when the write happened
            let bank = self.cart.upper_bank();
            self.watch_log.push((self.cur_pc, bank, addr, v));
        }
        match addr {
            0x0000..=0x7FFF | 0xA000..=0xBFFF => self.cart.write(addr, v),
            0x8000..=0x9FFF => self.ppu.vram[(addr - 0x8000) as usize] = v,
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize] = v,
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize] = v,
            0xFE00..=0xFE9F => self.ppu.oam[(addr - 0xFE00) as usize] = v,
            0xFF00 => self.joy_sel = v & 0x30,
            0xFF01 => self.sb = v,
            0xFF02 => {
                // No link cable: a started transfer finishes at once and reads 0xFF.
                self.sc = v;
                if v & 0x81 == 0x81 {
                    self.sb = 0xFF;
                    self.sc &= 0x7F;
                    self.iflag |= 0x08;
                }
            }
            0xFF04 => self.div = 0,
            0xFF05 => self.tima = v,
            0xFF06 => self.tma = v,
            0xFF07 => self.tac = v & 7,
            0xFF0F => self.iflag = v & 0x1F,
            0xFF10..=0xFF3F => self.apu.write(addr, v),
            0xFF46 => {
                let src = (v as u16) << 8;
                for i in 0..0xA0u16 {
                    let b = self.read(src + i);
                    self.ppu.oam[i as usize] = b;
                }
            }
            0xFF40..=0xFF4B => self.ppu.write(addr, v),
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = v,
            0xFFFF => self.ie = v,
            _ => {}
        }
    }

    pub fn tick(&mut self, cycles: u32) {
        // Timer: TIMA counts on the falling edge of the selected DIV bit.
        let bit = match self.tac & 3 {
            0 => 9,
            1 => 3,
            2 => 5,
            _ => 7,
        };
        let mut c = cycles;
        while c > 0 {
            let old = self.div;
            self.div = self.div.wrapping_add(4);
            c = c.saturating_sub(4);
            if self.tac & 4 != 0 && (old >> bit) & 1 == 1 && (self.div >> bit) & 1 == 0 {
                let (t, over) = self.tima.overflowing_add(1);
                if over {
                    self.tima = self.tma;
                    self.iflag |= 0x04;
                } else {
                    self.tima = t;
                }
            }
        }
        self.ppu.tick(cycles, &mut self.iflag);
        self.apu.tick(cycles);
    }
}
