//! A test game from the editor tries the level again: when Mario loses a life with lives left,
//! the game is back on the map and the level he was playing is entered again (with Play from
//! here, at the chosen place while it is still the level first entered).
use cart_model::GameMode;

/// A level to enter: its number (32 and up: a new level) and Mario's left x and feet y for
/// this entry only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub level: u8,
    pub at: Option<(u16, u16)>,
}

#[derive(Clone, Copy, Debug)]
pub struct Retry {
    entry: Entry,
    /// A life was being lost; it is acted on once the game shows the map.
    losing: bool,
}

impl Retry {
    /// The test game's first level.
    pub fn new(first: Entry) -> Retry {
        Retry { entry: first, losing: false }
    }

    /// The level being tried and where Mario begins it (the chosen place while it is the level
    /// first entered).
    pub fn entry(&self) -> Entry {
        self.entry
    }

    /// One frame: the game's mode, the level being played (when in a level) and the lives
    /// left. Returns the level to enter now.
    pub fn step(&mut self, mode: GameMode, playing: Option<u8>, lives: u8) -> Option<Entry> {
        if let Some(l) = playing.filter(|&l| l != self.entry.level) {
            self.entry = Entry { level: l, at: None };
        }
        match mode {
            GameMode::LosingLife => self.losing = true,
            GameMode::GameOver | GameMode::Playing => self.losing = false,
            GameMode::Map if self.losing => {
                self.losing = false;
                return (lives > 0).then_some(self.entry);
            }
            _ => {}
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use GameMode::*;

    fn run(r: &mut Retry, frames: &[(GameMode, Option<u8>, u8)]) -> Vec<Entry> {
        frames.iter().filter_map(|&(m, p, l)| r.step(m, p, l)).collect()
    }

    #[test]
    fn a_lost_life_enters_the_level_again_at_the_chosen_place() {
        let first = Entry { level: 3, at: Some((0x500, 0x1A0)) };
        let mut r = Retry::new(first);
        let died = [(Playing, Some(3), 5), (LosingLife, None, 5), (ToMap, None, 4), (Map, None, 4), (Map, None, 4)];
        assert_eq!(run(&mut r, &died), vec![first]);
        assert_eq!(run(&mut r, &died), vec![first], "every time");
    }

    #[test]
    fn game_over_finish_and_the_menu_stay_as_the_game_does() {
        let mut r = Retry::new(Entry { level: 3, at: None });
        assert!(run(&mut r, &[(Playing, Some(3), 1), (LosingLife, None, 1), (GameOver, None, 0), (Map, None, 0)]).is_empty(), "game over");
        assert!(run(&mut r, &[(Playing, Some(3), 1), (LosingLife, None, 1), (Map, None, 0)]).is_empty(), "no lives left");
        assert!(run(&mut r, &[(Playing, Some(3), 5), (ToMap, None, 5), (Map, None, 5)]).is_empty(), "finished, or back to the map from the menu");
    }

    #[test]
    fn another_level_entered_later_starts_at_its_own_start() {
        let mut r = Retry::new(Entry { level: 3, at: Some((0x500, 0x1A0)) });
        let died = [(Playing, Some(33), 5), (LosingLife, None, 5), (Map, None, 4)];
        assert_eq!(run(&mut r, &died), vec![Entry { level: 33, at: None }]);
    }
}
