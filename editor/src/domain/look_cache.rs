//! Object pictures captured from the game, kept as text so the next start has them at once:
//! a first line `NMB looks 3`, then one line per (level, object type): `level kind -` when the
//! object draws nothing, else `level kind w h dx dy sets pixels` (`sets`: the sprite tile sets
//! it uses, `level_graphics`) with one character per pixel (`.` see-through, `0`-`3` a shade).
//! An older cache is not read, so its pictures are captured again.
use cart_model::art::Look;

const HEAD: &str = "NMB looks 3";

/// A captured picture: the level it was captured in, the object type, the picture.
pub type Entry = (usize, u8, Option<Look>);

pub fn to_text(entries: &[Entry]) -> String {
    let mut out = format!("{HEAD}\n");
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

/// The entries of a cache text; a text of another version gives none, a damaged line is
/// skipped.
pub fn from_text(text: &str) -> Vec<Entry> {
    let mut lines = text.lines();
    if lines.next() != Some(HEAD) {
        return Vec::new();
    }
    lines.filter_map(entry).collect()
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
        let text = to_text(&entries);
        assert_eq!(text, "NMB looks 3\n3 31 2 2 -4 8 5 .130\n3 32 -\n");
        assert_eq!(from_text(&text), entries);
        assert_eq!(from_text("NMB looks 1\n3 32 -\n"), vec![], "an older cache is captured again");
        assert_eq!(from_text("NMB looks 3\n3 31 2 2 0 0 0 .1\n3 32 -\n"), vec![(3, 0x20, None)]);
    }
}
