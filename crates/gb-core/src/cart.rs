//! Cartridge: ROM, battery RAM and memory bank controllers (none, MBC1, MBC3, MBC5).

#[derive(Clone, Copy, PartialEq)]
enum Mbc {
    None,
    Mbc1,
    Mbc3,
    Mbc5,
}

#[derive(Clone)]
pub struct Cart {
    rom: Vec<u8>,
    ram: Vec<u8>,
    mbc: Mbc,
    rom_bank: u16,
    bank2: u8,
    mode: u8,
    ram_en: bool,
    battery: bool,
    ram_dirty: bool,
    title: String,
}

impl Cart {
    pub fn new(mut rom: Vec<u8>) -> Result<Cart, String> {
        if rom.len() < 0x150 {
            return Err("the file is too small to be a Game Boy cartridge".into());
        }
        let kind = rom[0x147];
        let (mbc, battery) = match kind {
            0x00 | 0x08 | 0x09 => (Mbc::None, kind == 0x09),
            0x01..=0x03 => (Mbc::Mbc1, kind == 0x03),
            0x0F..=0x13 => (Mbc::Mbc3, matches!(kind, 0x0F | 0x10 | 0x13)),
            0x19..=0x1E => (Mbc::Mbc5, matches!(kind, 0x1B | 0x1E)),
            _ => return Err(format!("cartridge type {kind:#04x} is not supported")),
        };
        let ram_size = match rom[0x149] {
            1 => 0x800,
            2 => 0x2000,
            3 => 0x8000,
            4 => 0x20000,
            5 => 0x10000,
            _ => 0,
        };
        // Pad the ROM to a power of two of 16 KB banks.
        let mut size = 0x8000;
        while size < rom.len() {
            size *= 2;
        }
        rom.resize(size, 0xFF);
        let title: String = rom[0x134..0x144]
            .iter()
            .take_while(|&&b| b != 0)
            .map(|&b| if (0x20..0x7F).contains(&b) { b as char } else { ' ' })
            .collect();
        Ok(Cart {
            rom,
            ram: vec![0; ram_size],
            mbc,
            rom_bank: 1,
            bank2: 0,
            mode: 0,
            ram_en: false,
            battery,
            ram_dirty: false,
            title: title.trim().to_string(),
        })
    }

    /// The ROM bank currently mapped at 0x4000-0x7FFF (for research tools).
    pub fn upper_bank(&self) -> usize {
        match self.mbc {
            Mbc::None => 1,
            Mbc::Mbc1 => (((self.bank2 as usize) << 5) | self.rom_bank as usize) % self.banks(),
            _ => self.rom_bank as usize % self.banks(),
        }
    }

    /// Swap the ROM image for another of the same size (the editor's what-if runs).
    pub fn set_rom(&mut self, rom: &[u8]) {
        if rom.len() == self.rom.len() {
            self.rom.copy_from_slice(rom);
        }
    }

    /// The whole ROM image (padded to a power of two of 16 KB banks).
    pub fn rom(&self) -> &[u8] {
        &self.rom
    }

    /// The ROM image to change in place (a port playing levels the cartridge has no room
    /// for writes them over others while they are played).
    pub fn rom_mut(&mut self) -> &mut [u8] {
        &mut self.rom
    }

