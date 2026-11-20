//! Languages and a translation dictionary. The program's texts are written in English; a
//! catalogue gives each one in the other languages, and a text it lacks shows in English.
//!
//! A catalogue is plain text, one entry per paragraph (blank lines between), each line a
//! language id, a colon, a space and the text; `#` starts a comment line. In the texts `\n`
//! is a line break and `\\` a backslash.
//!
//! ```text
//! en: Save
//! es: Guardar
//! ja: 保存
//! ```
use std::collections::HashMap;

/// The languages, in the order the editor's and the game's lists show them (and the game's
/// settings number them): the first three as before, then the others.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Lang {
    #[default]
    En,
    Es,
    Ja,
    Pt,
    Fr,
    De,
    It,
    Id,
    Ru,
    Zh,
    Ko,
}

/// How many languages there are.
pub const LANGS: usize = 11;

impl Lang {
    pub const ALL: [Lang; LANGS] = [Lang::En, Lang::Es, Lang::Ja, Lang::Pt, Lang::Fr, Lang::De, Lang::It, Lang::Id, Lang::Ru, Lang::Zh, Lang::Ko];

    /// The id in settings files and catalogues (ISO 639-1; `zh` is Simplified Chinese).
    pub fn id(self) -> &'static str {
        ["en", "es", "ja", "pt", "fr", "de", "it", "id", "ru", "zh", "ko"][self as usize]
    }

    pub fn from_id(id: &str) -> Option<Lang> {
        Lang::ALL.into_iter().find(|l| l.id() == id.trim())
    }

    /// The language's name written in itself.
    pub fn name(self) -> &'static str {
        ["English", "Español", "日本語", "Português", "Français", "Deutsch", "Italiano", "Bahasa Indonesia", "Русский", "简体中文", "한국어"][self as usize]
    }

    /// The language of a locale tag ("es-MX", "pt_BR", "zh-Hans"): its primary subtag, or
    /// English when it is none of them.
    pub fn from_tag(tag: &str) -> Lang {
        let primary = tag.split(['-', '_', '.']).next().unwrap_or("").to_ascii_lowercase();
        // "in" is Indonesian's old code, still used by some systems
        let primary = if primary == "in" { "id".to_string() } else { primary };
        Lang::from_id(&primary).unwrap_or(Lang::En)
    }
}

/// The texts of a catalogue, looked up by their English: each entry keeps a text per other
/// language (`Lang` order, English left out).
#[derive(Default, Debug)]
pub struct Dictionary {
    entries: HashMap<String, Vec<Option<String>>>,
}

impl Dictionary {
    /// Read a catalogue; an error names the line that is not an entry's.
    pub fn parse(text: &str) -> Result<Dictionary, String> {
        let mut d = Dictionary::default();
        let mut entry: Vec<Option<String>> = vec![None; LANGS];
        let mut close = |entry: &mut Vec<Option<String>>, line: usize| -> Result<(), String> {
            if entry.iter().all(Option::is_none) {
                return Ok(());
            }
            let mut texts = std::mem::replace(entry, vec![None; LANGS]);
            let en = texts.remove(0).ok_or(format!("line {line}: an entry without its English"))?;
            if d.entries.insert(en.clone(), texts).is_some() {
                return Err(format!("line {line}: \"{en}\" twice"));
            }
            Ok(())
        };
        for (i, line) in text.lines().enumerate() {
            let line = line.strip_suffix('\r').unwrap_or(line);
            if line.trim().is_empty() {
                close(&mut entry, i + 1)?;
                continue;
            }
            if line.starts_with('#') {
                continue;
            }
            let (id, value) = line.split_once(": ").ok_or(format!("line {}: not \"id: text\"", i + 1))?;
            let lang = Lang::from_id(id).ok_or(format!("line {}: unknown language \"{id}\"", i + 1))?;
            let slot = &mut entry[lang as usize];
            if slot.is_some() {
                return Err(format!("line {}: {id} twice in one entry", i + 1));
            }
            *slot = Some(unescape(value));
        }
        close(&mut entry, text.lines().count())?;
        Ok(d)
    }

