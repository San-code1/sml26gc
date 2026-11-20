//! The editor's own names for the game's objects, written by hand from the research's
//! pictures (`research objectlist`): what each object type is in a level, by object catalogue
//! (levels 0-9 and 10-31, `level_graphics`). Descriptions, not cartridge data; a type without
//! an entry shows its number.

/// What kind of object it is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    PowerUp,
    Enemy,
    Trap,
    Platform,
    /// The finish and midway bells.
    Goal,
    /// Draws nothing: something the level's code reads.
    Marker,
    /// Boss rooms and other objects that act on their own script.
    Scripted,
    /// Draws nothing: what the block under it gives when hit from below (`hidden`).
    BlockContent,
}

pub const CATEGORIES: [Category; 8] = [Category::PowerUp, Category::BlockContent, Category::Enemy, Category::Trap, Category::Platform, Category::Goal, Category::Marker, Category::Scripted];

use Category::*;

/// (second catalogue, type, levels it applies to (empty: every level of the catalogue), name,
/// category). The first matching entry names a (level, type).
const NAMES: &[(bool, u8, &[usize], &str, Category)] = &[
    // both catalogues
    (false, 0x0F, &[], "Heart (extra life)", PowerUp),
    (true, 0x0F, &[], "Heart (extra life)", PowerUp),
    (false, 0x16, &[], "Finish bell", Goal),
    (true, 0x16, &[], "Finish bell", Goal),
    (false, 0x17, &[], "Midway bell", Goal),
    (true, 0x17, &[19], "Boss room object", Scripted),
    (true, 0x17, &[], "Midway bell", Goal),
    (false, 0x1B, &[], "Mushroom", PowerUp),
    (true, 0x1B, &[19], "Boss room object", Scripted),
    (true, 0x1B, &[], "Mushroom", PowerUp),
    (false, 0x1C, &[], "Fire flower", PowerUp),
    (true, 0x1C, &[], "Fire flower", PowerUp),
    (false, 0x1D, &[], "Carrot", PowerUp),
    (true, 0x1D, &[], "Carrot", PowerUp),
    (true, 0x1E, &[], "Star", PowerUp),
    (false, 0x11, &[], "Hidden mushroom", BlockContent),
    (true, 0x11, &[], "Hidden mushroom", BlockContent),
    (false, 0x12, &[], "Hidden fire flower", BlockContent),
    (true, 0x12, &[], "Hidden fire flower", BlockContent),
    (false, 0x13, &[], "Hidden carrot", BlockContent),
    (true, 0x13, &[], "Hidden carrot", BlockContent),
    (false, 0x14, &[], "Hidden star", BlockContent),
    (true, 0x14, &[], "Hidden star", BlockContent),
    (false, 0x15, &[], "Hidden heart", BlockContent),
    (true, 0x15, &[], "Hidden heart", BlockContent),
    (false, 0x19, &[2], "Hidden 50 coins", BlockContent),
    (false, 0x19, &[], "Invisible marker", Marker),
    (true, 0x19, &[], "Invisible marker", Marker),
    // levels 0-9
    (false, 0x01, &[], "Koopa Troopa", Enemy),
    (false, 0x08, &[], "Paragoomba", Enemy),
    (false, 0x3A, &[], "Paragoomba", Enemy),
    (false, 0x09, &[], "Goomba", Enemy),
    (false, 0x0C, &[], "Piranha Plant", Enemy),
    (false, 0x07, &[], "Leafy plant", Enemy),
    (false, 0x0D, &[], "Biting plant", Enemy),
    (false, 0x1F, &[], "Frog", Enemy),
    (false, 0x20, &[], "Leaf", Platform),
    (false, 0x21, &[], "Porcupine", Enemy),
    (false, 0x22, &[], "Bee", Enemy),
    (false, 0x28, &[], "Lift", Platform),
    (false, 0x2A, &[], "Lift", Platform),
    (false, 0x2E, &[], "Lift", Platform),
    (false, 0x30, &[], "Lift", Platform),
    (false, 0x29, &[], "Long lift", Platform),
    (false, 0x2D, &[], "Long lift", Platform),
    (false, 0x44, &[], "Beetle", Enemy),
    (false, 0x58, &[], "Crowned fish", Enemy),
    (false, 0x35, &[], "Invisible marker", Marker),
    (false, 0x3E, &[], "Invisible marker", Marker),
    (false, 0x40, &[], "Winged king", Enemy),
    (false, 0x41, &[], "Snail", Enemy),
    (false, 0x42, &[], "Caterpillar", Enemy),
    (false, 0x43, &[], "Egg", Enemy),
    (false, 0x33, &[], "Giant bug", Enemy),
    (false, 0x34, &[], "Fly", Enemy),
    (false, 0x5D, &[], "Ant", Enemy),
    (false, 0x37, &[], "Owl", Enemy),
    (false, 0x38, &[], "Cloud", Platform),
    (false, 0x39, &[], "Bird", Enemy),
    (false, 0x3D, &[], "Crowned bird", Enemy),
    (false, 0x4D, &[], "Boo", Enemy),
    (false, 0x4E, &[], "Falling spike", Trap),
    (false, 0x4F, &[9], "Spark", Enemy),
    (false, 0x4F, &[], "Spike ball", Trap),
    (false, 0x51, &[], "Masked ghost", Enemy),
    (false, 0x53, &[], "Goblin", Enemy),
    (false, 0x54, &[], "One-eyed ghost", Enemy),
    (false, 0x55, &[], "Spirit", Enemy),
    (false, 0x56, &[], "Candle", Enemy),
    (false, 0x5E, &[], "Cheep Cheep", Enemy),
    (false, 0x5F, &[], "Cheep Cheep", Enemy),
    (false, 0x57, &[], "Jar", Enemy),
    (false, 0x5A, &[], "Witch's broom", Enemy),
    (false, 0x5C, &[], "Fireball", Enemy),
    // levels 10-31
    (true, 0x01, &[], "Koopa Troopa", Enemy),
    (true, 0x09, &[25], "Invisible marker", Marker),
    (true, 0x09, &[], "Goomba", Enemy),
    (true, 0x08, &[25], "Fireball", Enemy),
    (true, 0x08, &[], "Paragoomba", Enemy),
    (true, 0x3A, &[], "Paragoomba", Enemy),
    (true, 0x3E, &[], "Grated lift", Platform),
    (true, 0x3F, &[], "Invisible marker", Marker),
    (true, 0x40, &[], "Screw", Platform),
    (true, 0x41, &[], "Spinning blade", Trap),
    (true, 0x50, &[], "Spinning blade", Trap),
    (true, 0x4B, &[], "Spiked pillar", Trap),
    (true, 0x44, &[], "Ball rider", Enemy),
    (true, 0x4D, &[18], "Space hippo", Enemy),
    (true, 0x4D, &[13], "Bomb bat", Enemy),
    (true, 0x4D, &[], "Pig", Enemy),
    (true, 0x47, &[], "Bomb", Enemy),
    (true, 0x48, &[], "Toy soldier", Enemy),
    (true, 0x49, &[], "Lever", Platform),
    (true, 0x51, &[], "Invisible marker", Marker),
    (true, 0x53, &[], "Bullet Bill", Enemy),
    (true, 0x1F, &[], "Coin bag", PowerUp),
    (true, 0x43, &[], "Spike flower", Trap),
    (true, 0x4A, &[], "Spiked dome", Trap),
    (true, 0x4C, &[], "Big spiked dome", Trap),
    (true, 0x05, &[], "Cheep Cheep", Enemy),
    (true, 0x06, &[], "Shark", Enemy),
    (true, 0x07, &[], "Sea creature", Enemy),
    (true, 0x3D, &[], "Sea creature", Enemy),
    (true, 0x0B, &[], "Eel", Enemy),
    (true, 0x21, &[16], "Fin", Trap),
    (true, 0x21, &[], "Blade", Trap),
    (true, 0x2B, &[], "Long lift", Platform),
    (true, 0x2C, &[], "Long lift", Platform),
    (true, 0x2F, &[], "Long lift", Platform),
    (true, 0x55, &[], "Bone fish", Enemy),
    (true, 0x56, &[], "Urchin", Enemy),
    (true, 0x20, &[], "Ammonite", Enemy),
    (true, 0x18, &[], "Bubble", Enemy),
    (true, 0x39, &[], "Flying fish", Enemy),
    (true, 0x57, &[], "Sea urchin", Enemy),
    (true, 0x5B, &[], "Fish", Enemy),
    (true, 0x58, &[], "Helmet alien", Enemy),
    (true, 0x5A, &[], "Eye stalk alien", Enemy),
    (true, 0x5C, &[], "Boss room object", Scripted),
    (true, 0x5E, &[], "Boss room object", Scripted),
    (true, 0x5F, &[], "Debris", Scripted),
    (true, 0x22, &[], "Ant", Enemy),
    (true, 0x5D, &[], "Ant", Enemy),
    (true, 0x23, &[], "Ant with ball", Enemy),
    (true, 0x25, &[], "Red ant", Enemy),
    (true, 0x27, &[], "Soldier ant", Enemy),
    (true, 0x2E, &[30], "Invisible marker", Marker),
    (true, 0x2E, &[], "Lift", Platform),
    (true, 0x0C, &[23], "Invisible marker", Marker),
    (true, 0x0C, &[], "Piranha Plant", Enemy),
    (true, 0x0D, &[], "Biting plant", Enemy),
    (true, 0x33, &[], "Giant bug", Enemy),
    (true, 0x34, &[], "Fly", Enemy),
    (true, 0x42, &[], "Spitting plant", Enemy),
    (true, 0x35, &[], "Invisible marker", Marker),
    (true, 0x60, &[], "Wario room lift", Platform),
    (true, 0x61, &[], "Wario room lift", Platform),
    (true, 0x62, &[], "Wario room object", Scripted),
    (true, 0x63, &[], "Wario room object", Scripted),
    (true, 0x64, &[], "Rubble", Scripted),
    (true, 0x66, &[], "Falling blocks", Trap),
    (true, 0x67, &[], "Spark", Enemy),
    (true, 0x68, &[], "Wario room object", Scripted),
    (true, 0x69, &[], "Wario room object", Scripted),
    (true, 0x6A, &[], "Wario room object", Scripted),
    (true, 0x6B, &[], "Wario room object", Scripted),
    (true, 0x6E, &[], "Big spiked ball", Trap),
    (true, 0x6F, &[], "Spiked ball", Trap),
];

