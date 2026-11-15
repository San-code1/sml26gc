//! Native Windows port of a Game Boy cartridge: the original program runs on a
//! Game Boy implemented in Rust, drawn and heard through macroquad and cpal.
//!
//! Layers: `domain` (rules, no I/O) ← `application` (session use case) ← `ports` (traits)
//! ← `adapters` (macroquad, cpal, files, Win32). This file is the composition root: it
//! reads the command line, picks the adapters and starts the window loop.
mod adapters;
mod application;
mod checks;
mod domain;
mod ports;
mod selftest;

use adapters::cpal_audio::Audio;
use adapters::fs_stores::{BatteryFile, DefaultSettings, ProgressFile, SettingsFile};
use adapters::macroquad_display::{save_window_png, MacroquadDisplay, MacroquadWindow};
use adapters::macroquad_keyboard::MacroquadKeyboard;
use adapters::win32;
use application::preferences::Preferences;
use application::session::{Flow, GameSession, SessionParts};
use domain::keys::Key;
use domain::map_view::MapView;
use ports::{AudioOut, Keyboard, SettingsStore};
use std::path::PathBuf;

#[derive(Default)]
struct Args {
    rom: Option<PathBuf>,
    screenshot: Option<PathBuf>,
    self_test: Option<PathBuf>,
    dump: Option<PathBuf>,
    script: String,
    frames: u32,
    every: u32,
    /// Start straight in this level (32 and up: the editor's new levels), as the level
    /// editor's Play button asks.
    level: Option<u8>,
}

fn parse_args(v: &[String]) -> Args {
    let mut a = Args { frames: 600, every: 60, ..Default::default() };
    let mut i = 0;
    while i < v.len() {
        let next = v.get(i + 1).cloned().unwrap_or_default();
        match v[i].as_str() {
            "--screenshot" => a.screenshot = Some(next.into()),
            "--self-test" => a.self_test = Some(next.into()),
            "--dump" => a.dump = Some(next.into()),
            "--script" => a.script = next,
            "--frames" => a.frames = next.parse().unwrap_or(600),
            "--every" => a.every = next.parse().unwrap_or(60),
            "--rom" => a.rom = Some(next.into()),
            "--level" => a.level = next.parse().ok(),
            s => {
                if !s.starts_with("--") {
                    a.rom = Some(s.into());
                }
                i += 1;
                continue;
            }
        }
        i += 2; // option and its value
    }
    a
}

const TITLE: &str = "Game Boy Native (Rust)";

fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.first().map(|s| s == "--research").unwrap_or(false) {
        let rom = host::require_rom(None).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1)
        });
        checks::research_main(&rom, &raw[1..]);
        return;
    }
    let args = parse_args(&raw);
    let found = host::require_rom(args.rom.clone());

    if let Some(out) = &args.self_test {
        let res = selftest::run(found.as_deref().ok());
        if let Err(e) = std::fs::write(out, res) {
            eprintln!("could not write {}: {e}", out.display());
            std::process::exit(1);
        }
        return;
    }
    if let Some(dir) = &args.dump {
        selftest::dump(found.as_deref().ok(), dir, &args.script, args.frames, args.every);
        return;
    }

    // A check run (--screenshot) is unattended: silent, default settings, no menu.
    let checking = args.screenshot.is_some();
    let title = if checking { format!("{TITLE} [check {}]", std::process::id()) } else { TITLE.to_string() };
    if checking {
        win32::remember_foreground();
    }
    let conf = macroquad::window::Conf {
        window_title: title.clone(),
        window_width: 640,
        window_height: 576,
        high_dpi: false,
        window_resizable: true,
        ..Default::default()
    };
    macroquad::Window::from_config(conf, run(args, found, checking, title));
}

/// Plug the adapters into a session, or keep the error to show it.
fn start(args: &Args, found: Result<PathBuf, String>, checking: bool, audio: &dyn AudioOut) -> Result<GameSession, (Preferences, String)> {
    let settings: Box<dyn SettingsStore> = if checking { Box::new(DefaultSettings) } else { Box::new(SettingsFile) };
    let prefs = Preferences::new(settings, Box::new(MacroquadWindow));
    let rom = match found {
        Ok(r) => r,
        Err(e) => return Err((prefs, e)),
    };
    let gb = match host::load_gb(&rom, audio.sample_rate(), !checking) {
        Ok(g) => g,
        Err(e) => return Err((prefs, e)),
    };
    let progress = ProgressFile::for_rom(&rom);
    let map_view = if checking { None } else { host::load_custom_map(&rom).map(|m| MapView::new(m, progress.load())) };
    let parts = SessionParts {
        prefs,
        saves: Box::new(BatteryFile { rom }),
        progress: Box::new(progress),
        map_view,
        start_level: args.level,
        checking,
    };
    Ok(GameSession::new(gb, parts))
}

async fn run(args: Args, found: Result<PathBuf, String>, checking: bool, title: String) {
    macroquad::input::prevent_quit();
    let audio = if checking { Audio::silent() } else { Audio::new() };
    let keyboard = MacroquadKeyboard;
    let mut display = MacroquadDisplay::new();
    let mut app = start(&args, found, checking, &audio);
    let mut render_frames = 0u32;

    loop {
        if checking && render_frames == 1 {
            win32::send_to_back(&title);
        }
        let dt = macroquad::time::get_frame_time();
        match &mut app {
            Ok(session) => {
                if session.frame(dt, &keyboard, &audio) == Flow::Quit {
                    break;
                }
                display.present(session.screen(), session.palette(), session.settings().integer_scale);
            }
            Err((prefs, error)) => {
                if keyboard.pressed(Key::F11) {
                    prefs.toggle_fullscreen();
                }
                display.show_error(error);
            }
        }

        if let Some(out) = &args.screenshot {
            render_frames += 1;
            if render_frames >= 120 {
                save_window_png(out);
                return;
            }
        }

        if macroquad::input::is_quit_requested() {
            break;
        }
        macroquad::window::next_frame().await;
    }
    if let Ok(session) = &mut app {
        session.finish();
    }
}
