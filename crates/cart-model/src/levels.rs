//! Levels as the cartridge stores them, read from its code (see MECHANICS.md, "Levels"):
//!
//! * 32 level headers of 20 bytes, pointed to by the word table at bank 1 $55CB
//!   (indexed by the level number at $A269, copied to $A800 by 01:5579).
//!   Header byte $0D = ROM bank of the level map, low nibble of byte $12 = map number,
//!   bytes $0B-$0C = address (bank 8) of the 128 block definitions copied to $A600 (0:03FF).
//! * In the map bank, the word table at $42F0 gives each map's compressed data (0:0361).
//! * Compression (0:0361): a byte below $80 is one block; $80+v followed by n writes block v
//!   1+n times (n = 0 means 256). Runs stop at the end of each 256-block row. The decoder
//!   fills $B000-$DFFF: 48 rows of 256 blocks, each block 16x16 pixels.
//! * A block is four 8x8 tiles (top-left, top-right, bottom-left, bottom-right), $A600 + 4*id.
use gb_core::Gb;

pub const LEVELS: usize = 32;
pub const COLS: usize = 256;
pub const ROWS: usize = 48;
pub const MAP_BYTES: usize = COLS * ROWS;
const HEADER_TABLE: usize = 0x55CB; // bank 1, file offset = address
const MAP_TABLE: u16 = 0x42F0;

fn off(bank: u8, addr: u16) -> usize {
    bank as usize * 0x4000 + (addr as usize & 0x3FFF)
}

