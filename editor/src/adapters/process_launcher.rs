//! Starts `game.exe` (next to the editor) on the edited copy.
use crate::ports::GameLauncher;
use std::path::Path;

pub struct GameProcess;

impl GameLauncher for GameProcess {
    fn launch(&self, rom: &Path, level: Option<usize>) -> Result<(), String> {
        let game = host::game_exe();
        let mut cmd = std::process::Command::new(&game);
        cmd.arg(rom).arg("--cheats");
        if let Some(l) = level {
            cmd.arg("--level").arg(l.to_string());
        }
        cmd.spawn().map(|_| ()).map_err(|e| format!("{}: {e}", game.display()))
    }
}
