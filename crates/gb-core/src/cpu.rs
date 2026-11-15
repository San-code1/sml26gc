//! Sharp SM83 (Game Boy CPU) interpreter.
use crate::bus::Bus;
use crate::observer::BusObserver;

const ZF: u8 = 0x80;
const NF: u8 = 0x40;
const HF: u8 = 0x20;
const CF: u8 = 0x10;

#[derive(Clone)]
pub(crate) struct Cpu {
    pub(crate) a: u8,
    pub(crate) f: u8,
    pub(crate) b: u8,
    pub(crate) c: u8,
    pub(crate) d: u8,
    pub(crate) e: u8,
    pub(crate) h: u8,
    pub(crate) l: u8,
    pub(crate) sp: u16,
    pub(crate) pc: u16,
    pub(crate) ime: bool,
    ei_pending: bool,
    pub(crate) halted: bool,
}

impl Cpu {
    /// Register state right after the DMG boot ROM hands over to the cartridge.
    pub(crate) fn new() -> Self {
        Cpu {
            a: 0x01,
            f: 0xB0,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            sp: 0xFFFE,
            pc: 0x0100,
            ime: false,
            ei_pending: false,
            halted: false,
        }
    }

    fn hl(&self) -> u16 {
        ((self.h as u16) << 8) | self.l as u16
    }
    fn set_hl(&mut self, v: u16) {
        self.h = (v >> 8) as u8;
        self.l = v as u8;
    }
    fn get_rp(&self, i: u8) -> u16 {
        match i {
            0 => ((self.b as u16) << 8) | self.c as u16,
            1 => ((self.d as u16) << 8) | self.e as u16,
            2 => self.hl(),
            _ => self.sp,
        }
    }
    fn set_rp(&mut self, i: u8, v: u16) {
        match i {
            0 => {
                self.b = (v >> 8) as u8;
                self.c = v as u8
            }
            1 => {
                self.d = (v >> 8) as u8;
                self.e = v as u8
            }
            2 => self.set_hl(v),
            _ => self.sp = v,
        }
    }
    fn carry(&self) -> bool {
        self.f & CF != 0
    }
    fn fetch<O: BusObserver>(&mut self, bus: &mut Bus<O>) -> u8 {
        let v = bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        v
    }
    fn imm16<O: BusObserver>(&mut self, bus: &mut Bus<O>) -> u16 {
        let lo = self.fetch(bus) as u16;
        let hi = self.fetch(bus) as u16;
        (hi << 8) | lo
    }
    fn push<O: BusObserver>(&mut self, bus: &mut Bus<O>, v: u16) {
        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, (v >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, v as u8);
    }
    fn pop<O: BusObserver>(&mut self, bus: &mut Bus<O>) -> u16 {
        let lo = bus.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        let hi = bus.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        (hi << 8) | lo
    }
    fn get_r<O: BusObserver>(&mut self, bus: &mut Bus<O>, i: u8) -> u8 {
        match i {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => bus.read(self.hl()),
            _ => self.a,
        }
    }
    fn set_r<O: BusObserver>(&mut self, bus: &mut Bus<O>, i: u8, v: u8) {
        match i {
            0 => self.b = v,
            1 => self.c = v,
            2 => self.d = v,
            3 => self.e = v,
            4 => self.h = v,
            5 => self.l = v,
            6 => bus.write(self.hl(), v),
            _ => self.a = v,
        }
    }
    fn cond(&self, i: u8) -> bool {
        match i {
            0 => self.f & ZF == 0,
            1 => self.f & ZF != 0,
            2 => self.f & CF == 0,
            _ => self.f & CF != 0,
        }
    }

    fn alu(&mut self, k: u8, v: u8) {
        let a = self.a;
        let c = self.carry() as u8;
        match k {
            0 | 1 => {
                let cin = if k == 1 { c } else { 0 };
                let r = a as u16 + v as u16 + cin as u16;
                self.a = r as u8;
                self.f = z(self.a)
                    | if (a & 0xF) + (v & 0xF) + cin > 0xF { HF } else { 0 }
                    | if r > 0xFF { CF } else { 0 };
            }
            2 | 3 | 7 => {
                let cin = if k == 3 { c } else { 0 };
                let r = a as i16 - v as i16 - cin as i16;
                let res = r as u8;
                self.f = z(res)
                    | NF
                    | if ((a & 0xF) as i16 - (v & 0xF) as i16 - cin as i16) < 0 { HF } else { 0 }
                    | if r < 0 { CF } else { 0 };
                if k != 7 {
                    self.a = res;
                }
            }
            4 => {
                self.a = a & v;
                self.f = z(self.a) | HF;
            }
            5 => {
                self.a = a ^ v;
                self.f = z(self.a);
            }
            _ => {
                self.a = a | v;
                self.f = z(self.a);
            }
        }
    }