    /// Take the ROM image out (leaving it empty) and put it back, to copy the rest cheaply.
    pub(crate) fn take_rom(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.rom)
    }

    pub(crate) fn put_rom(&mut self, rom: Vec<u8>) {
        self.rom = rom;
    }

    /// Battery-backed (or plain) cartridge RAM.
    pub fn ram(&self) -> &[u8] {
        &self.ram
    }

    pub fn ram_mut(&mut self) -> &mut [u8] {
        &mut self.ram
    }

    /// Take the RAM out (leaving it empty), e.g. to carry it over to a fresh power-on.
    pub fn take_ram(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.ram)
    }

    pub fn set_ram(&mut self, ram: Vec<u8>) {
        self.ram = ram;
    }

    /// The cartridge keeps its RAM with a battery (it has save files).
    pub fn has_battery(&self) -> bool {
        self.battery
    }

    /// The program has changed the RAM since mark_saved.
    pub fn needs_saving(&self) -> bool {
        self.ram_dirty
    }

    pub fn mark_saved(&mut self) {
        self.ram_dirty = false;
    }

    /// The name in the cartridge header.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// A byte of the ROM file by its offset.
    pub fn rom_byte(&self, off: usize) -> u8 {
        self.rom.get(off).copied().unwrap_or(0xFF)
    }

    fn banks(&self) -> usize {
        self.rom.len() / 0x4000
    }

    pub(crate) fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => {
                let bank = if self.mbc == Mbc::Mbc1 && self.mode == 1 {
                    ((self.bank2 as usize) << 5) % self.banks()
                } else {
                    0
                };
                self.rom[bank * 0x4000 + addr as usize]
            }
            0x4000..=0x7FFF => {
                let bank = match self.mbc {
                    Mbc::None => 1,
                    Mbc::Mbc1 => (((self.bank2 as usize) << 5) | self.rom_bank as usize) % self.banks(),
                    _ => self.rom_bank as usize % self.banks(),
                };
                self.rom[bank * 0x4000 + (addr as usize - 0x4000)]
            }
            0xA000..=0xBFFF => {
                if !self.ram_en && self.mbc != Mbc::None || self.ram.is_empty() {
                    return 0xFF;
                }
                let i = self.ram_index(addr);
                self.ram[i]
            }
            _ => 0xFF,
        }
    }

    fn ram_index(&self, addr: u16) -> usize {
        let bank = match self.mbc {
            Mbc::Mbc1 => {
                if self.mode == 1 {
                    self.bank2 as usize
                } else {
                    0
                }
            }
            Mbc::Mbc3 | Mbc::Mbc5 => self.bank2 as usize,
            Mbc::None => 0,
        };
        (bank * 0x2000 + (addr as usize - 0xA000)) % self.ram.len()
    }

    pub(crate) fn write(&mut self, addr: u16, v: u8) {
        match addr {
            0x0000..=0x1FFF => self.ram_en = v & 0x0F == 0x0A,
            0x2000..=0x3FFF => match self.mbc {
                Mbc::Mbc1 => {
                    let b = (v & 0x1F) as u16;
                    self.rom_bank = if b == 0 { 1 } else { b };
                }
                Mbc::Mbc3 => {
                    let b = (v & 0x7F) as u16;
                    self.rom_bank = if b == 0 { 1 } else { b };
                }
                Mbc::Mbc5 => {
                    if addr < 0x3000 {
                        self.rom_bank = (self.rom_bank & 0x100) | v as u16;
                    } else {
                        self.rom_bank = (self.rom_bank & 0xFF) | (((v & 1) as u16) << 8);
                    }
                }
                Mbc::None => {}
            },
            0x4000..=0x5FFF => {
                self.bank2 = match self.mbc {
                    Mbc::Mbc1 => v & 3,
                    Mbc::Mbc3 => v & 0x0F,
                    Mbc::Mbc5 => v & 0x0F,
                    Mbc::None => 0,
                }
            }
            0x6000..=0x7FFF => {
                if self.mbc == Mbc::Mbc1 {
                    self.mode = v & 1;
                }
            }
            0xA000..=0xBFFF => {
                if (self.ram_en || self.mbc == Mbc::None) && !self.ram.is_empty() {
                    if self.mbc == Mbc::Mbc3 && self.bank2 > 3 {
                        return; // real-time clock registers: not emulated
                    }
                    let i = self.ram_index(addr);
                    if self.ram[i] != v {
                        self.ram[i] = v;
                        self.ram_dirty = true;
                    }
                }
            }
            _ => {}
        }
    }
}
