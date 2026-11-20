//! Reverse-engineering command line over the person's own cartridge.
//!
//!   research disasm BANK ADDR N | find SCRIPT | watch SCRIPT ADDRS FROM TO | trace ...
//!
//! Commands are rows of `COMMANDS`: a name and a function reading its arguments. They
//! only read the cartridge (and write pictures where asked); nothing here is used by the
//! game or the editor.
mod added;
mod blocks;
mod cheats;
mod checks;
mod console;
mod disasm;
mod levels;
mod memory;
mod probe;
mod rom_views;
mod testing;
mod zones;

use std::path::{Path, PathBuf};

/// A command's arguments: hex addresses, decimal counts, address lists, paths.
struct Args(Vec<String>);

impl Args {
    fn s(&self, i: usize) -> String {
        self.0.get(i).cloned().unwrap_or_default()
    }
    /// A 16-bit hex number ("0x" optional); 0 when missing or bad.
    fn hex(&self, i: usize) -> u16 {
        hex(&self.s(i))
    }
    /// A hex file offset.
    fn big(&self, i: usize) -> usize {
        usize::from_str_radix(self.s(i).trim_start_matches("0x"), 16).unwrap_or(0)
    }
    fn num(&self, i: usize) -> u32 {
        self.s(i).parse().unwrap_or(0)
    }
    fn path(&self, i: usize) -> PathBuf {
        PathBuf::from(self.s(i))
    }
    /// Hex addresses and ranges: "ffc2,a200-a20f".
    fn list(&self, i: usize) -> Vec<u16> {
        self.s(i)
            .split(',')
            .flat_map(|p| match p.split_once('-') {
                Some((a, b)) => (hex(a)..=hex(b)).collect::<Vec<u16>>(),
                None => vec![hex(p)],
            })
            .collect()
    }
}

fn hex(s: &str) -> u16 {
    u16::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0)
}

type Run = fn(&Path, &Args);

