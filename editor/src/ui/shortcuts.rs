//! Every keyboard shortcut of the editor: the keys that run each panel action and the text
//! its button shows, from one table.
//!
//! Keys are read by their place on the keyboard (US layout), so only letters, comma, period
//! and named keys are used: they sit in the same place on a Spanish keyboard. Plus and minus
//! accept the keys that carry them on both layouts and the keypad.
use macroquad::prelude::{is_key_down, is_key_pressed, KeyCode};

/// Where the editor is: which keys mean what depends on it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ctx {
    Blocks,
    Objects,
    Picture,
    Points,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Undo,
    Redo,
    Save,
    Play,
    Grid,
    Pencil,
    BoxFill,
    Select,
    Overview,
    BlocksObjects,
    PrevLevel,
    NextLevel,
    NewLevel,
    DeleteLevel,
    WorldEditor,
    PrevMap,
    NextMap,
    SwitchTab,
    Original,
    ZoomOut,
    ZoomIn,
    Copy,
    Cut,
    Paste,
    ClearSelection,
    Escape,
    DeleteObject,
    PrevPoint,
    NextPoint,
    PrevOpens,
    NextOpens,
    PrevDoor,
    NextDoor,
    MakeExit1,
    MakeExit2,
    DeletePathRight,
    DeletePathLeft,
    DeletePathUp,
    DeletePathDown,
    NewPath,
    NewPoint,
    DeletePoint,
}

pub struct Shortcut {
    pub action: Action,
    ctxs: &'static [Ctx],
    ctrl: bool,
    shift: bool,
    keys: &'static [KeyCode],
    pub label: &'static str,
}

use Action::*;
use Ctx::*;
use KeyCode as K;

const ALL: &[Ctx] = &[Blocks, Objects, Picture, Points];
const LEVEL: &[Ctx] = &[Blocks, Objects];
const WORLD: &[Ctx] = &[Picture, Points];
const PAINT: &[Ctx] = &[Blocks, Picture];
const PALETTES: &[Ctx] = &[Blocks, Objects, Picture];

const fn s(action: Action, ctxs: &'static [Ctx], ctrl: bool, shift: bool, keys: &'static [KeyCode], label: &'static str) -> Shortcut {
    Shortcut { action, ctxs, ctrl, shift, keys, label }
}

pub const TABLE: &[Shortcut] = &[
    s(Undo, ALL, true, false, &[K::Z], "Ctrl+Z"),
    s(Redo, ALL, true, false, &[K::Y], "Ctrl+Y"),
    s(Redo, ALL, true, true, &[K::Z], "Ctrl+Shift+Z"),
    s(Save, ALL, true, false, &[K::S], "Ctrl+S"),
    s(Play, ALL, false, false, &[K::F5], "F5"),
    s(WorldEditor, ALL, false, false, &[K::F2], "F2"),
    s(Original, ALL, true, true, &[K::O], "Ctrl+Shift+O"),
    s(Grid, PALETTES, false, false, &[K::G], "G"),
    s(Pencil, PAINT, false, false, &[K::B], "B"),
    s(BoxFill, PAINT, false, false, &[K::R], "R"),
    s(Select, PAINT, false, false, &[K::E], "E"),
    s(Copy, PAINT, true, false, &[K::C], "Ctrl+C"),
    s(Cut, PAINT, true, false, &[K::X], "Ctrl+X"),
    s(Paste, PAINT, true, false, &[K::V], "Ctrl+V"),
    s(ClearSelection, PAINT, false, false, &[K::Delete], "Del"),
    s(Escape, PAINT, false, false, &[K::Escape], "Esc"),
    s(ZoomOut, PALETTES, true, false, &[K::Minus, K::Slash, K::KpSubtract], "Ctrl+-"),
    s(ZoomIn, PALETTES, true, false, &[K::Equal, K::RightBracket, K::KpAdd], "Ctrl++"),
    s(Overview, LEVEL, false, false, &[K::M], "M"),
    s(BlocksObjects, LEVEL, false, false, &[K::Tab, K::O], "Tab"),
    s(PrevLevel, LEVEL, false, false, &[K::PageUp], "PgUp"),
    s(NextLevel, LEVEL, false, false, &[K::PageDown], "PgDn"),
    s(NewLevel, LEVEL, true, false, &[K::N], "Ctrl+N"),
    s(DeleteLevel, LEVEL, true, false, &[K::Delete], "Ctrl+Del"),
    s(DeleteObject, &[Objects], false, false, &[K::Delete, K::Backspace], "Del"),
    s(PrevMap, WORLD, false, false, &[K::PageUp], "PgUp"),
    s(NextMap, WORLD, false, false, &[K::PageDown], "PgDn"),
    s(SwitchTab, WORLD, false, false, &[K::Tab], "Tab"),
    s(PrevPoint, &[Points], false, false, &[K::Comma], ","),
    s(NextPoint, &[Points], false, false, &[K::Period], "."),
    s(PrevOpens, &[Points], false, false, &[K::J], "J"),
    s(NextOpens, &[Points], false, false, &[K::K], "K"),
    s(PrevDoor, &[Points], false, false, &[K::U], "U"),
    s(NextDoor, &[Points], false, false, &[K::I], "I"),
    s(MakeExit1, &[Points], false, false, &[K::T], "T"),
    s(MakeExit2, &[Points], false, true, &[K::T], "Shift+T"),
    s(DeletePathRight, &[Points], false, true, &[K::Right], "Shift+Right"),
    s(DeletePathLeft, &[Points], false, true, &[K::Left], "Shift+Left"),
    s(DeletePathUp, &[Points], false, true, &[K::Up], "Shift+Up"),
    s(DeletePathDown, &[Points], false, true, &[K::Down], "Shift+Down"),
    s(NewPath, &[Points], false, false, &[K::P], "P"),
    s(NewPoint, &[Points], false, false, &[K::N], "N"),
    s(DeletePoint, &[Points], false, false, &[K::Delete], "Del"),
];

/// The actions whose keys went down this frame.
pub fn pressed(ctx: Ctx) -> Vec<Action> {
    let ctrl = is_key_down(K::LeftControl) || is_key_down(K::RightControl);
    let shift = is_key_down(K::LeftShift) || is_key_down(K::RightShift);
    TABLE.iter().filter(|s| s.ctxs.contains(&ctx) && s.ctrl == ctrl && s.shift == shift && s.keys.iter().any(|&k| is_key_pressed(k))).map(|s| s.action).collect()
}

/// The keys of an action, as its button shows them.
pub fn keys(action: Action) -> &'static str {
    TABLE.iter().find(|s| s.action == action).map(|s| s.label).unwrap_or("")
}

/// A button's text: its name and its keys.
pub fn with_keys(name: &str, action: Action) -> String {
    format!("{name} ({})", keys(action))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_combination_does_two_things_and_panning_keys_stay_free() {
        for ctx in [Blocks, Objects, Picture, Points] {
            let mut seen = Vec::new();
            for s in TABLE.iter().filter(|s| s.ctxs.contains(&ctx)) {
                for &k in s.keys {
                    let combo = (s.ctrl, s.shift, k);
                    assert!(!seen.contains(&combo), "{ctx:?}: {combo:?} twice");
                    seen.push(combo);
                    if !s.ctrl {
                        assert!(![K::W, K::A, K::S, K::D, K::Space].contains(&k), "{k:?} pans the view");
                        assert!(s.shift || ![K::Left, K::Right, K::Up, K::Down].contains(&k), "{k:?} pans the view");
                    }
                }
            }
        }
    }
}
