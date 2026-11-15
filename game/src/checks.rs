//! Checks of the game's own additions, run from the command line without a window:
//!   game --research customcheck OUT.png   the editor-made world map, walked and entered (and a door
//!                                         to a zone map, drawn into OUT_zone.png)
//!   game --research menushots DIR         the pause menu's screens in every language
use crate::domain::map_view::MapView;
use crate::domain::menu::Menu;
use crate::domain::settings::Settings;
use cart_model::GameDriver;
use gb_core::palette::PALETTES;
use host::load_gb;
use cart_model::custom_map::{CustomMap, Point};
use gb_core::script::{buttons_at, parse_script};
use host::png_out::{write_rgba, write_screen};
use std::path::Path;

/// The port's own map: write a three-point map next to a temporary copy of the cartridge,
/// walk right with the d-pad, press A and check the game loads that point's level.
pub fn custom_check(rom_path: &Path, png: &Path) {
    let dir = std::env::temp_dir().join("gb_port_custom_check");
    let _ = std::fs::create_dir_all(&dir);
    let rom_copy = dir.join("check.gb");
    let _ = std::fs::copy(rom_path, &rom_copy);
    let map = CustomMap {
        points: vec![Point::new(200, 260, Some(0)), Point::new(280, 250, Some(6)), Point::new(280, 330, None)],
        paths: vec![(0, 1), (1, 2)],
        start: 0,
    };
    host::save_custom_map(&map, &rom_copy).unwrap();
    let loaded = host::load_custom_map(&rom_copy).expect("map file");
    let mut v = MapView::new(loaded, host::progress::load(&host::progress_path(&rom_copy)));
    let parsed_same = v.map == map;
    let Ok(mut g) = load_gb(&rom_copy, 44100, false) else { return };
    g.boot_to_map();
    let on_map = g.clone();
    v.prepare(&g);
    let mut fb = vec![0u8; 160 * 144];
    let mut entered = None;
    for f in 0..200 {
        let right = f < 5;
        let enter = f == 150;
        if let Some(l) = v.update(false, right, false, false, enter) {
            entered = Some(l);
            break;
        }
    }
    v.draw(&mut fb, 0);
    // saved 4x bigger so the small marks are easy to see
    let k = 4;
    let (w, h) = (160 * k, 144 * k);
    let mut rgba = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let c = PALETTES[0][fb[y / k * 160 + x / k] as usize & 3];
            let p = (y * w + x) * 4;
            rgba[p..p + 4].copy_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8, 255]);
        }
    }
    if let Err(e) = write_rgba(png, w, h, &rgba) {
        eprintln!("{e}");
    }
    let ok = entered.map(|l| g.enter_level_anywhere(l)).unwrap_or(false);
    let rom = g.cart().rom().to_vec();
    let (_, s) = cart_model::levels::map_location(&rom, cart_model::levels::header(&rom, 6));
    let same = cart_model::live::map_in_memory(&g) == cart_model::levels::decode(&rom, s).0;
    println!("map file read back identical={parsed_same}; walked to point {} ; A asked for level {entered:?}; game loaded it={ok}; level 6 blocks in memory={same}", v.cur);
    let _ = std::fs::remove_dir_all(&dir);

    // a door from the world map to the Tree Zone's map, and a level entered there
    let tree = cart_model::original_points::OnMap::Zone(0);
    let door = CustomMap { points: vec![Point::new(200, 260, None), Point { x: 76, y: 30, level: Some(5), map: tree }], paths: vec![(0, 1)], start: 0 };
    let mut z = MapView::new(door, Default::default());
    z.prepare(&on_map);
    let through = z.update(false, false, false, false, true).is_none() && z.cur == 1;
    z.prepare(&on_map);
    let level = z.update(false, false, false, false, true);
    let mut zf = vec![0u8; 160 * 144];
    z.draw(&mut zf, 0);
    let drawn = zf.iter().any(|&p| p != zf[0]);
    let zone_png = png.with_file_name(format!("{}_zone.png", png.file_stem().unwrap_or_default().to_string_lossy()));
    if let Err(e) = write_screen(&zone_png, &zf, &PALETTES[0]) {
        eprintln!("{e}");
    }
    println!("door: went through={through}; zone map drawn={drawn}; A there asked for level {level:?}");
}

/// Save the pause menu's screens over a level frame as PNGs into `dir`.
pub fn menu_shots(rom: &Path, dir: &Path) {
    let Ok(mut gb) = load_gb(rom, 44100, false) else { return };
    let sc = parse_script("S@400-405;S@600-605;S@800-805;R@960-1000;");
    for f in 0..1010 {
        gb.set_buttons(buttons_at(&sc, f));
        gb.run_frame();
        gb.clear_audio();
    }
    let _ = std::fs::create_dir_all(dir);
    let mut st = Settings::default();
    for (lang, tag) in ["en", "es", "ja"].iter().enumerate() {
        st.lang = lang;
        for name in ["main", "controls", "wait", "display", "sound", "language", "confirm"] {
            let mut screen = *gb.screen();
            Menu::preview(name).draw(&mut screen, &st);
            if let Err(e) = write_screen(&dir.join(format!("{tag}-{name}.png")), &screen, &PALETTES[0]) {
                eprintln!("{e}");
            }
        }
    }
}

pub fn research_main(rom: &Path, v: &[String]) {
    let g = |i: usize| v.get(i).cloned().unwrap_or_default();
    match g(0).as_str() {
        "customcheck" => custom_check(rom, Path::new(&g(1))),
        "menushots" => menu_shots(rom, Path::new(&g(1))),
        _ => eprintln!("game --research customcheck OUT.png | menushots DIR   (other research commands: research.exe)"),
    }
}
