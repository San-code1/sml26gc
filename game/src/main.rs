//! Native Windows port of a Game Boy cartridge: the original program runs on a
//! Game Boy implemented in Rust, drawn and heard through macroquad and cpal.
mod audio;
mod mapview;
mod menu;
mod checks;
mod selftest;

pub use cart_model::GameDriver;
pub use gb_core::palette::{fb_to_rgba, PALETTES};
pub use gb_core::{ppu, Buttons, Gb, FRAME_SECONDS};
pub use host::{find_rom, load_gb, save_path};
use macroquad::prelude::*;
use std::path::{Path, PathBuf};

#[derive(Default)]
struct Args {
    rom: Option<PathBuf>,
    screenshot: Option<PathBuf>,
    self_test: Option<PathBuf>,
    dump: Option<PathBuf>,
    script: String,
    frames: u32,
    every: u32,
    /// Start straight in this level (0..=31), as the level editor's Play button asks.
    level: Option<u8>,
}

fn parse_args() -> Args {
    let mut a = Args { frames: 600, every: 60, ..Default::default() };
    let v: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < v.len() {
        let next = v.get(i + 1).cloned().unwrap_or_default();
        match v[i].as_str() {
            "--screenshot" => {
                a.screenshot = Some(next.into());
                i += 1;
            }
            "--self-test" => {
                a.self_test = Some(next.into());
                i += 1;
            }
            "--dump" => {
                a.dump = Some(next.into());
                i += 1;
            }
            "--script" => {
                a.script = next;
                i += 1;
            }
            "--frames" => {
                a.frames = next.parse().unwrap_or(600);
                i += 1;
            }
            "--every" => {
                a.every = next.parse().unwrap_or(60);
                i += 1;
            }
            "--rom" => {
                a.rom = Some(next.into());
                i += 1;
            }
            "--level" => {
                a.level = next.parse().ok();
                i += 1;
            }
            s if !s.starts_with("--") => a.rom = Some(s.into()),
            _ => {}
        }
        i += 1;
    }
    a
}

fn write_save(gb: &mut Gb, rom_path: &Path) {
    if !gb.cart().has_battery() || !gb.cart().needs_saving() {
        return;
    }
    let p = save_path(rom_path);
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if std::fs::write(&p, gb.cart().ram()).is_ok() {
        gb.cart_mut().mark_saved();
    }
}

fn read_keys(keys: &[KeyCode; 8]) -> Buttons {
    // Order matches menu::BUTTON_NAMES: Up, Down, Left, Right, A, B, Start, Select.
    const BITS: [Buttons; 8] = [Buttons::UP, Buttons::DOWN, Buttons::LEFT, Buttons::RIGHT, Buttons::A, Buttons::B, Buttons::START, Buttons::SELECT];
    let mut b = Buttons::NONE;
    for (k, bit) in keys.iter().zip(BITS) {
        if is_key_down(*k) {
            b |= bit;
        }
    }
    // A cartridge never sees opposite directions held together.
    if b & (Buttons::LEFT | Buttons::RIGHT) == Buttons::LEFT | Buttons::RIGHT {
        b &= !(Buttons::LEFT | Buttons::RIGHT);
    }
    if b & (Buttons::UP | Buttons::DOWN) == Buttons::UP | Buttons::DOWN {
        b &= !(Buttons::UP | Buttons::DOWN);
    }
    b
}

const TITLE: &str = "Game Boy Native (Rust)";

