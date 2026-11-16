//! Where a level ends: its goal blocks. Touching block $4A ends the level (00:1CBA, game
//! mode $18); block $4B does the same and sets $A2B4 (the secret exit). The bell above a goal
//! is object kind $16 and leads to the bonus game (level $FF).
use crate::levels::{COLS, ROWS};
use crate::objects::Obj;

/// The levels that end with their boss: they have no goal block.
pub const BOSS_LEVELS: [usize; 7] = [5, 9, 13, 16, 19, 23, 24];
/// Object kind of the bell above a goal.
pub const BELL: u8 = 0x16;
/// How far left of and above the goal block a bell is looked for, in pixels (the game's are
/// 40-72 left and 88-200 above).
const BELL_REACH: (i32, i32) = (96, 224);
const BLOCK: i32 = 16;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Goal {
    Finish,
    Secret,
}

impl Goal {
    pub fn block(self) -> u8 {
        match self {
            Goal::Finish => 0x4A,
            Goal::Secret => 0x4B,
        }
    }
}

/// The cells (column, row) of `goal`'s blocks in a level map, top first.
pub fn cells(map: &[u8], goal: Goal) -> Vec<(usize, usize)> {
    let b = goal.block();
    let mut v: Vec<(usize, usize)> = map.iter().enumerate().filter(|&(_, &m)| m == b).map(|(i, _)| (i % COLS, i / COLS)).collect();
    v.sort_by_key(|&(c, r)| (r, c));
    v
}

/// The bell that belongs to a goal whose top block is at cell `at`.
pub fn bell(objs: &[Obj], at: (usize, usize)) -> Option<usize> {
    let (gx, gy) = (at.0 as i32 * BLOCK, at.1 as i32 * BLOCK);
    objs.iter().position(|o| {
        let (dx, dy) = (gx - o.x as i32, gy - o.y as i32);
        o.kind == BELL && (0..=BELL_REACH.0).contains(&dx) && (0..=BELL_REACH.1).contains(&dy)
    })
}

/// Put `goal` with its top block at cell `to` (a goal of several blocks keeps its shape; a
/// level without one gets one block). The cells it leaves get back `under` (what they had
/// before it came), or the block to their left when that is not known; `under` then holds
/// what the new cells had. The finish takes its bell along. False when nothing moved.
pub fn place(map: &mut [u8], objs: &mut [Obj], goal: Goal, to: (usize, usize), under: &mut Vec<u8>) -> bool {
    let b = goal.block();
    let old = cells(map, goal);
    let shape: Vec<(i32, i32)> = match old.first() {
        Some(&(c0, r0)) => old.iter().map(|&(c, r)| (c as i32 - c0 as i32, r as i32 - r0 as i32)).collect(),
        None => vec![(0, 0)],
    };
    let new: Vec<(usize, usize)> = shape.iter().map(|&(dc, dr)| (to.0 as i32 + dc, to.1 as i32 + dr)).filter(|&(c, r)| c >= 0 && r >= 0 && (c as usize) < COLS && (r as usize) < ROWS).map(|(c, r)| (c as usize, r as usize)).collect();
    if new.is_empty() || new == old {
        return false;
    }
    let bell = (goal == Goal::Finish).then(|| old.first().and_then(|&at| bell(objs, at))).flatten();
    for (k, &(c, r)) in old.iter().enumerate() {
        map[r * COLS + c] = match under.get(k) {
            Some(&u) => u,
            None if c > 0 => map[r * COLS + c - 1],
            None => 0,
        };
    }
    *under = new.iter().map(|&(c, r)| map[r * COLS + c]).collect();
    for &(c, r) in &new {
        map[r * COLS + c] = b;
    }
    if let (Some(i), Some(&from)) = (bell, old.first()) {
        let (dx, dy) = ((new[0].0 as i32 - from.0 as i32) * BLOCK, (new[0].1 as i32 - from.1 as i32) * BLOCK);
        let o = &mut objs[i];
        o.x = (o.x as i32 + dx).clamp(0, (COLS as i32 * BLOCK) - 8) as u16;
        o.y = (o.y as i32 + dy).clamp(0, (ROWS as i32 * BLOCK) - 8) as u16;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level() -> (Vec<u8>, Vec<Obj>) {
        let mut map = vec![0x10; COLS * ROWS];
        map[30 * COLS + 254] = 0x4A;
        map[30 * COLS + 253] = 0x22;
        let objs = vec![Obj { x: 4008, y: 360, kind: BELL, flag: false }, Obj { x: 100, y: 100, kind: 0x01, flag: false }];
        (map, objs)
    }

    #[test]
    fn the_finish_moves_with_its_bell_and_gives_back_its_cell() {
        let (mut map, mut objs) = level();
        let mut under = Vec::new();
        assert_eq!(bell(&objs, (254, 30)), Some(0));
        assert!(place(&mut map, &mut objs, Goal::Finish, (100, 20), &mut under));
        assert_eq!(cells(&map, Goal::Finish), vec![(100, 20)]);
        assert_eq!(map[30 * COLS + 254], 0x22, "the old cell gets the block to its left");
        assert_eq!((objs[0].x, objs[0].y), (4008 - 154 * 16, 360 - 160));
        assert_eq!((objs[1].x, objs[1].y), (100, 100));
        map[20 * COLS + 101] = 0x33;
        assert!(place(&mut map, &mut objs, Goal::Finish, (102, 20), &mut under));
        assert_eq!((map[20 * COLS + 100], map[20 * COLS + 102]), (0x10, 0x4A), "a later move gives back what the cell had");
        assert!(!place(&mut map, &mut objs, Goal::Finish, (102, 20), &mut under));
    }

    #[test]
    fn a_goal_of_two_blocks_keeps_its_shape_and_a_level_without_one_gets_one() {
        let mut map = vec![0x10; COLS * ROWS];
        map[29 * COLS + 126] = 0x4A;
        map[30 * COLS + 126] = 0x4A;
        let mut under = Vec::new();
        assert!(place(&mut map, &mut [], Goal::Finish, (50, 10), &mut under));
        assert_eq!(cells(&map, Goal::Finish), vec![(50, 10), (50, 11)]);
        assert!(cells(&map, Goal::Secret).is_empty());
        assert!(place(&mut map, &mut [], Goal::Secret, (60, 5), &mut Vec::new()));
        assert_eq!(cells(&map, Goal::Secret), vec![(60, 5)]);
    }
}
