//! Undo / redo of whole states, for anything that can be cloned (a level's blocks and
//! objects, the world map picture...).

/// How many steps back are kept.
pub const LIMIT: usize = 100;

pub struct History<T> {
    undo: Vec<T>,
    redo: Vec<T>,
}

impl<T> Default for History<T> {
    fn default() -> Self {
        History {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }
}

impl<T> History<T> {
    /// Remember `state` before changing it. Anything redoable is forgotten.
    pub fn record(&mut self, state: T) {
        self.undo.push(state);
        if self.undo.len() > LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Go one step back; `current` becomes redoable. False when there is nothing to undo.
    pub fn undo(&mut self, current: &mut T) -> bool {
        match self.undo.pop() {
            Some(s) => {
                self.redo.push(std::mem::replace(current, s));
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self, current: &mut T) -> bool {
        match self.redo.pop() {
            Some(s) => {
                self.undo.push(std::mem::replace(current, s));
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn back_and_forth() {
        let mut h = History::default();
        let mut v = 1;
        h.record(v);
        v = 2;
        h.record(v);
        v = 3;
        assert!(h.undo(&mut v) && v == 2);
        assert!(h.undo(&mut v) && v == 1);
        assert!(!h.undo(&mut v));
        assert!(h.redo(&mut v) && v == 2);
        h.record(v); // a new change forgets the redo
        assert!(!h.redo(&mut v));
    }

    #[test]
    fn keeps_the_last_hundred() {
        let mut h = History::default();
        let mut v = 0;
        for i in 0..150 {
            h.record(i);
        }
        let mut steps = 0;
        while h.undo(&mut v) {
            steps += 1;
        }
        assert_eq!((steps, v), (LIMIT, 50));
    }
}