    /// `en` in `lang`, or `en` itself when the catalogue has no such text.
    pub fn get<'a>(&'a self, lang: Lang, en: &'a str) -> &'a str {
        if lang == Lang::En {
            return en;
        }
        self.entries.get(en).and_then(|t| t[lang as usize - 1].as_deref()).unwrap_or(en)
    }

    /// Every entry: its English, and its text in `lang` if it has one (English: itself).
    pub fn entries(&self, lang: Lang) -> impl Iterator<Item = (&str, Option<&str>)> {
        self.entries.iter().map(move |(en, t)| (en.as_str(), if lang == Lang::En { Some(en.as_str()) } else { t[lang as usize - 1].as_deref() }))
    }

    /// The languages entry `en` has no text in (empty when it has all of them).
    pub fn missing(&self, en: &str) -> Vec<Lang> {
        let Some(t) = self.entries.get(en) else { return Lang::ALL[1..].to_vec() };
        Lang::ALL[1..].iter().copied().filter(|&l| t[l as usize - 1].is_none()).collect()
    }
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const CATALOGUE: &str = "# a comment\nen: Save\nes: Guardar\nja: 保存\n\nen: Two\\nlines\nes: Dos\\nlineas\n\n";

    #[test]
    fn a_catalogue_is_read_and_looked_up_by_its_english() {
        let d = Dictionary::parse(CATALOGUE).unwrap();
        assert_eq!(d.get(Lang::Es, "Save"), "Guardar");
        assert_eq!(d.get(Lang::Ja, "Save"), "保存");
        assert_eq!(d.get(Lang::En, "Save"), "Save");
        assert_eq!(d.get(Lang::Es, "Two\nlines"), "Dos\nlineas");
    }

    #[test]
    fn eleven_languages_by_id_name_and_tag() {
        assert_eq!(Lang::ALL.len(), LANGS);
        assert!(Lang::ALL.iter().enumerate().all(|(i, &l)| l as usize == i && Lang::from_id(l.id()) == Some(l)));
        assert_eq!((Lang::from_tag("pt_BR.UTF-8"), Lang::from_tag("zh-Hans-CN"), Lang::from_tag("in_ID"), Lang::from_tag("ar-EG")), (Lang::Pt, Lang::Zh, Lang::Id, Lang::En));
        assert_eq!((Lang::Ko.name(), Lang::Ru.id()), ("한국어", "ru"));
        let d = Dictionary::parse("en: Save\nde: Speichern\nko: 저장\n").unwrap();
        assert_eq!((d.get(Lang::De, "Save"), d.get(Lang::Ko, "Save"), d.get(Lang::Fr, "Save")), ("Speichern", "저장", "Save"));
        assert_eq!(d.missing("Save").len(), LANGS - 3);
    }

    #[test]
    fn a_missing_text_shows_in_english() {
        let d = Dictionary::parse(CATALOGUE).unwrap();
        assert_eq!(d.get(Lang::Ja, "Two\nlines"), "Two\nlines", "no Japanese for it");
        assert_eq!(d.get(Lang::Es, "Unknown"), "Unknown");
    }

    #[test]
    fn bad_catalogues_are_refused() {
        assert!(Dictionary::parse("es: Guardar\n").is_err(), "no English");
        assert!(Dictionary::parse("en: A\n\nen: A\n").is_err(), "twice");
        assert!(Dictionary::parse("en: A\nar: B\n").is_err(), "unknown language");
        assert!(Dictionary::parse("en A\n").is_err(), "no colon");
    }

    #[test]
    fn locale_tags_give_the_language() {
        assert_eq!(Lang::from_tag("es-MX"), Lang::Es);
        assert_eq!(Lang::from_tag("ja-JP"), Lang::Ja);
        assert_eq!(Lang::from_tag("fr-FR"), Lang::Fr);
        assert_eq!(Lang::from_tag("hi-IN"), Lang::En);
        assert_eq!(Lang::from_tag("ES_es"), Lang::Es);
        assert_eq!(Lang::from_id("ja"), Some(Lang::Ja));
    }
}
