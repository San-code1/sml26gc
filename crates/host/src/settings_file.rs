//! The settings file (`settings.cfg` next to the programs): `key=value` lines. What the keys
//! mean belongs to the game; this only reads and writes the file.
use crate::paths::settings_path;

/// The file's text, if there is one.
pub fn read() -> Option<String> {
    std::fs::read_to_string(settings_path()).ok()
}

pub fn write(text: &str) -> std::io::Result<()> {
    std::fs::write(settings_path(), text)
}

/// The value of `key` in `key=value` text (spaces around both are ignored).
pub fn value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().filter_map(|l| l.split_once('=')).find(|(k, _)| k.trim() == key).map(|(_, v)| v.trim())
}

/// The language chosen in the game ("en", "es" or "ja"), if any.
pub fn language() -> Option<String> {
    read().and_then(|t| value(&t, "language").map(str::to_string))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values() {
        let t = "key0=Up\npalette=1\n language = es \n";
        assert_eq!(value(t, "language"), Some("es"));
        assert_eq!(value(t, "palette"), Some("1"));
        assert_eq!(value(t, "volume"), None);
    }
}
