//! Starts `game.exe` (next to the editor) on the project's cartridge, as a test game, and
//! keeps its process to tell whether it is still open.
use crate::ports::{GameLauncher, TestGame};
use cart_model::start_point::Start;
use std::cell::RefCell;
use std::path::Path;
use std::process::Child;

#[derive(Default)]
pub struct GameProcess {
    last: RefCell<Option<Child>>,
}

impl GameLauncher for GameProcess {
    fn launch(&self, rom: &Path, level: Option<usize>, start: Option<(u16, u16)>, point: Option<Start>, game: TestGame) -> Result<(), String> {
        let exe = host::game_exe();
        let mut cmd = std::process::Command::new(&exe);
        cmd.arg(rom).arg("--cheats");
        cmd.arg("--test").arg("--lives").arg(game.lives.to_string()).arg("--coins").arg(game.coins.to_string()).arg("--power").arg(game.power.to_string());
        if let Some(l) = level {
            cmd.arg("--level").arg(l.to_string());
        }
        if let Some((x, feet)) = start {
            cmd.arg("--start").arg(format!("{x},{feet}"));
        }
        if let Some(p) = point {
            cmd.arg("--point").arg(match p.new_zone {
                Some(k) => format!("{:02X}@{k}", p.point),
                None => format!("{:02X}", p.point),
            });
        }
        let child = cmd.spawn().map_err(|e| format!("{}: {e}", exe.display()))?;
        *self.last.borrow_mut() = Some(child);
        Ok(())
    }

    fn running(&self) -> bool {
        let mut last = self.last.borrow_mut();
        let open = last.as_mut().is_some_and(|c| matches!(c.try_wait(), Ok(None)));
        if !open {
            *last = None;
        }
        open
    }
}
