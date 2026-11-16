//! Object pictures captured from the game in the background: threads with their own consoles
//! (one per spare processor core) capture every (level, object type) the cache does not have
//! yet, the shown level's first, so the window never waits for them, and the sprite tiles the
//! game's levels' own objects show over their animation (`art::object_tiles`, ten seconds
//! each): those right after the game's levels' own pictures, before every other type's. Each
//! level's start without objects, which every capture compares with, is played once and
//! shared (`art::Baseline`). The results are kept (`look_cache`).
use cart_model::art::{baseline, capture_look_with, object_tiles_with, Baseline, Look};
use cart_model::levels::LEVELS;
use cart_model::{objects, GameDriver};
use gb_core::Gb;
use std::collections::{HashSet, VecDeque};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex, OnceLock};

/// Frames an object's tiles are watched for.
const TILE_FRAMES: u32 = 600;
/// The capture consoles' sound rate: nobody hears it, and fewer samples are quicker.
const CAPTURE_SOUND_RATE: u32 = 1000;
/// Processor cores left to the window and the rest of the machine.
const SPARE_CORES: usize = 2;
const MOST_WORKERS: usize = 12;

pub struct LookLoader {
    want: Arc<Mutex<VecDeque<(usize, u8)>>>,
    /// Pictures the window asked for: they go before the tiles too.
    urgent: Arc<Mutex<HashSet<(usize, u8)>>>,
    got: Receiver<(usize, u8, Option<Look>)>,
    tiles: Receiver<(usize, u8, Vec<u8>)>,
    tiles_left: Arc<Mutex<usize>>,
    /// How many pictures and tiles there were to capture when it started.
    totals: (usize, usize),
}

impl LookLoader {
    /// Start capturing, in `rom`, what is not in `known`: each level's own object types
    /// first, then the tiles of the game's levels' own types not in `known_tiles`, then every
    /// type in every level (a picture asked for with `prefer` comes first all along).
    pub fn start(rom: Vec<u8>, original: Vec<u8>, known: &HashSet<(usize, u8)>, known_tiles: &HashSet<(usize, u8)>) -> LookLoader {
        let mut native: Vec<(usize, u8)> = (0..LEVELS).flat_map(|l| objects::read(&original, l).0.into_iter().map(move |o| (l, o.kind))).collect();
        let mut seen_tiles = known_tiles.clone();
        native.retain(|key| seen_tiles.insert(*key));
        let tiles_left = Arc::new(Mutex::new(native.len()));
        let tile_jobs = Arc::new(Mutex::new(native.into_iter()));
        let mut order: Vec<(usize, u8)> = (0..LEVELS).flat_map(|l| objects::read(&rom, l).0.into_iter().map(move |o| (l, o.kind))).collect();
        order.extend((0..LEVELS).flat_map(|l| (0..128u8).map(move |k| (l, k))));
        let mut seen = known.clone();
        order.retain(|key| seen.insert(*key));
        let own: Arc<HashSet<(usize, u8)>> = Arc::new((0..LEVELS).flat_map(|l| objects::read(&rom, l).0.into_iter().map(move |o| (l, o.kind))).collect());
        let urgent = Arc::new(Mutex::new(HashSet::new()));
        let want = Arc::new(Mutex::new(order.into_iter().collect::<VecDeque<_>>()));
        let baselines: Arc<Vec<OnceLock<Option<Baseline>>>> = Arc::new((0..LEVELS).map(|_| OnceLock::new()).collect());
        let (rom, original) = (Arc::new(rom), Arc::new(original));
        let (tx, got) = channel();
        let (tiles_tx, tiles) = channel();
        let workers = std::thread::available_parallelism().map(|n| n.get().saturating_sub(SPARE_CORES)).unwrap_or(1).clamp(1, MOST_WORKERS);
        for _ in 0..workers {
            let (queue, asked, left, jobs, own, baselines) = (want.clone(), urgent.clone(), tiles_left.clone(), tile_jobs.clone(), own.clone(), baselines.clone());
            let (rom, original, tx, tiles_tx) = (rom.clone(), original.clone(), tx.clone(), tiles_tx.clone());
            std::thread::spawn(move || {
                let Ok(mut g) = Gb::new(rom.to_vec(), CAPTURE_SOUND_RATE) else { return };
                if !g.boot_to_map() {
                    return;
                }
                let base = |l: usize| baselines[l].get_or_init(|| baseline(&g, &rom, &original, l, TILE_FRAMES)).as_ref();
                loop {
                    // a picture the window asked for, or a level's own, before the tiles
                    let tiles_done = left.lock().map(|n| *n == 0).unwrap_or(true);
                    let picture = queue.lock().ok().and_then(|mut q| {
                        let key = *q.front()?;
                        (own.contains(&key) || asked.lock().is_ok_and(|a| a.contains(&key)) || tiles_done).then(|| q.pop_front()).flatten()
                    });
                    if let Some((l, k)) = picture {
                        let look = base(l).and_then(|b| capture_look_with(&g, &rom, &original, l, k, b));
                        if tx.send((l, k, look)).is_err() {
                            return;
                        }
                        continue;
                    }
                    let job = jobs.lock().ok().and_then(|mut j| j.next());
                    match job {
                        Some((l, k)) => {
                            let t = base(l).map(|b| object_tiles_with(&g, &rom, &original, l, k, b)).unwrap_or_default();
                            if tiles_tx.send((l, k, t)).is_err() {
                                return;
                            }
                            if let Ok(mut n) = left.lock() {
                                *n -= 1;
                            }
                        }
                        // no tile left to start: the rest of the pictures, then stop
                        None if queue.lock().map(|q| q.is_empty()).unwrap_or(true) => return,
                        None => std::thread::sleep(std::time::Duration::from_millis(20)),
                    }
                }
            });
        }
        let totals = (want.lock().map(|q| q.len()).unwrap_or(0), tiles_left.lock().map(|n| *n).unwrap_or(0));
        LookLoader { want, urgent, got, tiles, tiles_left, totals }
    }

    /// How many objects' tiles are still to be captured.
    pub fn tiles_left(&self) -> usize {
        self.tiles_left.lock().map(|n| *n).unwrap_or(0)
    }

    /// The object tiles captured since the last call.
    pub fn take_tiles(&self) -> Vec<(usize, u8, Vec<u8>)> {
        self.tiles.try_iter().collect()
    }

    /// Capture these next (the shown level's), in this order.
    pub fn prefer(&self, keys: &[(usize, u8)]) {
        if let Ok(mut u) = self.urgent.lock() {
            u.extend(keys.iter().copied());
        }
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
        self.want.lock().map(|q| q.is_empty()).unwrap_or(true) && self.tiles_left.lock().map(|n| *n == 0).unwrap_or(true)
    }

    /// Pictures left and in all, tiles left and in all.
    pub fn progress(&self) -> ((usize, usize), (usize, usize)) {
        let pictures = self.want.lock().map(|q| q.len()).unwrap_or(0);
        ((pictures, self.totals.0), (self.tiles_left(), self.totals.1))
    }
}
