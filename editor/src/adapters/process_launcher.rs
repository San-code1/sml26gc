//! Starts `game.exe` (next to the editor) on the project's cartridge, as a test game.
use crate::ports::GameLauncher;
use std::path::Path;

pub struct GameProcess;

impl GameLauncher for GameProcess {
    fn launch(&self, rom: &Path, level: Option<usize>, start: Option<(u16, u16)>, lives: u32, coins: u32) -> Result<(), String> {
        let game = host::game_exe();
        let mut cmd = std::process::Command::new(&game);
        cmd.arg(rom).arg("--cheats");
        cmd.arg("--test").arg("--lives").arg(lives.to_string()).arg("--coins").arg(coins.to_string());
        if let Some(l) = level {
            cmd.arg("--level").arg(l.to_string());
        }
        if let Some((x, feet)) = start {
            cmd.arg("--start").arg(format!("{x},{feet}"));
        }
        cmd.spawn().map(|_| ()).map_err(|e| format!("{}: {e}", game.display()))
    }
}
