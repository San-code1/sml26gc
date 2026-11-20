//! The fonts the editor and the game's start screen draw a language with when the window
//! library's built-in font (ASCII only) cannot: the files in `fonts/` at the repository root,
//! all under the SIL Open Font License (their licences beside them).
//!
//! - BIZ UDGothic: Japanese.
//! - Noto Sans SC: Simplified Chinese.
//! - Noto Sans KR: Korean, and Russian (its Cyrillic is proportional; BIZ UDGothic's is as
//!   wide as a Japanese character).
//!
//! The Latin languages are written without accents and keep the built-in font.
use i18n::Lang;

pub const BIZ_UD_GOTHIC: &[u8] = include_bytes!("../../../fonts/BIZUDGothic-Regular.ttf");
pub const NOTO_SANS_SC: &[u8] = include_bytes!("../../../fonts/NotoSansSC-Regular.otf");
pub const NOTO_SANS_KR: &[u8] = include_bytes!("../../../fonts/NotoSansKR-Regular.otf");

/// Which font a language is drawn with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    /// The window library's own.
    BuiltIn,
    BizUdGothic,
    NotoSansSc,
    NotoSansKr,
}

impl Face {
    pub fn of(lang: Lang) -> Face {
        match lang {
            Lang::Ja => Face::BizUdGothic,
            Lang::Zh => Face::NotoSansSc,
            Lang::Ko | Lang::Ru => Face::NotoSansKr,
            Lang::En | Lang::Es | Lang::Pt | Lang::Fr | Lang::De | Lang::It | Lang::Id => Face::BuiltIn,
        }
    }

    /// The font file's bytes (None: the built-in font).
    pub fn bytes(self) -> Option<&'static [u8]> {
        match self {
            Face::BuiltIn => None,
            Face::BizUdGothic => Some(BIZ_UD_GOTHIC),
            Face::NotoSansSc => Some(NOTO_SANS_SC),
            Face::NotoSansKr => Some(NOTO_SANS_KR),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_has_a_face_and_the_files_are_fonts() {
        for l in Lang::ALL {
            if let Some(b) = Face::of(l).bytes() {
                assert!(b.starts_with(&[0, 1, 0, 0]) || b.starts_with(b"OTTO"), "{l:?}: not a font file");
            }
        }
        assert_eq!(Face::of(Lang::Fr), Face::BuiltIn);
    }
}