/// Argument 0 is the command name, so a command's own arguments start at 1.
const COMMANDS: &[(&str, Run)] = &[
    ("disasm", |r, a| disasm::disasm(r, a.hex(1) as usize, a.hex(2), a.num(3))),
    ("table", |r, a| disasm::table(r, a.big(1), a.big(2), a.num(3) as usize)),
    ("poke", |r, a| memory::poke(r, &a.s(1), a.num(2), &a.s(3), a.num(4), &a.path(5))),
    ("leave", |r, a| memory::leave(r, &a.s(1), a.num(2), &a.s(3), a.num(4), &a.path(5))),
    ("cmpmap", |r, a| memory::cmp_map(r, &a.s(1), a.num(2), &a.s(3), a.num(4), a.num(5))),
    ("diff", |r, a| memory::diff(r, &a.s(1), a.num(2), a.num(3), a.hex(4) as u8, a.hex(5) as u8)),
    ("worldshot", |r, a| rom_views::world_shot(r, a.big(1), a.big(2), a.num(3) as usize, &a.path(4))),
    ("looks", |r, a| rom_views::looks(r, a.num(1) as usize, &a.path(2))),
    ("worldcheck", |r, _| checks::world_check(r)),
    ("objcheck", |r, _| checks::objects_check(r)),
    ("objects", |r, a| rom_views::objects(r, a.num(1) as u8)),
    ("reads", |r, a| memory::reads(r, &a.s(1), a.hex(2) as usize, a.hex(3), a.hex(4), a.num(5), a.num(6))),
    ("free", |r, _| rom_views::free_space(r)),
    ("storecheck", |r, _| checks::store_check(r)),
    ("extracheck", |r, _| checks::extra_check(r)),
    ("routecheck", |r, _| checks::route_check(r)),
    ("dragcheck", |r, a| checks::drag_check(r, a.hex(1) as u8, a.num(2) as i32)),
    ("doortry", |r, a| checks::door_try(r, &a.s(1), a.hex(2) as u8, &a.s(3), &a.path(4))),
    ("doorcheck", |r, _| checks::door_check(r)),
    ("exitcheck", |r, a| checks::exit_check(r, a.hex(1) as u8, &a.s(2), &a.s(3))),
    ("levels", |r, _| checks::levels_check(r)),
    ("modes", |r, a| memory::modes(r, &a.s(1), &a.s(2))),
    ("find", |r, a| memory::find_player(r, &a.s(1))),
    ("watch", |r, a| memory::watch(r, &a.s(1), &a.list(2), a.num(3), a.num(4))),
    ("trace", |r, a| memory::trace(r, &a.s(1), &a.list(2), a.num(3), a.num(4))),
    ("mapafter", |r, a| rom_views::map_after(r, a.num(1) as u8, &a.path(2), (!a.s(3).is_empty()).then(|| a.hex(3) as u8))),
    ("mapscan", |r, a| rom_views::map_scan(r, &a.path(1))),
    ("savediff", |r, _| memory::save_diff(r)),
    ("pathscan", |r, a| rom_views::path_scan(r, &a.s(1))),
    ("unlock", |r, a| rom_views::unlock(r, a.hex(1) as u8, a.hex(2), a.hex(3))),
    ("walk", |r, a| rom_views::walk_test(r, (a.s(1) != "-").then(|| a.hex(1) as u8), &a.s(2), a.num(3))),
    ("clearscan", |r, _| rom_views::clear_scan(r)),
    ("cleardiff", |r, a| rom_views::clear_diff(r, a.num(1) as u8)),
    ("modetry", |r, a| rom_views::mode_try(r, a.num(1) as u8, a.hex(2) as u8, &a.s(3))),
    ("zonetry", |r, _| zones::zone_try(r)),
    ("zoneplaces", |r, _| zones::zone_places(r)),
    ("clearram", |r, _| zones::clear_ram(r)),
    ("ramfree", |r, _| zones::ram_free(r)),
    ("pointcheck", |r, _| added::point_check(r)),
    ("newzonecheck", |r, _| zones::new_zone_check(r)),
    ("sparecheck", |r, _| zones::spare_check(r)),
    ("startprobe", |r, _| levels::start_probe(r)),
    ("startcheck", |r, _| levels::start_check(r)),
    ("secretprobe", |r, _| levels::secret_probe(r)),
    ("finishcheck", |r, _| levels::finish_check(r)),
    ("secretcheck", |r, _| levels::secret_check(r)),
    ("walkprobe", |r, a| levels::walk_probe(r, a.s(1) == "own")),
    ("damageprobe", |r, a| cheats::damage_probe(r, a.num(1) as u8, a.num(2), a.num(3), &a.list(4))),
    ("pokeprobe", |r, a| cheats::poke_probe(r, a.num(1) as u8, a.hex(2), a.hex(3) as u8)),
    ("growprobe", |r, a| cheats::grow_probe(r, a.num(1) as u8, a.num(2), a.num(3))),
    ("slotsprobe", |r, a| cheats::slots_probe(r, a.num(1) as u8, a.num(2), &a.0[3..].iter().filter_map(|s| s.parse().ok()).collect::<Vec<u16>>())),
    ("itemprobe", |r, a| cheats::item_probe(r, a.num(1), a.num(2))),
    ("summonprobe", |r, a| cheats::summon_probe(r, a.num(1) as u8, &a.list(2))),
    ("lookprobe", |r, a| cheats::look_probe(r, a.num(1) as u8, a.hex(2) as u8, &a.list(3), &a.path(4))),
    ("timerprobe", |r, a| cheats::timer_probe(r, a.num(1) as u8)),
    ("finishprobe", |r, a| cheats::finish_probe(r, a.num(1) as u8, &a.s(2))),
    ("bellprobe", |r, a| cheats::bell_probe(r, a.num(1) as u8)),
    ("tilesprobe", |r, _a| cheats::tiles_probe(r)),
    ("patchprobe", |r, a| cheats::patch_probe(r, a.num(1) as u8, a.num(2), a.num(3), &a.s(4))),
    ("recordsprobe", |r, a| cheats::records_probe(r, a.num(1) as u8)),
    ("facingprobe", |r, a| cheats::facing_probe(r, a.num(1) as u8)),
    ("cheatcheck", |r, _a| cheats::cheat_check(r)),
    ("doorprobe", |r, a| cheats::door_probe(r, a.hex(1) as u8, a.num(2) == 1)),
    ("checkpointlist", |r, _a| testing::checkpoint_list(r)),
    ("classesprobe", |r, a| testing::classes_probe(r, a.0.get(1).and_then(|s| s.parse().ok()))),
    ("checkpointprobe", |r, a| testing::checkpoint_probe(r, a.num(1) as u8, a.num(2) as u16, a.num(3) as u16)),
    ("loadwatch", |r, a| testing::load_watch(r, a.num(1) as u8, &a.list(2))),
    ("headerreads", |r, a| testing::header_reads(r, a.num(1) as u8)),
    ("crowdprobe", |r, a| testing::crowd_probe(r, a.num(1) as u16)),
    ("crowdlist", |r, _a| testing::crowd_list(r)),
    ("flagsprobe", |r, a| testing::flags_probe(r, a.num(1) as u8)),
    ("openprobe", |r, a| testing::open_probe(r, a.num(1) == 1, a.num(2))),
    ("flagreaders", |r, _a| testing::flag_readers(r)),
    ("roomprobe", |r, a| testing::room_probe(r, a.hex(1) as u16, a.hex(2) as u16)),
    ("bellsprobe", |r, a| testing::bells_probe(r, a.num(1) as u16, a.num(2))),
    ("checkpointcheck", |r, _a| testing::checkpoint_check(r)),
    ("ways", |r, a| testing::ways(r, &a.list(1))),
    ("spotcheck", |r, _a| testing::spot_check(r)),
    ("comboprobe", |r, a| testing::combo_probe(r, a.num(1) as usize, a.num(2) as usize, a.num(3) == 1)),
    ("physicsprobe", |r, a| testing::physics_probe(r, a.num(1) as u8)),
    ("blockview", |r, a| blocks::block_view(r, a.num(1) as u8, a.num(2) as u16, a.num(3) as u16)),
    ("blockprobe", |r, a| blocks::block_probe(r, &if a.s(1) == "all" { Vec::new() } else { a.list(1) }, &if a.s(2).is_empty() { Vec::new() } else { a.list(2) })),
    ("tableprobe", |r, a| testing::table_probe(r, a.hex(1) as u16, a.num(2) as u8, a.num(3) as u8)),
    ("cameraprobe", |r, a| testing::camera_probe(r, a.num(1) as u8)),
    ("scrollprobe", |r, a| testing::scroll_probe(r, a.num(1) as u8, a.num(2) as u8, a.num(3) as u16, a.num(4) == 1)),
    ("animationprobe", |r, a| testing::animation_probe(r, a.num(1) as u8)),
    ("bossprobe", |r, a| testing::boss_probe(r, a.num(1) as u8, a.num(2) as usize, a.num(3))),
    ("tablescheck", |r, _a| testing::tables_check(r)),
    ("typegroups", |r, _a| testing::type_groups(r)),
    ("startobjects", |r, a| testing::start_objects_probe(r, a.num(1) as u8, a.hex(2) as u8, a.s(3).parse().unwrap_or(0), a.s(4).parse().unwrap_or(0), a.num(5) as u16, a.num(6) as u16)),
    ("firstscreen", |r, _a| testing::first_screen_objects(r)),
    ("titleframes", |r, a| testing::title_frames(r, a.num(1), &a.path(2))),
    ("levelobjects", |r, a| testing::level_objects_probe(r, a.num(1) as u8, a.num(2))),
    ("objecttiles", |r, _a| testing::object_tiles(r)),
    ("objectlist", |r, a| testing::object_list(r, &a.path(1))),
    ("walkright", |r, a| testing::walk_probe(r, a.num(1) as u8)),
    ("enemymix", |r, a| testing::enemy_mix(r, a.num(1) as usize, a.num(2) as usize, a.hex(3) as u8)),
    ("enemysetcheck", |r, _a| testing::enemy_set_check(r)),
    ("capturebench", |r, _a| testing::capture_bench(r)),
    ("cataloguereads", |r, a| testing::catalogue_reads(r, a.num(1) as u8, a.num(2))),
    ("mixprobe", |r, a| testing::mix_probe(r, a.num(1) as usize, a.hex(2) as u8, a.num(3) as usize)),
    ("unusedcheck", |r, a| testing::unused_check(r, a.hex(1) as u16, a.hex(2) as u16, a.0.get(3).and_then(|b| usize::from_str_radix(b, 16).ok()))),
    ("mixall", |r, a| testing::mix_all(r, a.num(1) as usize, a.num(2) as usize)),
    ("spawns", |r, _| testing::spawns(r)),
    ("stomp", |r, a| testing::stomp(r, a.num(1) as usize, a.hex(2) as u8)),
    ("slotwatch", |r, a| testing::slot_watch(r, a.num(1) as usize, a.hex(2) as u8, None)),
    ("killwatch", |r, a| testing::kill_watch(r, a.num(1) as usize, a.hex(2) as u8, 0)),
    ("spawncode", |r, a| testing::spawn_code(r, a.num(1) as usize, a.hex(2) as u8)),
    ("fulltiles", |r, _| testing::full_tiles(r)),
    ("mariooam", |r, a| testing::mario_oam(r, a.num(1) as usize)),
    ("addmember", |r, a| testing::add_member(r, &a.s(1), a.num(2) as usize, a.num(3) as usize, a.hex(4) as u8)),
    ("startpoint", |r, _| testing::start_point(r)),
    ("continueat", |r, a| testing::continue_at(r, a.hex(1) as u8)),
    ("newstart", |r, a| testing::new_start(r, a.hex(1) as u8)),
    ("newstartframes", |r, a| testing::new_start_frames(r, a.hex(1) as u8)),
    ("mapprobe", |r, a| testing::door_probe(r, a.hex(1) as u8, (a.num(2) > 0).then(|| a.num(2) as u8 - 1))),
    ("goalprobe", |r, a| levels::goal_probe(r, a.num(1) as u8, a.hex(2), a.hex(3), &a.s(4))),
];

