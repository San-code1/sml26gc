//! The window's Language menu, on the project screen and in the editor: Automatic (the
//! system's language) or one language, the one in use ticked.
use super::widgets::tr;
use crate::ports::{MenuBar, MenuEntry};
use i18n::Lang;

/// Automatic's id; the languages follow in `Lang::ALL` order.
const AUTOMATIC_ID: u16 = 201;

/// The language a choice means: the chosen one, or the system's.
pub fn resolve(choice: Option<Lang>) -> Lang {
    choice.unwrap_or_else(host::system_language::system_language)
}

/// Add the menu at the right of the bar, `choice` ticked (None: Automatic).
pub fn add(menu: &mut dyn MenuBar, lang: Lang, choice: Option<Lang>) {
    let mut entries = vec![MenuEntry::Item { id: AUTOMATIC_ID, label: tr(lang, "Automatic (system)"), keys: "" }, MenuEntry::Separator];
    entries.extend(Lang::ALL.iter().enumerate().map(|(i, l)| MenuEntry::Item { id: AUTOMATIC_ID + 1 + i as u16, label: l.name(), keys: "" }));
    menu.add_menu(tr(lang, "&Language"), &entries);
    menu.check(AUTOMATIC_ID, choice.is_none());
    for (i, l) in Lang::ALL.iter().enumerate() {
        menu.check(AUTOMATIC_ID + 1 + i as u16, choice == Some(*l));
    }
}

/// The choice entry `id` makes, if it is one of this menu's: Some(None) is Automatic.
pub fn chosen(id: u16) -> Option<Option<Lang>> {
    match id.checked_sub(AUTOMATIC_ID)? {
        0 => Some(None),
        n => Lang::ALL.get(n as usize - 1).map(|l| Some(*l)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entries_give_their_choice() {
        assert_eq!(chosen(AUTOMATIC_ID), Some(None));
        assert_eq!(chosen(AUTOMATIC_ID + 3), Some(Some(Lang::Ja)));
        assert_eq!(chosen(AUTOMATIC_ID + 4), None);
        assert_eq!(chosen(1), None);
    }
}
