//! Reverse-engineering command line over the person's own cartridge.
//!
//!   research disasm BANK ADDR N | find SCRIPT | watch SCRIPT ADDRS FROM TO | trace ...
//!
//! (Before the workspace split this was `gb_port --research ...`.)
mod research;

pub use cart_model::{levels, GameDriver};
pub use gb_core::palette::PALETTES;
pub use gb_core::{Buttons, Gb};
pub use host::load_gb;
use std::path::Path;

fn main() {
    let v: Vec<String> = std::env::args().skip(1).collect();
    let rom = host::require_rom(None).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1)
    });
    let hex = |s: &str| u16::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0);
    let list = |s: &str| {
        s.split(',')
            .flat_map(|p| match p.split_once('-') {
                Some((a, b)) => (hex(a)..=hex(b)).collect::<Vec<u16>>(),
                None => vec![hex(p)],
            })
            .collect::<Vec<u16>>()
    };
    let num = |s: &str| s.parse::<u32>().unwrap_or(0);
    let g = |i: usize| v.get(i).cloned().unwrap_or_default();
    let big = |s: &str| usize::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0);
    match g(0).as_str() {
        "disasm" => research::disasm(&rom, hex(&g(1)) as usize, hex(&g(2)), num(&g(3))),
        "table" => research::table(&rom, big(&g(1)), big(&g(2)), num(&g(3)) as usize),
        "poke" => research::poke(&rom, &g(1), num(&g(2)), &g(3), num(&g(4)), Path::new(&g(5))),
        "leave" => research::leave(&rom, &g(1), num(&g(2)), &g(3), num(&g(4)), Path::new(&g(5))),
        "cmpmap" => research::cmp_map(&rom, &g(1), num(&g(2)), &g(3), num(&g(4)), num(&g(5))),
        "diff" => research::diff(&rom, &g(1), num(&g(2)), num(&g(3)), hex(&g(4)) as u8, hex(&g(5)) as u8),
        "worldshot" => research::world_shot(&rom, big(&g(1)), big(&g(2)), num(&g(3)) as usize, Path::new(&g(4))),
        "looks" => research::looks(&rom, num(&g(1)) as usize, Path::new(&g(2))),
        "worldcheck" => research::world_check(&rom),
        "objcheck" => research::objects_check(&rom),
        "objects" => research::objects(&rom, num(&g(1)) as u8),
        "reads" => research::reads(&rom, &g(1), hex(&g(2)) as usize, hex(&g(3)), hex(&g(4)), num(&g(5)), num(&g(6))),
        "free" => research::free_space(&rom),
        "storecheck" => research::store_check(&rom),
        "levels" => research::levels_check(&rom),
        "modes" => research::modes(&rom, &g(1), &g(2)),
        "find" => research::find_player(&rom, &g(1)),
        "watch" => research::watch(&rom, &g(1), &list(&g(2)), num(&g(3)), num(&g(4))),
        "trace" => research::trace(&rom, &g(1), &list(&g(2)), num(&g(3)), num(&g(4))),
        _ => eprintln!(
            "research: disasm BANK ADDR N | table OFF LEN ROW | find SCRIPT | watch SCRIPT ADDRS FROM TO | trace SCRIPT ADDRS FROM TO\n\
             \x20         poke | leave | cmpmap | diff | worldshot | looks | worldcheck | objcheck | objects | reads | free | storecheck | levels | modes\n\
             (customcheck and menushots belong to the game: game --research ...)"
        ),
    }
}
