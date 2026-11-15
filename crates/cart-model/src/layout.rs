//! Where each zone's level maps sit in the ROM, how much room they have, and packing edited
//! maps back in.
use crate::error::{BankUsage, CartError};
use crate::levels::{decode, encode, header, map_location, LEVELS};
use crate::patch::{is_expanded, TWIN};
use crate::rom::{offset, set_word, BANK_SIZE};

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
        let s = offset(bank as usize | 0x20, 0x4000) + (start & 0x3FFF);
        let end = offset(bank as usize | 0x20, 0x4000) + BANK_SIZE;
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
pub fn store_bank(
    rom: &mut [u8],
    original: &[u8],
    bank: u8,
    edited: &[(u8, &[u8])],
) -> Result<BankUsage, CartError> {
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
        return Err(CartError::ZoneFull {
            bank,
            need,
            capacity,
        });
    }
    let expanded = is_expanded(rom);
    let mut at = start;
    for (entry, data) in datas {
        rom[at..at + data.len()].copy_from_slice(&data);
        let addr = 0x4000 + (at & 0x3FFF) as u16;
        set_word(rom, entry, addr);
        if expanded {
            // keep the twin bank's table the same
            set_word(rom, entry + TWIN, addr);
        }
        at += data.len();
    }
    Ok(BankUsage {
        bank,
        used: need,
        capacity,
    })
}
