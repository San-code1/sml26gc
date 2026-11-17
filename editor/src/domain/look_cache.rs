//! Object pictures captured from the game, kept as text so the next start has them at once:
//! a first line `NMB looks 5`, then one line per (level, object type): `level kind -` when the
//! object draws nothing, else `level kind w h dx dy sets pixels` (`sets`: the sprite tile sets
//! it uses, `level_graphics`) with one character per pixel (`.` see-through, `0`-`3` a shade);
//! and `t level kind tiles` (hex, no spaces) for the sprite tiles an object shows over its
//! animation (`art::object_tiles`). An older cache is not read, so it is captured again.
use cart_model::art::Look;

const HEAD: &str = "NMB looks 5";

/// A captured picture: the level it was captured in, the object type, the picture.
pub type Entry = (usize, u8, Option<Look>);
/// The sprite tiles an object type shows in a level over its animation.
pub type Tiles = (usize, u8, Vec<u8>);

pub fn to_text(entries: &[Entry], tiles: &[Tiles]) -> String {
    let mut out = format!("{HEAD}\n");
    for (l, k, t) in tiles {
        out += &format!("t {l} {k} {}\n", t.iter().map(|b| format!("{b:02x}")).collect::<String>());
    }
    for (l, k, look) in entries {
        match look {
            None => out += &format!("{l} {k} -\n"),
            Some(lk) => {
                let px: String = lk.px.iter().map(|p| p.map_or('.', |s| (b'0' + s) as char)).collect();
                out += &format!("{l} {k} {} {} {} {} {} {px}\n", lk.w, lk.h, lk.dx, lk.dy, lk.sets);
            }
        }
    }
    out
}

/// The pictures and tiles of a cache text; a text of another version gives none, a damaged
/// line is skipped.
pub fn from_text(text: &str) -> (Vec<Entry>, Vec<Tiles>) {
    let mut lines = text.lines();
    if lines.next() != Some(HEAD) {
        return (Vec::new(), Vec::new());
    }
    let (tiles, pictures): (Vec<&str>, Vec<&str>) = lines.partition(|l| l.starts_with("t "));
    (pictures.into_iter().filter_map(entry).collect(), tiles.into_iter().filter_map(tiles_entry).collect())
}

fn tiles_entry(line: &str) -> Option<Tiles> {
    let f: Vec<&str> = line.split(' ').collect();
    let (l, k) = (f.get(1)?.parse().ok()?, f.get(2)?.parse().ok()?);
    let hex = f.get(3).copied().unwrap_or("");
    let t = (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()).collect::<Option<Vec<u8>>>()?;
    Some((l, k, t))
}

fn entry(line: &str) -> Option<Entry> {
    let f: Vec<&str> = line.split(' ').collect();
    let (l, k) = (f.first()?.parse().ok()?, f.get(1)?.parse().ok()?);
    if f.get(2) == Some(&"-") {
        return Some((l, k, None));
    }
    let (w, h): (usize, usize) = (f.get(2)?.parse().ok()?, f.get(3)?.parse().ok()?);
    let (dx, dy) = (f.get(4)?.parse().ok()?, f.get(5)?.parse().ok()?);
    let sets = f.get(6)?.parse().ok()?;
    let px: Vec<Option<u8>> = f.get(7)?.bytes().map(|b| (b != b'.').then(|| b - b'0')).collect();
    (px.len() == w * h && px.iter().all(|p| p.is_none_or(|s| s < 4))).then_some((l, k, Some(Look { w, h, px, dx, dy, sets })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pictures_go_to_text_and_back() {
        let look = Look { w: 2, h: 2, px: vec![None, Some(1), Some(3), Some(0)], dx: -4, dy: 8, sets: 5 };
        let entries = vec![(3, 0x1F, Some(look)), (3, 0x20, None)];
        let tiles = vec![(3, 0x1F, vec![0xB0, 0xB1, 0x0A]), (4, 0x11, vec![])];
        let text = to_text(&entries, &tiles);
        assert_eq!(text, "NMB looks 5\nt 3 31 b0b10a\nt 4 17 \n3 31 2 2 -4 8 5 .130\n3 32 -\n");
        assert_eq!(from_text(&text), (entries, tiles));
        assert_eq!(from_text("NMB looks 3\n3 32 -\n"), (vec![], vec![]), "an older cache is captured again");
        assert_eq!(from_text("NMB looks 5\n3 31 2 2 0 0 0 .1\n3 32 -\nt 1 2 zz\n"), (vec![(3, 0x20, None)], vec![]));
    }
}