    fn add_sp_e<O: BusObserver>(&mut self, bus: &mut Bus<O>) -> u16 {
        let e = self.fetch(bus);
        let sp = self.sp;
        let r = sp.wrapping_add(e as i8 as i16 as u16);
        self.f = if (sp & 0xF) + (e as u16 & 0xF) > 0xF { HF } else { 0 }
            | if (sp & 0xFF) + (e as u16) > 0xFF { CF } else { 0 };
        r
    }

    /// Execute one instruction (or service an interrupt). Returns T-cycles used.
    pub(crate) fn step<O: BusObserver>(&mut self, bus: &mut Bus<O>) -> u32 {
        let pending = bus.ie & bus.iflag & 0x1F;
        if pending != 0 {
            self.halted = false;
            if self.ime {
                self.ime = false;
                self.ei_pending = false;
                let bit = pending.trailing_zeros();
                bus.iflag &= !(1 << bit);
                let pc = self.pc;
                self.push(bus, pc);
                self.pc = 0x40 + 8 * bit as u16;
                return 20;
            }
        }
        if self.halted {
            return 4;
        }
        let enable = self.ei_pending;
        if O::ENABLED {
            bus.cur_pc = self.pc;
        }
        let op = self.fetch(bus);
        let cyc = self.exec(bus, op);
        if enable && self.ei_pending {
            self.ime = true;
            self.ei_pending = false;
        }
        cyc
    }