#[cfg(windows)]
mod win {
    use std::sync::atomic::{AtomicIsize, Ordering};
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    pub static PREV: AtomicIsize = AtomicIsize::new(0);
    pub fn remember_foreground() {
        unsafe { PREV.store(GetForegroundWindow() as isize, Ordering::SeqCst) }
    }
    /// Push our window behind the others and give focus back to whatever had it.
    pub fn send_to_back(title: &str) {
        let w: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
        unsafe {
            let hwnd = FindWindowW(std::ptr::null(), w.as_ptr());
            if !hwnd.is_null() {
                SetWindowPos(hwnd, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
            let prev = PREV.load(Ordering::SeqCst);
            if prev != 0 {
                SetForegroundWindow(prev as _);
            }
        }
    }
}

fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.first().map(|s| s == "--research").unwrap_or(false) {
        let rom = find_rom(None).expect("no cartridge");
        checks::research_main(&rom, &raw[1..]);
        return;
    }
    let args = parse_args();
    let rom = find_rom(args.rom.clone());

    if let Some(out) = &args.self_test {
        let res = selftest::run(rom.as_deref());
        if let Err(e) = std::fs::write(out, res) {
            eprintln!("could not write {}: {e}", out.display());
            std::process::exit(1);
        }
        return;
    }
    if let Some(dir) = &args.dump {
        selftest::dump(rom.as_deref(), dir, &args.script, args.frames, args.every);
        return;
    }

    let quiet = args.screenshot.is_some();
    let title = if quiet { format!("{TITLE} [check {}]", std::process::id()) } else { TITLE.to_string() };
    #[cfg(windows)]
    if quiet {
        win::remember_foreground();
    }
    let conf = Conf {
        window_title: title.clone(),
        window_width: 640,
        window_height: 576,
        high_dpi: false,
        window_resizable: true,
        ..Default::default()
    };
    macroquad::Window::from_config(conf, run(args, rom, title));
}

async fn run(args: Args, rom_path: Option<PathBuf>, title: String) {
    let quiet = args.screenshot.is_some();
    prevent_quit();
    let audio =if quiet { audio::Audio::silent() } else { audio::Audio::new() };
    let mut error = String::new();
    let mut gb = match &rom_path {
        Some(p) => match load_gb(p, audio.rate, !quiet) {
            Ok(g) => Some(g),
            Err(e) => {
                error = e;
                None
            }
        },
        None => {
            error = "No Game Boy cartridge file (.gb) was found.\nPut it in a 'rom' folder next to this program\nor pass its path on the command line.".into();
            None
        }
    };

    // Started from the level editor: go straight into the chosen level.
    if let (Some(n), Some(g)) = (args.level, gb.as_mut()) {
        if g.boot_to_map() {
            g.enter_level(n.min(31));
        }
    }

    let mut rgba = vec![0u8; ppu::W * ppu::H * 4];
    let tex = Texture2D::from_rgba8(ppu::W as u16, ppu::H as u16, &rgba);
    tex.set_filter(FilterMode::Nearest);
    let mut st = if quiet { menu::Settings::default() } else { menu::Settings::load() };
    if st.fullscreen && !quiet {
        set_fullscreen(true);
    }
    let mut acc = 0.0f64;
    let mut render_frames = 0u32;
    let mut save_timer = 0.0f32;
    let mut pause: Option<menu::Menu> = None;
    // After the menu closes, the game sees no buttons until every key is let go,
    // so the key that closed it doesn't also press a button in the game.
    let mut input_lock = false;
    let mut start_tap = 0u32;
    let mut hint_time = if quiet { 0.0 } else { 4.0f32 };
    let mut screen = vec![0u8; ppu::W * ppu::H];
    let mut prev_mode = 0u8;
    // The editor-made world map for this cartridge, if there is one (see mapview.rs).
    let mut mapview = if quiet { None } else { rom_path.as_ref().and_then(|p| mapview::MapView::load(p)) };
    // The level being played, to mark it cleared when the map comes back without a lost life.
    let mut playing: Option<u8> = None;
    let mut lost_life = false;
    let mut map_frames = 0u32;

    loop {
        if quiet && render_frames == 1 {
            #[cfg(windows)]
            win::send_to_back(&title);
        }
        if is_key_pressed(KeyCode::F11) {
            st.fullscreen = !st.fullscreen;
            set_fullscreen(st.fullscreen);
            st.save();
        }

        if let Some(g) = gb.as_mut() {
            // In a level ($FF9B = 4: game mode byte, 1A title, 0C map, 04 level) the Start key
            // opens this pause menu; elsewhere it is the game's own Start. Esc opens it anywhere.
            let mode = g.mode();
            let in_level = mode == 0x04;
            // Show the "ESC: menu / Enter: pause" tag again each time a level begins.
            if in_level && prev_mode != 0x04 && prev_mode != 0x08 && !quiet {
                hint_time = 3.0;
            }
            prev_mode = mode;
            // Follow the level being played: modes $09 (losing a life) and $05 (game over)
            // mean it wasn't cleared; reaching the map ($0C) otherwise means it was.
            match mode {
                0x04 if playing.is_none() => {
                    playing = Some(g.read(0xA269));
                    lost_life = false;
                }
                0x09 | 0x05 => lost_life = true,
                0x0C => {
                    if let (Some(l), Some(v)) = (playing, mapview.as_mut()) {
                        if !lost_life {
                            v.mark_cleared(l);
                        }
                    }
                    playing = None;
                }
                0x1A => playing = None,
                _ => {}
            }
            map_frames = if mode == 0x0C { map_frames + 1 } else { 0 };
            // The editor-made map takes over the world map screen once it has finished appearing.
            let custom = mode == 0x0C && map_frames > 20 && mapview.as_ref().map(|v| v.enabled).unwrap_or(false);
            if pause.is_none() && !quiet {
                let start_key = is_key_pressed(st.keys[6]) && in_level && !input_lock;
                if is_key_pressed(KeyCode::Escape) || start_key {
                    pause = Some(menu::Menu::new(mode, mapview.as_ref().map(|v| v.enabled)));
                }
            } else if let Some(m) = pause.as_mut() {
                match m.update(&mut st) {
                    menu::Action::None => {}
                    menu::Action::Close => {
                        pause = None;
                        input_lock = true;
                    }
                    menu::Action::PressStart => {
                        pause = None;
                        input_lock = true;
                        start_tap = 8;
                    }
                    menu::Action::Quit => break,
                    menu::Action::BackToMap => {
                        pause = None;
                        input_lock = true;
                        lost_life = true; // left on purpose: not cleared
                        g.back_to_map();
                    }
                    menu::Action::ToggleWorld => {
                        if let Some(v) = mapview.as_mut() {
                            v.enabled = !v.enabled;
                        }
                    }
                    menu::Action::FileSelect => {
                        pause = None;
                        input_lock = true;
                        g.back_to_file_select();
                        acc = 0.0;
                    }
                }
            }
            if quiet {
                // Checks: run the game a little faster so the title screen is up.
                for _ in 0..4 {
                    g.run_frame();
                }
                g.clear_audio();
            } else if pause.is_some() {
                // Paused: the game is frozen and silent.
                g.clear_audio();
                acc = 0.0;
            } else {
                let mut b = read_keys(&st.keys);
                let held_any = !b.is_empty() || is_key_down(KeyCode::Enter) || is_key_down(KeyCode::Space) || is_key_down(KeyCode::Escape);
                if input_lock {
                    if held_any {
                        b = Buttons::NONE;
                    } else {
                        input_lock = false;
                    }
                }
                if in_level {
                    b &= !Buttons::START;
                }
                if start_tap > 0 {
                    start_tap -= 1;
                    b |= Buttons::START;
                }
                if custom {
                    // Our map gets the buttons; the game's own map idles underneath (its music plays).
                    let v = mapview.as_mut().unwrap();
                    v.prepare(g);
                    let k = |i: usize| is_key_down(st.keys[i]) && !input_lock;
                    let enter = !input_lock && (is_key_pressed(st.keys[4]) || is_key_pressed(st.keys[6]));
                    if let Some(level) = v.update(k(2), k(3), k(0), k(1), enter) {
                        g.enter_level_anywhere(level);
                        acc = 0.0;
                        input_lock = true;
                    }
                    b = Buttons::NONE;
                }
                g.set_buttons(b);
                let fast = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
                acc += get_frame_time().min(0.1) as f64;
                // Keep the sound queue fed: nudge timing when it runs low or high.
                let q = audio.queued_seconds();
                let target = FRAME_SECONDS * if q < 0.03 { 0.9 } else if q > 0.09 { 1.1 } else { 1.0 };
                let mut ran = 0;
                while acc >= target && ran < 4 {
                    acc -= target;
                    g.run_frame();
                    ran += 1;
                }
                if fast {
                    for _ in 0..3 {
                        g.run_frame();
                    }
                    g.clear_audio();
                }
                audio.push(g.audio(), st.volume as f32 / 10.0);
                g.clear_audio();
                save_timer += get_frame_time();
                if save_timer > 5.0 {
                    save_timer = 0.0;
                    if let Some(p) = &rom_path {
                        write_save(g, p);
                    }
                }
            }
            screen.copy_from_slice(g.screen());
            if custom {
                if let Some(v) = &mapview {
                    v.draw(&mut screen, st.lang);
                }
            }
            if let Some(m) = &pause {
                m.draw(&mut screen, &st);
            } else if hint_time > 0.0 {
                hint_time -= get_frame_time();
                menu::draw_hint(&mut screen, st.lang);
            }
            fb_to_rgba(&screen, &PALETTES[st.palette % PALETTES.len()], &mut rgba);
            tex.update_from_bytes(ppu::W as u32, ppu::H as u32, &rgba);
        }

        clear_background(Color::from_hex(0x101010));
        if gb.is_some() {
            let sw = screen_width();
            let sh = screen_height();
            let fit = (sw / ppu::W as f32).min(sh / ppu::H as f32);
            let scale = if st.integer_scale { fit.floor().max(1.0) } else { fit };
            let scale = if scale * (ppu::W as f32) > sw || scale * (ppu::H as f32) > sh {
                (sw / ppu::W as f32).min(sh / ppu::H as f32)
            } else {
                scale
            };
            let w = ppu::W as f32 * scale;
            let h = ppu::H as f32 * scale;
            draw_texture_ex(
                &tex,
                ((sw - w) / 2.0).floor(),
                ((sh - h) / 2.0).floor(),
                WHITE,
                DrawTextureParams { dest_size: Some(vec2(w, h)), ..Default::default() },
            );
        } else {
            for (i, line) in error.lines().enumerate() {
                draw_text(line, 20.0, 60.0 + i as f32 * 30.0, 26.0, WHITE);
            }
        }

        if let Some(out) = &args.screenshot {
            render_frames += 1;
            if render_frames >= 120 {
                // Screen data is bottom-up and export_png writes it that way round,
                // so the saved picture comes out upright.
                let img = get_screen_data();
                if let Some(d) = out.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                img.export_png(&out.to_string_lossy());
                return;
            }
        }

        if is_quit_requested() {
            break;
        }
        next_frame().await;
    }
    if let (Some(g), Some(p)) = (gb.as_mut(), &rom_path) {
        write_save(g, p);
    }
}
