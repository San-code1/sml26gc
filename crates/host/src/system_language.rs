//! The language the system shows its own texts in (Windows: the user's display language).
use i18n::Lang;

/// Spanish or Japanese when the system shows its texts in them, English otherwise.
pub fn system_language() -> Lang {
    imp::system_language()
}

#[cfg(windows)]
mod imp {
    use i18n::Lang;
    use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;

    /// Primary language ids (the low 10 bits of a LANGID).
    const LANG_SPANISH: u16 = 0x0A;
    const LANG_JAPANESE: u16 = 0x11;

    pub fn system_language() -> Lang {
        // SAFETY: no arguments, no pointers; it only reads the user's settings.
        let id = unsafe { GetUserDefaultUILanguage() };
        match id & 0x3FF {
            LANG_SPANISH => Lang::Es,
            LANG_JAPANESE => Lang::Ja,
            _ => Lang::En,
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use i18n::Lang;

    pub fn system_language() -> Lang {
        std::env::var("LANG").map(|t| Lang::from_tag(&t)).unwrap_or(Lang::En)
    }
}
