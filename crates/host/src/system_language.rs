//! The language the system shows its own texts in (Windows: the user's display language).
use i18n::Lang;

/// The system's language when it is one of `Lang::ALL`, English otherwise.
pub fn system_language() -> Lang {
    imp::system_language()
}

#[cfg(windows)]
mod imp {
    use i18n::Lang;
    use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;

    /// Primary language ids (the low 10 bits of a LANGID) and their languages; any Chinese
    /// gives Simplified Chinese.
    const LANGS: [(u16, Lang); 10] = [(0x0A, Lang::Es), (0x11, Lang::Ja), (0x16, Lang::Pt), (0x0C, Lang::Fr), (0x07, Lang::De), (0x10, Lang::It), (0x21, Lang::Id), (0x19, Lang::Ru), (0x04, Lang::Zh), (0x12, Lang::Ko)];

    pub fn system_language() -> Lang {
        // SAFETY: no arguments, no pointers; it only reads the user's settings.
        let id = unsafe { GetUserDefaultUILanguage() };
        LANGS.iter().find(|l| l.0 == id & 0x3FF).map_or(Lang::En, |l| l.1)
    }
}

#[cfg(not(windows))]
mod imp {
    use i18n::Lang;

    pub fn system_language() -> Lang {
        std::env::var("LANG").map(|t| Lang::from_tag(&t)).unwrap_or(Lang::En)
    }
}
