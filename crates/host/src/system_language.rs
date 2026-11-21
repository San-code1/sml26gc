//! The language the system shows its own texts in (Windows: the user's display language; other
//! systems: the locale variables).
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

    /// The locale variables in the order POSIX gives them precedence for messages.
    pub fn system_language() -> Lang {
        let var = |name| std::env::var(name).ok();
        from_locale([var("LC_ALL"), var("LC_MESSAGES"), var("LANG")])
    }

    /// The first locale set and naming a language (`C` and `POSIX` name none).
    fn from_locale(locales: [Option<String>; 3]) -> Lang {
        locales.into_iter().flatten().find(|l| !l.is_empty() && !["C", "POSIX"].contains(&l.split('.').next().unwrap_or(""))).map_or(Lang::En, |l| Lang::from_tag(&l))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn some(s: &str) -> Option<String> {
            Some(s.to_string())
        }

        #[test]
        fn locale_variables_in_order() {
            assert_eq!(from_locale([None, None, some("es_ES.UTF-8")]), Lang::Es);
            assert_eq!(from_locale([some("fr_FR.UTF-8"), some("de_DE.UTF-8"), some("es_ES.UTF-8")]), Lang::Fr);
            assert_eq!(from_locale([some(""), some("de_DE.UTF-8"), some("es_ES.UTF-8")]), Lang::De);
            assert_eq!(from_locale([some("C.UTF-8"), None, some("ja_JP.UTF-8")]), Lang::Ja);
            assert_eq!(from_locale([some("POSIX"), None, None]), Lang::En);
            assert_eq!(from_locale([None, None, None]), Lang::En);
        }
    }
}
