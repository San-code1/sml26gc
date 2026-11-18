//! The editor's texts in its language: the catalogue `editor/i18n/editor.txt` (English, the
//! text in the code, with its Spanish and Japanese) and the game zones' names.
use cart_model::zone_maps::ZONES;
use i18n::{Dictionary, Lang};
use std::sync::OnceLock;

/// `en` in `lang` (English when the catalogue has no translation for it).
pub fn tr(lang: Lang, en: &'static str) -> &'static str {
    static CATALOGUE: OnceLock<Dictionary> = OnceLock::new();
    CATALOGUE.get_or_init(|| Dictionary::parse(CATALOGUE_TEXT).expect("the editor's catalogue")).get(lang, en)
}

pub const CATALOGUE_TEXT: &str = include_str!("../i18n/editor.txt");

/// The game zones' Japanese names, in `ZONES` order.
const ZONE_NAMES_JA: [&str; 8] = ["ツリーゾーン", "パンプキンゾーン", "マクロゾーン", "スペースゾーン", "タートルゾーン", "マリオゾーン", "ワリオ城", "ボーナスゲーム"];

/// Game zone `z`'s name.
pub fn zone_name(lang: Lang, z: usize) -> &'static str {
    match lang {
        Lang::En => ZONES[z].name,
        Lang::Es => ZONES[z].name_es,
        Lang::Ja => ZONE_NAMES_JA[z],
    }
}

/// Game zone `z`'s name without the word "zone", for short lists.
pub fn zone_short(lang: Lang, z: usize) -> &'static str {
    let n = zone_name(lang, z);
    match lang {
        Lang::En => n.trim_end_matches(" Zone"),
        Lang::Es => n.trim_start_matches("Zona "),
        Lang::Ja => n.trim_end_matches("ゾーン"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;

    /// A Rust string literal's text, from just after its opening quote; None when it is not
    /// closed.
    fn literal(s: &str) -> Option<String> {
        let mut out = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '"' => return Some(out),
                '\\' => match chars.next()? {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'u' => {
                        let code: String = chars.by_ref().skip(1).take_while(|&c| c != '}').collect();
                        out.push(char::from_u32(u32::from_str_radix(&code, 16).ok()?)?);
                    }
                    '\n' | '\r' => {
                        while chars.peek().is_some_and(|c| c.is_whitespace()) {
                            chars.next();
                        }
                    }
                    other => out.push(other),
                },
                c => out.push(c),
            }
        }
        None
    }

    /// Every text the editor's sources look up: the literal after `tr(<language>, `, and the
    /// objects' names (`object_names`, after the levels they apply to).
    fn texts_in(dir: &Path, found: &mut BTreeSet<String>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                texts_in(&path, found);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let src = std::fs::read_to_string(&path).unwrap();
            if path.file_name().is_some_and(|n| n == "object_names.rs") {
                found.extend(src.match_indices("], \"").filter_map(|(at, m)| literal(&src[at + m.len()..])));
            }
            for (at, _) in src.match_indices("tr(") {
                let before = src[..at].chars().next_back();
                if before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') || src[..at].ends_with("fn ") {
                    continue;
                }
                let rest = &src[at + 3..];
                let Some(comma) = rest.find(',') else { continue };
                let after = rest[comma + 1..].trim_start();
                if let Some(text) = after.strip_prefix('"').and_then(literal) {
                    found.insert(text);
                }
            }
        }
    }

    #[test]
    fn every_text_has_its_spanish_and_japanese_and_every_entry_is_used() {
        let mut used = BTreeSet::new();
        texts_in(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut used);
        let catalogue = Dictionary::parse(CATALOGUE_TEXT).unwrap();
        let mut missing = Vec::new();
        let mut entries = BTreeSet::new();
        for (en, es, ja) in catalogue.entries() {
            entries.insert(en.to_string());
            if es.is_none() || ja.is_none() {
                missing.push(format!("{en:?}: {}{}", if es.is_none() { "no es " } else { "" }, if ja.is_none() { "no ja" } else { "" }));
            }
        }
        missing.extend(used.iter().filter(|t| !entries.contains(*t)).map(|t| format!("{t:?}: not in the catalogue")));
        let unused: Vec<&String> = entries.iter().filter(|e| !used.contains(*e)).collect();
        missing.sort();
        assert!(missing.is_empty(), "{} texts lack translations:\n{}", missing.len(), missing.join("\n"));
        assert!(unused.is_empty(), "entries no source uses: {unused:?}");
    }

    #[test]
    fn the_japanese_font_has_every_character_of_the_japanese_texts() {
        let font = ttf_parser::Face::parse(include_bytes!("../fonts/BIZUDGothic-Regular.ttf"), 0).unwrap();
        let catalogue = Dictionary::parse(CATALOGUE_TEXT).unwrap();
        let texts = catalogue.entries().filter_map(|e| e.2).chain(ZONE_NAMES_JA).chain(Lang::ALL.map(Lang::name));
        let mut missing: BTreeSet<char> = BTreeSet::new();
        for t in texts {
            missing.extend(t.chars().filter(|&c| !c.is_control() && font.glyph_index(c).is_none()));
        }
        assert!(missing.is_empty(), "characters without a glyph: {missing:?}");
    }
}