const USAGE: &str = "research [--with-save]: disasm BANK ADDR N | table OFF LEN ROW | find SCRIPT | watch SCRIPT ADDRS FROM TO | trace SCRIPT ADDRS FROM TO\n\
                     \x20         poke | leave | cmpmap | diff | worldshot | looks | worldcheck | objcheck | objects | reads | free | storecheck | extracheck | routecheck | dragcheck | finishcheck | secretcheck | pointcheck | cheatcheck | walkprobe | ramfree | levels | modes ...\n\
                     RESEARCH_ROM=<file> uses that cartridge file\n\
                     (menushots belongs to the game: game --research menushots DIR)";

fn main() {
    let mut v: Vec<String> = std::env::args().skip(1).collect();
    if let Some(i) = v.iter().position(|a| a == "--with-save") {
        v.remove(i);
        console::use_battery_save(true);
    }
    let args = Args(v);
    let rom = host::require_rom(std::env::var_os("RESEARCH_ROM").map(std::path::PathBuf::from)).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1)
    });
    match COMMANDS.iter().find(|c| c.0 == args.s(0)) {
        Some((_, run)) => run(&rom, &args),
        None => eprintln!("{USAGE}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argument_forms() {
        let a = Args(["watch", "x", "ffc2,a200-a202", "0x10", "12"].map(String::from).to_vec());
        assert_eq!(a.list(2), vec![0xFFC2, 0xA200, 0xA201, 0xA202]);
        assert_eq!((a.hex(3), a.big(3), a.num(4), a.num(9)), (0x10, 0x10, 12, 0));
        assert_eq!(COMMANDS.len(), 117);
    }
}