/// What comes out of a block content's block: a power-up object type (for its picture and
/// name), or coins (the item marked M, which no object type shows).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gives {
    Object(u8),
    Coins(u16),
}

/// What a block content gives when the block under it is hit from below (research
/// `blockprobe`, `cart_model::objects`), and the power-up Mario gets instead when he is big,
/// if that differs. The same in both object catalogues.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hidden {
    pub gives: Gives,
    pub big_gives: Option<u8>,
}

/// The blocks a block content works in (`cart_model::objects`): the plain block ($00, which a
/// big Mario breaks), the ? block ($01), $02, $03 and the invisible block ($7E).
pub const HIDDEN_IN: [u8; 5] = cart_model::objects::RELEASED_BY;

pub fn hidden(kind: u8) -> Option<Hidden> {
    let gives = |gives: u8| Some(Hidden { gives: Gives::Object(gives), big_gives: None });
    match kind {
        0x11 => Some(Hidden { gives: Gives::Object(0x1B), big_gives: Some(0x1C) }),
        0x12 => gives(0x1C),
        0x13 => gives(0x1D),
        0x14 => gives(0x1E),
        0x15 => gives(0x0F),
        0x19 => Some(Hidden { gives: Gives::Coins(50), big_gives: None }),
        _ => None,
    }
}

