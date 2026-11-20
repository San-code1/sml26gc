//! The shared adapters' messages in every language: `i18n/host.txt`, read once.
use i18n::{Dictionary, Lang};
use std::sync::OnceLock;

/// The catalogue's text (the editor's text check reads it too).
pub const CATALOGUE_TEXT: &str = include_str!("../i18n/host.txt");

static CATALOGUE: OnceLock<Dictionary> = OnceLock::new();

/// `en` in `lang` (English when the catalogue lacks it).
pub fn tr(lang: Lang, en: &'static str) -> &'static str {
    CATALOGUE.get_or_init(|| Dictionary::parse(CATALOGUE_TEXT).expect("the host catalogue")).get(lang, en)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_message_has_every_language() {
        let d = Dictionary::parse(CATALOGUE_TEXT).unwrap();
        let missing: Vec<String> = d.entries(Lang::En).filter(|(en, _)| !d.missing(en).is_empty()).map(|(en, _)| en.to_string()).collect();
        assert!(missing.is_empty(), "{missing:?}");
        assert_eq!(tr(Lang::De, "The project has no project file."), "Das Projekt hat keine Projektdatei.");
    }
}
