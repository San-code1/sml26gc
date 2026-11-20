//! The editor's texts in its language: the catalogue `editor/i18n/editor.txt` (English, the
//! text in the code, with its translations).
use i18n::{Dictionary, Lang};
use std::sync::OnceLock;

/// `en` in `lang` (English when the catalogue has no translation for it).
pub fn tr(lang: Lang, en: &'static str) -> &'static str {
    static CATALOGUE: OnceLock<Dictionary> = OnceLock::new();
    CATALOGUE.get_or_init(|| Dictionary::parse(CATALOGUE_TEXT).expect("the editor's catalogue")).get(lang, en)
}

pub const CATALOGUE_TEXT: &str = include_str!("../i18n/editor.txt");

/// The game zones' names in English, in `ZONES` order (the catalogue has the others): the full
/// name, and the short one for lists (without the word "zone").
const ZONE_NAMES: [(&str, &str); 8] = [("Tree Zone", "Tree"), ("Pumpkin Zone", "Pumpkin"), ("Macro Zone", "Macro"), ("Space Zone", "Space"), ("Turtle Zone", "Turtle"), ("Mario Zone", "Mario"), ("Wario's Castle", "Wario's Castle"), ("Bonus game", "Bonus game")];

/// Game zone `z`'s name.
pub fn zone_name(lang: Lang, z: usize) -> &'static str {
    tr(lang, ZONE_NAMES[z].0)
}

/// Game zone `z`'s name without the word "zone", for short lists.
pub fn zone_short(lang: Lang, z: usize) -> &'static str {
    tr(lang, ZONE_NAMES[z].1)
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
            // names kept in lists and translated where they are shown: the world map's pieces,
            // Mario's powers
            found.extend(src.match_indices("piece(\"").filter_map(|(at, m)| literal(&src[at + m.len()..])));
            for line in src.lines().filter(|l| l.trim_start().starts_with("const POWERS") || l.starts_with("const ZONE_NAMES")) {
                found.extend(line.split('"').skip(1).step_by(2).map(String::from));
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

    /// The languages an entry lacks, as "no fr de".
    fn lacks(catalogue: &Dictionary, en: &str) -> Option<String> {
        let ids: Vec<&str> = catalogue.missing(en).into_iter().map(Lang::id).collect();
        (!ids.is_empty()).then(|| format!("{en:?}: no {}", ids.join(" ")))
    }

    #[test]
    fn every_text_has_every_language_and_every_entry_is_used() {
        let mut used = BTreeSet::new();
        texts_in(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut used);
        let catalogue = Dictionary::parse(CATALOGUE_TEXT).unwrap();
        let mut missing = Vec::new();
        let mut entries = BTreeSet::new();
        for (en, _) in catalogue.entries(Lang::En) {
            entries.insert(en.to_string());
            missing.extend(lacks(&catalogue, en));
        }
        // the shared adapters' messages, which the editor shows too
        let host = Dictionary::parse(host::texts::CATALOGUE_TEXT).unwrap();
        missing.extend(host.entries(Lang::En).filter_map(|(en, _)| lacks(&host, en)));
        missing.extend(used.iter().filter(|t| !entries.contains(*t)).map(|t| format!("{t:?}: not in the catalogue")));
        let unused: Vec<&String> = entries.iter().filter(|e| !used.contains(*e)).collect();
        missing.sort();
        assert!(missing.is_empty(), "{} texts lack translations:\n{}", missing.len(), missing.join("\n"));
        assert!(unused.is_empty(), "entries no source uses: {unused:?}");
    }

    /// Every language drawn with a font of its own (Japanese, Chinese, Korean, Russian): every
    /// character of its texts (catalogue, the shared messages, its own name) has a glyph in that
    /// font.
    #[test]
    fn each_language_s_font_has_every_character_of_its_texts() {
        let catalogue = Dictionary::parse(CATALOGUE_TEXT).unwrap();
        let host = Dictionary::parse(host::texts::CATALOGUE_TEXT).unwrap();
        let mut missing = Vec::new();
        for lang in Lang::ALL {
            let Some(bytes) = fonts::Face::of(lang).bytes() else { continue };
            let font = ttf_parser::Face::parse(bytes, 0).unwrap();
            let texts = catalogue.entries(lang).filter_map(|e| e.1).chain(host.entries(lang).filter_map(|e| e.1)).chain([lang.name()]);
            let mut lacking: BTreeSet<char> = BTreeSet::new();
            for t in texts {
                lacking.extend(t.chars().filter(|&c| !c.is_control() && font.glyph_index(c).is_none()));
            }
            if !lacking.is_empty() {
                missing.push(format!("{}: {lacking:?}", lang.id()));
            }
        }
        assert!(missing.is_empty(), "characters without a glyph: {missing:?}");
    }
}

#[cfg(test)]
mod zone_tests {
    use super::*;
    use cart_model::zone_maps::ZONES;

    #[test]
    fn english_and_spanish_zone_names_are_the_game_zones_own() {
        for (z, zone) in ZONES.iter().enumerate() {
            assert_eq!((zone_name(Lang::En, z), zone_name(Lang::Es, z)), (zone.name, zone.name_es));
        }
    }
}
