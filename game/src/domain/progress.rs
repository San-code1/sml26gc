//! Following the level being played, to know when one has been cleared: reaching the map
//! again counts, unless a life was lost (or the level was left from the menu) on the way.
use cart_model::GameMode;

#[derive(Default)]
pub struct LevelTracker {
    playing: Option<u8>,
    lost_life: bool,
}

impl LevelTracker {
    /// Look at this frame's mode. `level` reads the level number (asked only when a level
    /// starts). Returns the level just cleared, if any.
    pub fn observe(&mut self, mode: GameMode, level: impl FnOnce() -> u8) -> Option<u8> {
        match mode {
            GameMode::Playing if self.playing.is_none() => {
                self.playing = Some(level());
                self.lost_life = false;
            }
            GameMode::LosingLife | GameMode::GameOver => self.lost_life = true,
            GameMode::Map => {
                let cleared = self.playing.filter(|_| !self.lost_life);
                self.playing = None;
                return cleared;
            }
            GameMode::FileSelect => self.playing = None,
            _ => {}
        }
        None
    }

    /// The level was left on purpose (back to the map from the menu): not cleared.
    pub fn abandon(&mut self) {
        self.lost_life = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleared_only_without_a_lost_life() {
        let mut t = LevelTracker::default();
        assert_eq!(t.observe(GameMode::Playing, || 6), None);
        assert_eq!(t.observe(GameMode::Playing, || unreachable!()), None);
        assert_eq!(t.observe(GameMode::Map, || 0), Some(6));
        assert_eq!(t.observe(GameMode::Map, || 0), None);

        t.observe(GameMode::Playing, || 3);
        t.observe(GameMode::LosingLife, || 0);
        t.observe(GameMode::Playing, || 0);
        assert_eq!(t.observe(GameMode::Map, || 0), None);

        t.observe(GameMode::Playing, || 4);
        t.abandon();
        assert_eq!(t.observe(GameMode::Map, || 0), None);
    }
}
