//! Level editor: opens the person's cartridge (or their edited copy), shows each of the
//! 32 levels with the game's own blocks and graphics, lets them paint blocks, place
//! objects and edit the world map, and saves into an edited copy. The original file is
//! never written.
//!
//! Layers: `domain` (document, undo history, tools) ← `application` (saving, captured
//! graphics) ← `ports` ← `adapters` (files, game.exe) and `ui` (macroquad panels). This
//! file is the composition root.
mod adapters;
mod application;
mod domain;
mod ports;
mod ui;

use adapters::file_dialogs::SystemDialogs;
use adapters::fs_repository;
use adapters::menu_bar::WindowMenu;
use adapters::process_launcher::GameProcess;
use cart_model::original_points::OnMap;
use cart_model::zone_maps::ZONES;
use macroquad::prelude::*;
use std::path::PathBuf;
use ui::{col, Editor, Mode, Ports, TAB_POINTS};

const TITLE: &str = "Level Editor (Rust)";

fn conf() -> Conf {
    Conf { window_title: TITLE.into(), window_width: 1280, window_height: 820, window_resizable: true, high_dpi: false, ..Default::default() }
}

/// Show a message until the window is closed.
async fn show_forever(text: &str, size: f32, line: f32) -> ! {
    loop {
        clear_background(col(3));
        for (i, l) in text.lines().enumerate() {
            draw_text(l, 40.0, 60.0 + i as f32 * line, size, col(0));
        }
        next_frame().await;
    }
}

#[macroquad::main(conf)]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |f: &str| args.iter().any(|a| a == f);
    let value = |f: &str| args.iter().position(|a| a == f).and_then(|i| args.get(i + 1));
    let shot = value("--screenshot").map(PathBuf::from);
    let level = value("--level").and_then(|v| v.parse::<usize>().ok());
    let rom_arg = args.iter().find(|a| !a.starts_with("--") && a.to_lowercase().ends_with(".gb")).map(PathBuf::from);

    clear_background(col(3));
    draw_text("Loading...", 40.0, 60.0, 30.0, col(0));
    next_frame().await;

    let rom_path = match host::require_rom(rom_arg) {
        Ok(p) => p,
        Err(e) => show_forever(&e, 30.0, 34.0).await,
    };
    let opened = match fs_repository::open(&rom_path) {
        Ok(o) => o,
        Err(e) => show_forever(&e, 26.0, 0.0).await,
    };
    let es = fs_repository::spanish();
    let ports = Ports {
        store: Box::new(opened.store),
        launcher: Box::new(GameProcess),
        prefs: Box::new(fs_repository::PrefsFile),
        dialogs: Box::new(SystemDialogs { es, owner: TITLE }),
        text_files: Box::new(fs_repository::TextFile),
        menu: Box::new(WindowMenu::new(TITLE)),
    };
    let mut ed = Editor::new(opened.doc, ports, opened.had_copy, es);

    ed.select_level(level.unwrap_or(0).min(ed.level_count() - 1));
    if flag("--objects") {
        ed.set_mode(Mode::Objects);
    }
    if flag("--world") {
        ed.enter_world(true);
        if flag("--points") {
            ed.world_tab = TAB_POINTS;
        }
        // --point HEX: choose that map point (its map is shown)
        if let Some(p) = value("--point").and_then(|v| usize::from_str_radix(v, 16).ok()).filter(|&p| p < cart_model::world::POINT_COUNT) {
            ed.choose_point(p);
        }
        // --map N: zone map N-1 (0 = the world map)
        if let Some(z) = value("--map").and_then(|v| v.parse::<usize>().ok()).filter(|&z| (1..=ZONES.len()).contains(&z)) {
            ed.show_map(OnMap::Zone(z - 1));
        }
    }
    if shot.is_some() && ed.mode == Mode::Objects {
        ed.capture_level_looks();
    }
    let mut frames = 0;
    loop {
        ed.frame();
        frames += 1;
        if let Some(p) = &shot {
            if frames >= 20 {
                if let Some(d) = p.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                get_screen_data().export_png(&p.to_string_lossy());
                return;
            }
        }
        next_frame().await;
    }
}