/// The first level of the second object catalogue.
const SECOND: usize = 10;

/// The editor's name and category for object type `kind` in game level `level` (0-31).
pub fn name(level: usize, kind: u8) -> Option<(&'static str, Category)> {
    let second = level >= SECOND;
    NAMES.iter().find(|e| e.0 == second && e.1 == kind && (e.2.is_empty() || e.2.contains(&level))).map(|e| (e.3, e.4))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_by_catalogue_and_level() {
        assert_eq!(name(7, 0x1B), Some(("Mushroom", PowerUp)));
        assert_eq!(name(11, 0x1B), Some(("Mushroom", PowerUp)));
        assert_eq!(name(19, 0x1B), Some(("Boss room object", Scripted)), "a level's own meaning first");
        assert_eq!(name(6, 0x4F), Some(("Spike ball", Trap)));
        assert_eq!(name(9, 0x4F), Some(("Spark", Enemy)));
        assert_eq!(name(3, 0x7F), None);
        assert_eq!(name(0, 0x11), Some(("Hidden mushroom", BlockContent)), "level 00's $11 is a block content");
        assert_eq!(hidden(0x11), Some(Hidden { gives: Gives::Object(0x1B), big_gives: Some(0x1C) }));
        assert_eq!(hidden(0x15).map(|h| h.gives), Some(Gives::Object(0x0F)));
        assert_eq!(name(5, 0x0F), Some(("Heart (extra life)", PowerUp)));
        assert_eq!((name(2, 0x19), hidden(0x19).map(|h| h.gives)), (Some(("Hidden 50 coins", BlockContent)), Some(Gives::Coins(50))));
        assert_eq!(name(4, 0x19), Some(("Invisible marker", Marker)), "a block content in level 02 only");
    }
}