    fn exec<O: BusObserver>(&mut self, bus: &mut Bus<O>, op: u8) -> u32 {
        match op {
            0x00 => 4,
            0x01 | 0x11 | 0x21 | 0x31 => {
                let v = self.imm16(bus);
                self.set_rp((op >> 4) & 3, v);
                12
            }
            0x02 | 0x12 => {
                let a = self.get_rp((op >> 4) & 1);
                bus.write(a, self.a);
                8
            }
            0x22 => {
                let a = self.hl();
                bus.write(a, self.a);
                self.set_hl(a.wrapping_add(1));
                8
            }
            0x32 => {
                let a = self.hl();
                bus.write(a, self.a);
                self.set_hl(a.wrapping_sub(1));
                8
            }
            0x0A | 0x1A => {
                let a = self.get_rp((op >> 4) & 1);
                self.a = bus.read(a);
                8
            }
            0x2A => {
                let a = self.hl();
                self.a = bus.read(a);
                self.set_hl(a.wrapping_add(1));
                8
            }
            0x3A => {
                let a = self.hl();
                self.a = bus.read(a);
                self.set_hl(a.wrapping_sub(1));
                8
            }
            0x03 | 0x13 | 0x23 | 0x33 => {
                let i = (op >> 4) & 3;
                let v = self.get_rp(i).wrapping_add(1);
                self.set_rp(i, v);
                8
            }
            0x0B | 0x1B | 0x2B | 0x3B => {
                let i = (op >> 4) & 3;
                let v = self.get_rp(i).wrapping_sub(1);
                self.set_rp(i, v);
                8
            }
            0x04 | 0x0C | 0x14 | 0x1C | 0x24 | 0x2C | 0x34 | 0x3C => {
                let r = (op >> 3) & 7;
                let v = self.get_r(bus, r);
                let n = v.wrapping_add(1);
                self.set_r(bus, r, n);
                self.f = (self.f & CF) | z(n) | if v & 0xF == 0xF { HF } else { 0 };
                if r == 6 { 12 } else { 4 }
            }
            0x05 | 0x0D | 0x15 | 0x1D | 0x25 | 0x2D | 0x35 | 0x3D => {
                let r = (op >> 3) & 7;
                let v = self.get_r(bus, r);
                let n = v.wrapping_sub(1);
                self.set_r(bus, r, n);
                self.f = (self.f & CF) | z(n) | NF | if v & 0xF == 0 { HF } else { 0 };
                if r == 6 { 12 } else { 4 }
            }
            0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x36 | 0x3E => {
                let r = (op >> 3) & 7;
                let v = self.fetch(bus);
                self.set_r(bus, r, v);
                if r == 6 { 12 } else { 8 }
            }
            0x07 => {
                let c = self.a >> 7;
                self.a = (self.a << 1) | c;
                self.f = if c != 0 { CF } else { 0 };
                4
            }
            0x0F => {
                let c = self.a & 1;
                self.a = (self.a >> 1) | (c << 7);
                self.f = if c != 0 { CF } else { 0 };
                4
            }
            0x17 => {
                let c = self.a >> 7;
                self.a = (self.a << 1) | self.carry() as u8;
                self.f = if c != 0 { CF } else { 0 };
                4
            }
            0x1F => {
                let c = self.a & 1;
                self.a = (self.a >> 1) | ((self.carry() as u8) << 7);
                self.f = if c != 0 { CF } else { 0 };
                4
            }
            0x08 => {
                let a = self.imm16(bus);
                bus.write(a, self.sp as u8);
                bus.write(a.wrapping_add(1), (self.sp >> 8) as u8);
                20
            }
            0x09 | 0x19 | 0x29 | 0x39 => {
                let hl = self.hl();
                let v = self.get_rp((op >> 4) & 3);
                let r = hl as u32 + v as u32;
                self.f = (self.f & ZF)
                    | if (hl & 0xFFF) + (v & 0xFFF) > 0xFFF { HF } else { 0 }
                    | if r > 0xFFFF { CF } else { 0 };
                self.set_hl(r as u16);
                8
            }
            0x10 => {
                self.fetch(bus);
                4
            }
            0x18 => {
                let e = self.fetch(bus) as i8;
                self.pc = self.pc.wrapping_add(e as i16 as u16);
                12
            }
            0x20 | 0x28 | 0x30 | 0x38 => {
                let e = self.fetch(bus) as i8;
                if self.cond((op >> 3) & 3) {
                    self.pc = self.pc.wrapping_add(e as i16 as u16);
                    12
                } else {
                    8
                }
            }
            0x27 => {
                let mut a = self.a;
                let mut c = self.carry();
                let h = self.f & HF != 0;
                if self.f & NF == 0 {
                    let mut adj = 0u8;
                    if h || (a & 0xF) > 9 {
                        adj |= 0x06;
                    }
                    if c || a > 0x99 {
                        adj |= 0x60;
                        c = true;
                    }
                    a = a.wrapping_add(adj);
                } else {
                    let mut adj = 0u8;
                    if h {
                        adj |= 0x06;
                    }
                    if c {
                        adj |= 0x60;
                    }
                    a = a.wrapping_sub(adj);
                }
                self.a = a;
                self.f = z(a) | (self.f & NF) | if c { CF } else { 0 };
                4
            }
            0x2F => {
                self.a = !self.a;
                self.f |= NF | HF;
                4
            }
            0x37 => {
                self.f = (self.f & ZF) | CF;
                4
            }
            0x3F => {
                self.f = (self.f & ZF) | if self.carry() { 0 } else { CF };
                4
            }
            0x76 => {
                self.halted = true;
                4
            }
            0x40..=0x7F => {
                let v = self.get_r(bus, op & 7);
                self.set_r(bus, (op >> 3) & 7, v);
                if op & 7 == 6 || (op >> 3) & 7 == 6 { 8 } else { 4 }
            }
            0x80..=0xBF => {
                let v = self.get_r(bus, op & 7);
                self.alu((op >> 3) & 7, v);
                if op & 7 == 6 { 8 } else { 4 }
            }
            0xC0 | 0xC8 | 0xD0 | 0xD8 => {
                if self.cond((op >> 3) & 3) {
                    self.pc = self.pop(bus);
                    20
                } else {
                    8
                }
            }
            0xC1 | 0xD1 | 0xE1 | 0xF1 => {
                let v = self.pop(bus);
                match (op >> 4) & 3 {
                    3 => {
                        self.a = (v >> 8) as u8;
                        self.f = v as u8 & 0xF0;
                    }
                    i => self.set_rp(i, v),
                }
                12
            }
            0xC5 | 0xD5 | 0xE5 | 0xF5 => {
                let v = match (op >> 4) & 3 {
                    3 => ((self.a as u16) << 8) | self.f as u16,
                    i => self.get_rp(i),
                };
                self.push(bus, v);
                16
            }
            0xC2 | 0xCA | 0xD2 | 0xDA => {
                let a = self.imm16(bus);
                if self.cond((op >> 3) & 3) {
                    self.pc = a;
                    16
                } else {
                    12
                }
            }
            0xC3 => {
                self.pc = self.imm16(bus);
                16
            }
            0xC4 | 0xCC | 0xD4 | 0xDC => {
                let a = self.imm16(bus);
                if self.cond((op >> 3) & 3) {
                    let pc = self.pc;
                    self.push(bus, pc);
                    self.pc = a;
                    24
                } else {
                    12
                }
            }
            0xCD => {
                let a = self.imm16(bus);
                let pc = self.pc;
                self.push(bus, pc);
                self.pc = a;
                24
            }
            0xC6 | 0xCE | 0xD6 | 0xDE | 0xE6 | 0xEE | 0xF6 | 0xFE => {
                let v = self.fetch(bus);
                self.alu((op >> 3) & 7, v);
                8
            }
            0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => {
                let pc = self.pc;
                self.push(bus, pc);
                self.pc = (op & 0x38) as u16;
                16
            }
            0xC9 => {
                self.pc = self.pop(bus);
                16
            }
            0xD9 => {
                self.pc = self.pop(bus);
                self.ime = true;
                16
            }
            0xCB => self.exec_cb(bus),
            0xE0 => {
                let n = self.fetch(bus);
                bus.write(0xFF00 | n as u16, self.a);
                12
            }
            0xF0 => {
                let n = self.fetch(bus);
                self.a = bus.read(0xFF00 | n as u16);
                12
            }
            0xE2 => {
                bus.write(0xFF00 | self.c as u16, self.a);
                8
            }
            0xF2 => {
                self.a = bus.read(0xFF00 | self.c as u16);
                8
            }
            0xE8 => {
                self.sp = self.add_sp_e(bus);
                16
            }
            0xF8 => {
                let v = self.add_sp_e(bus);
                self.set_hl(v);
                12
            }
            0xE9 => {
                self.pc = self.hl();
                4
            }
            0xF9 => {
                self.sp = self.hl();
                8
            }
            0xEA => {
                let a = self.imm16(bus);
                bus.write(a, self.a);
                16
            }
            0xFA => {
                let a = self.imm16(bus);
                self.a = bus.read(a);
                16
            }
            0xF3 => {
                self.ime = false;
                self.ei_pending = false;
                4
            }
            0xFB => {
                self.ei_pending = true;
                4
            }
            _ => 4, // unused opcodes: treat as no-op
        }
    }

