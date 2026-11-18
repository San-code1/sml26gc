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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Lang {
    #[default]
    En,
    Es,
    Ja,
}

impl Lang {
    pub const ALL: [Lang; 3] = [Lang::En, Lang::Es, Lang::Ja];

    /// The id in settings files and catalogues.
    pub fn id(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Es => "es",
            Lang::Ja => "ja",
        }
    }

    pub fn from_id(id: &str) -> Option<Lang> {
        Lang::ALL.into_iter().find(|l| l.id() == id.trim())
    }

    /// The language's name written in itself.
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Es => "Español",
            Lang::Ja => "日本語",
        }
    }

    /// The language of a locale tag ("es-MX", "ja_JP"): Spanish or Japanese, else English.
    pub fn from_tag(tag: &str) -> Lang {
        let primary = tag.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase();
        match primary.as_str() {
            "es" => Lang::Es,
            "ja" => Lang::Ja,
            _ => Lang::En,
        }
    }
}

/// The texts of a catalogue, looked up by their English.
#[derive(Default, Debug)]
pub struct Dictionary {
    entries: HashMap<String, [Option<String>; 2]>,
}

impl Dictionary {
    /// Read a catalogue; an error names the line that is not an entry's.
    pub fn parse(text: &str) -> Result<Dictionary, String> {
        let mut d = Dictionary::default();
        let mut entry: [Option<String>; 3] = [None, None, None];
        let mut close = |entry: &mut [Option<String>; 3], line: usize| -> Result<(), String> {
            if entry.iter().all(Option::is_none) {
                return Ok(());
            }
            let [en, es, ja] = std::mem::take(entry);
            let en = en.ok_or(format!("line {line}: an entry without its English"))?;
            if d.entries.insert(en.clone(), [es, ja]).is_some() {
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
        let slot = match lang {
            Lang::En => return en,
            Lang::Es => 0,
            Lang::Ja => 1,
        };
        self.entries.get(en).and_then(|t| t[slot].as_deref()).unwrap_or(en)
    }

    /// Every entry: its English, and whether it has each other language.
    pub fn entries(&self) -> impl Iterator<Item = (&str, Option<&str>, Option<&str>)> {
        self.entries.iter().map(|(en, [es, ja])| (en.as_str(), es.as_deref(), ja.as_deref()))
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
    fn a_missing_text_shows_in_english() {
        let d = Dictionary::parse(CATALOGUE).unwrap();
        assert_eq!(d.get(Lang::Ja, "Two\nlines"), "Two\nlines", "no Japanese for it");
        assert_eq!(d.get(Lang::Es, "Unknown"), "Unknown");
    }

    #[test]
    fn bad_catalogues_are_refused() {
        assert!(Dictionary::parse("es: Guardar\n").is_err(), "no English");
        assert!(Dictionary::parse("en: A\n\nen: A\n").is_err(), "twice");
        assert!(Dictionary::parse("en: A\nfr: B\n").is_err(), "unknown language");
        assert!(Dictionary::parse("en A\n").is_err(), "no colon");
    }

    #[test]
    fn locale_tags_give_the_language() {
        assert_eq!(Lang::from_tag("es-MX"), Lang::Es);
        assert_eq!(Lang::from_tag("ja-JP"), Lang::Ja);
        assert_eq!(Lang::from_tag("fr-FR"), Lang::En);
        assert_eq!(Lang::from_tag("ES_es"), Lang::Es);
        assert_eq!(Lang::from_id("ja"), Some(Lang::Ja));
    }
}