fn word(rom: &[u8], at: usize) -> u16 {
    rom[at] as u16 | (rom[at + 1] as u16) << 8
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Header {
    /// File offset of the 20-byte header.
    pub at: usize,
    pub bank: u8,
    pub map: u8,
}

pub fn header(rom: &[u8], level: usize) -> Header {
    let at = off(1, word(rom, HEADER_TABLE + level * 2));
    Header { at, bank: rom[at + 0x0D], map: rom[at + 0x12] & 0x0F }
}

/// The bank the decoder reads map data from: the level's own bank, or in an expanded
/// copy its twin $20 banks higher (see `expand`).
pub fn data_bank(rom: &[u8], bank: u8) -> u8 {
    if is_expanded(rom) {
        bank | 0x20
    } else {
        bank
    }
}

/// File offset of the map table entry and of the compressed data.
pub fn map_location(rom: &[u8], h: Header) -> (usize, usize) {
    let entry = off(h.bank, MAP_TABLE) + h.map as usize * 2;
    (entry, off(data_bank(rom, h.bank), word(rom, entry)))
}

// ---------------------------------------------------------------- room for bigger levels
//
// The cartridge has no free space in its level banks, so an edited copy is made 1 MB (the
// MBC1 chip addresses up to 2 MB): banks $20-$3F are copies of $00-$3F, and a tiny patch
// makes the level decoder (0:0361, which runs with the screen off and no interrupts) read
// map data from bank $20 + the level's bank, then switch back. Each zone gets a whole
// 16 KB bank for its maps instead of the ~6 KB it had.
//   0:037C  ld a,$00 -> ld a,$01   upper bank bit set while decoding (0:037B writes it to $4000)
//   0:03A9  ld a,h / cp $e0        -> jp $39BD (end-of-map test moved to the stub)
//   0:39BD  stub in the unused $FF filler:
//           ld a,h / cp $e0 / jp nz,$0386 / xor a / ld ($4000),a / ret
// Bank $20 holds the same patched copy of bank 0, because with the upper bit set the
// MBC1 maps bank $20 at $0000-$3FFF.

const STUB_AT: usize = 0x39BD;
const STUB: [u8; 11] = [0x7C, 0xFE, 0xE0, 0xC2, 0x86, 0x03, 0xAF, 0xEA, 0x00, 0x40, 0xC9];
// Objects the same way: the object loader 03:6BBB (called once, at 0:0581, screen off) is
// run with the upper bank bit set, so it reads its table and lists from bank $23, whose
// $4000-$6036 the loader never uses and so can hold bigger lists.
//   0:0581  call $6BBB -> call $39C8
//   0:39C8  ld a,1 / ld ($4000),a / call $6BBB / xor a / ld ($4000),a / ret
pub const OBJ_STUB_AT: usize = 0x39C8;
const OBJ_STUB: [u8; 13] = [0x3E, 0x01, 0xEA, 0x00, 0x40, 0xCD, 0xBB, 0x6B, 0xAF, 0xEA, 0x00, 0x40, 0xC9];

pub fn is_expanded(rom: &[u8]) -> bool {
    rom.len() == 0x10_0000 && rom[0x037C] == 0x01 && rom[STUB_AT..STUB_AT + STUB.len()] == STUB
}

pub fn objects_patched(rom: &[u8]) -> bool {
    is_expanded(rom) && rom[OBJ_STUB_AT..OBJ_STUB_AT + OBJ_STUB.len()] == OBJ_STUB && rom[0x0582] == OBJ_STUB_AT as u8
}

/// Can this cartridge be expanded safely? (the patched bytes must be what we expect)
pub fn can_expand(original: &[u8]) -> bool {
    original.len() == 0x8_0000
        && original[0x037B..0x037D] == [0x3E, 0x00]
        && original[0x03A9..0x03AF] == [0x7C, 0xFE, 0xE0, 0x20, 0xD8, 0xC9]
        && original[STUB_AT..OBJ_STUB_AT + OBJ_STUB.len()].iter().all(|&b| b == 0xFF)
        && original[0x0581..0x0584] == [0xCD, 0xBB, 0x6B]
        && original[0x0147] == 0x03
}

/// Add the object patch to a copy expanded before it existed (its bank $23 is still a plain
/// copy of bank 3, so the game reads the same lists until they're saved again).
pub fn upgrade(rom: &mut [u8]) {
    if is_expanded(rom) && !objects_patched(rom) && rom[OBJ_STUB_AT..OBJ_STUB_AT + OBJ_STUB.len()].iter().all(|&b| b == 0xFF) {
        for base in [0usize, 0x8_0000] {
            rom[base + 0x0581..base + 0x0584].copy_from_slice(&[0xCD, OBJ_STUB_AT as u8, (OBJ_STUB_AT >> 8) as u8]);
            rom[base + OBJ_STUB_AT..base + OBJ_STUB_AT + OBJ_STUB.len()].copy_from_slice(&OBJ_STUB);
        }
    }
}

pub fn expand(original: &[u8]) -> Result<Vec<u8>, String> {
    if is_expanded(original) {
        let mut r = original.to_vec();
        upgrade(&mut r);
        return Ok(r);
    }
    if !can_expand(original) {
        return Err("this cartridge isn't the version the editor knows; levels can only be saved if they fit in place".into());
    }
    let mut rom = original.to_vec();
    rom.extend_from_slice(original);
    for base in [0usize, 0x8_0000] {
        rom[base + 0x037C] = 0x01;
        rom[base + 0x03A9..base + 0x03AC].copy_from_slice(&[0xC3, STUB_AT as u8, (STUB_AT >> 8) as u8]);
        rom[base + STUB_AT..base + STUB_AT + STUB.len()].copy_from_slice(&STUB);
        rom[base + 0x0581..base + 0x0584].copy_from_slice(&[0xCD, OBJ_STUB_AT as u8, (OBJ_STUB_AT >> 8) as u8]);
        rom[base + OBJ_STUB_AT..base + OBJ_STUB_AT + OBJ_STUB.len()].copy_from_slice(&OBJ_STUB);
    }
    // header: ROM size 1 MB, and its checksum
    rom[0x0148] = 0x05;
    let mut x: u8 = 0;
    for &b in &rom[0x0134..=0x014C] {
        x = x.wrapping_sub(b).wrapping_sub(1);
    }
    rom[0x014D] = x;
    Ok(rom)
}

/// Decode a map exactly as 0:0361 does; returns the blocks and how many bytes it read.
pub fn decode(rom: &[u8], start: usize) -> (Vec<u8>, usize) {
    let mut out = vec![0u8; MAP_BYTES];
    let mut pos = 0usize;
    let mut src = start;
    let next = |src: &mut usize| -> u8 {
        let b = rom.get(*src).copied().unwrap_or(0);
        *src += 1;
        b
    };
    while pos < MAP_BYTES {
        let b = next(&mut src);
        if b & 0x80 == 0 {
            out[pos] = b;
            pos += 1;
            continue;
        }
        let v = b & 0x7F;
        out[pos] = v;
        pos += 1;
        if pos % COLS == 0 {
            continue;
        }
        let mut n = next(&mut src);
        loop {
            out[pos] = v;
            pos += 1;
            if pos % COLS == 0 {
                break;
            }
            n = n.wrapping_sub(1);
            if n == 0 {
                break;
            }
        }
    }
    (out, src - start)
}

/// Compress a map so that `decode` gives it back exactly.
pub fn encode(map: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for row in map.chunks(COLS) {
        let mut i = 0;
        while i < row.len() {
            let v = row[i] & 0x7F;
            let mut len = 1;
            while i + len < row.len() && row[i + len] & 0x7F == v && len < 256 {
                len += 1;
            }
            if len == 1 {
                out.push(v);
            } else {
                out.push(0x80 | v);
                out.push((len - 1) as u8);
            }
            i += len;
        }
    }
    out
}

/// Everything needed to draw a level: its blocks, the block definitions and the tile
/// graphics the game loads for it, captured by letting the game itself load the level.
#[derive(Clone)]
pub struct LevelArt {
    pub metatiles: Vec<u8>, // 128 * 4 tile numbers
    pub vram: Vec<u8>,
    pub lcdc: u8,
    pub bgp: u8,
}

impl LevelArt {
    pub fn capture(gb: &Gb) -> LevelArt {
        LevelArt {
            metatiles: gb.bus.cart.ram[0x600..0x800].to_vec(),
            vram: gb.bus.ppu.vram.to_vec(),
            lcdc: gb.bus.ppu.lcdc,
            bgp: gb.bus.ppu.bgp(),
        }
    }

    /// Shade (0-3) of pixel (x, y) of block `id`.
    pub fn block_pixel(&self, id: u8, x: usize, y: usize) -> u8 {
        let q = (y / 8) * 2 + x / 8;
        let tile = self.metatiles[(id as usize & 0x7F) * 4 + q];
        let base = if self.lcdc & 0x10 != 0 {
            tile as usize * 16
        } else {
            (0x1000 + (tile as i8 as i32) * 16) as usize
        };
        let a = base + (y % 8) * 2;
        let bit = 7 - (x % 8);
        let c = ((self.vram[a + 1] >> bit) & 1) << 1 | ((self.vram[a] >> bit) & 1);
        (self.bgp >> (c * 2)) & 3
    }
}

/// The level map as the game holds it after loading ($B000-$BFFF in cartridge RAM,
/// $C000-$DFFF in work RAM).
pub fn map_in_memory(gb: &Gb) -> Vec<u8> {
    let mut m = gb.bus.cart.ram[0x1000..0x2000].to_vec();
    m.extend_from_slice(&gb.bus.wram[..]);
    m
}

/// The maps kept in one bank: (map number, table entry offset, data offset, data length),
/// in the order they sit in the ROM.
pub fn bank_maps(rom: &[u8], bank: u8) -> Vec<(u8, usize, usize, usize)> {
    let mut v: Vec<(u8, usize, usize, usize)> = Vec::new();
    for l in 0..LEVELS {
        let h = header(rom, l);
        if h.bank == bank && !v.iter().any(|m| m.0 == h.map) {
            let (entry, start) = map_location(rom, h);
            let (_, len) = decode(rom, start);
            v.push((h.map, entry, start, len));
        }
    }
    v.sort_by_key(|m| m.2);
    v
}

/// Where a bank's maps may go and how many bytes that is. In the original layout they sit
/// one after another (first map's start to last map's end); in an expanded copy they have
/// the whole twin bank from the first map's start to the bank's end.
pub fn bank_capacity(rom: &[u8], original: &[u8], bank: u8) -> (usize, usize) {
    let maps = bank_maps(original, bank);
    let start = maps.iter().map(|m| m.2).min().unwrap_or(0);
    if is_expanded(rom) {
        let s = off(bank | 0x20, 0x4000) + (start & 0x3FFF);
        let end = off(bank | 0x20, 0x4000) + 0x4000;
        return (s, end - s);
    }
    let end = maps.iter().map(|m| m.2 + m.3).max().unwrap_or(0);
    (start, end - start)
}

/// Bytes the bank's maps need with these edited maps, and the bytes available.
pub fn bank_space(rom: &[u8], original: &[u8], bank: u8, edited: &[(u8, &[u8])]) -> (usize, usize) {
    let need = bank_maps(rom, bank)
        .iter()
        .map(|m| match edited.iter().find(|e| e.0 == m.0) {
            Some((_, map)) => encode(map).len(),
            None => m.3,
        })
        .sum();
    (need, bank_capacity(rom, original, bank).1)
}

/// Write edited maps into the ROM image: every map of the bank is packed again, one after
/// another, in the space it has (see `bank_capacity`), and the bank's map table is
/// pointed at the new places. Every level that uses an edited map changes too.
pub fn store_bank(rom: &mut [u8], original: &[u8], bank: u8, edited: &[(u8, &[u8])]) -> Result<String, String> {
    let (start, capacity) = bank_capacity(rom, original, bank);
    let maps = bank_maps(rom, bank);
    let datas: Vec<(usize, Vec<u8>)> = maps
        .iter()
        .map(|m| match edited.iter().find(|e| e.0 == m.0) {
            Some((_, map)) => (m.1, encode(map)),
            None => (m.1, rom[m.2..m.2 + m.3].to_vec()),
        })
        .collect();
    let need: usize = datas.iter().map(|d| d.1.len()).sum();
    if need > capacity {
        return Err(format!(
            "this zone's levels need {need} bytes but the cartridge has {capacity} for them ({} too many); use longer rows of the same block or simplify another level of the zone",
            need - capacity
        ));
    }
    let expanded = is_expanded(rom);
    let mut at = start;
    for (entry, data) in datas {
        rom[at..at + data.len()].copy_from_slice(&data);
        let addr = 0x4000 + (at & 0x3FFF) as u16;
        rom[entry] = addr as u8;
        rom[entry + 1] = (addr >> 8) as u8;
        if expanded {
            // keep the twin bank's table the same
            rom[entry + 0x8_0000] = addr as u8;
            rom[entry + 0x8_0001] = (addr >> 8) as u8;
        }
        at += data.len();
    }
    Ok(format!("bank {bank:02X}: {need}/{capacity} bytes"))
}

/// Levels that use the same map (bank and number) as `level`.
pub fn sharing(rom: &[u8], level: usize) -> Vec<usize> {
    let h = header(rom, level);
    (0..LEVELS).filter(|&l| l != level && {
        let o = header(rom, l);
        o.bank == h.bank && o.map == h.map
    }).collect()
}