    fn exec_cb<O: BusObserver>(&mut self, bus: &mut Bus<O>) -> u32 {
        let op = self.fetch(bus);
        let r = op & 7;
        let n = (op >> 3) & 7;
        let v = self.get_r(bus, r);
        let hl = r == 6;
        match op >> 6 {
            0 => {
                let c = self.carry() as u8;
                let (res, co) = match n {
                    0 => (v.rotate_left(1), v >> 7),
                    1 => (v.rotate_right(1), v & 1),
                    2 => ((v << 1) | c, v >> 7),
                    3 => ((v >> 1) | (c << 7), v & 1),
                    4 => (v << 1, v >> 7),
                    5 => ((v >> 1) | (v & 0x80), v & 1),
                    6 => (v.rotate_left(4), 0),
                    _ => (v >> 1, v & 1),
                };
                self.f = z(res) | if co != 0 { CF } else { 0 };
                self.set_r(bus, r, res);
                if hl { 16 } else { 8 }
            }
            1 => {
                self.f = (self.f & CF) | HF | if v & (1 << n) == 0 { ZF } else { 0 };
                if hl { 12 } else { 8 }
            }
            2 => {
                self.set_r(bus, r, v & !(1 << n));
                if hl { 16 } else { 8 }
            }
            _ => {
                self.set_r(bus, r, v | (1 << n));
                if hl { 16 } else { 8 }
            }
        }
    }
}

fn z(v: u8) -> u8 {
    if v == 0 { ZF } else { 0 }
}
