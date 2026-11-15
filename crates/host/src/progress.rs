//! Levels cleared on the editor-made world map: level numbers separated by spaces.
use std::collections::HashSet;
use std::path::Path;

pub fn load(path: &Path) -> HashSet<u8> {
    std::fs::read_to_string(path)
        .map(|t| parse(&t))
        .unwrap_or_default()
}

pub fn save(path: &Path, cleared: &HashSet<u8>) -> std::io::Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::write(path, format(cleared))
}

fn parse(text: &str) -> HashSet<u8> {
    text.split_whitespace()
        .filter_map(|w| w.parse().ok())
        .collect()
}

fn format(cleared: &HashSet<u8>) -> String {
    let mut v: Vec<u8> = cleared.iter().copied().collect();
    v.sort();
    v.iter()
        .map(|l| l.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorted_text_and_back() {
        let set: HashSet<u8> = [7, 0, 31, 12].into_iter().collect();
        assert_eq!(format(&set), "0 7 12 31");
        assert_eq!(parse("0 7  12\n31 x 300"), set);
    }
}
