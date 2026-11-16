//! Object pictures captured from the game in the background: a thread with its own console
//! captures every (level, object type) the cache does not have yet, the shown level's first,
//! so the window never waits for them. Each capture plays the game for a few frames, which is
//! why they are many seconds of work in all and are kept (`look_cache`).
use cart_model::art::{capture_look, Look};
use cart_model::levels::LEVELS;
use cart_model::{objects, GameDriver};
use gb_core::Gb;
use std::collections::{HashSet, VecDeque};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

pub struct LookLoader {
    want: Arc<Mutex<VecDeque<(usize, u8)>>>,
    got: Receiver<(usize, u8, Option<Look>)>,
}

impl LookLoader {
    /// Start capturing, in `rom`, what is not in `known`: each level's own object types
    /// first, then every type in every level.
    pub fn start(rom: Vec<u8>, original: Vec<u8>, known: &HashSet<(usize, u8)>) -> LookLoader {
        let mut order: Vec<(usize, u8)> = (0..LEVELS).flat_map(|l| objects::read(&rom, l).0.into_iter().map(move |o| (l, o.kind))).collect();
        order.extend((0..LEVELS).flat_map(|l| (0..128u8).map(move |k| (l, k))));
        let mut seen = known.clone();
        order.retain(|key| seen.insert(*key));
        let want = Arc::new(Mutex::new(order.into_iter().collect::<VecDeque<_>>()));
        let (tx, got) = channel();
        let queue = want.clone();
        std::thread::spawn(move || {
            let Ok(mut g) = Gb::new(rom.clone(), 44100) else { return };
            if !g.boot_to_map() {
                return;
            }
            loop {
                let next = queue.lock().ok().and_then(|mut q| q.pop_front());
                let Some((l, k)) = next else { return };
                if tx.send((l, k, capture_look(&g, &rom, &original, l, k))).is_err() {
                    return;
                }
            }
        });
        LookLoader { want, got }
    }

    /// Capture these next (the shown level's), in this order.
    pub fn prefer(&self, keys: &[(usize, u8)]) {
        let Ok(mut q) = self.want.lock() else { return };
        for key in keys.iter().rev() {
            if let Some(i) = q.iter().position(|k| k == key) {
                q.remove(i);
                q.push_front(*key);
            }
        }
    }

    /// The pictures captured since the last call.
    pub fn take(&self) -> Vec<(usize, u8, Option<Look>)> {
        self.got.try_iter().collect()
    }

    /// Wait for the next picture (the check pictures need the shown level's at once).
    pub fn wait(&self) -> Option<(usize, u8, Option<Look>)> {
        self.got.recv_timeout(std::time::Duration::from_secs(5)).ok()
    }

    /// Nothing is left to capture.
    pub fn done(&self) -> bool {
        self.want.lock().map(|q| q.is_empty()).unwrap_or(true)
    }
}
