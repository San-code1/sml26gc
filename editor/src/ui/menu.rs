//! The window's File menu: its entries come from the shortcut table and run as actions;
//! entries that can't run where the editor is are greyed out.
use super::shortcuts::{keys, Action};
use super::widgets::tr;
use super::Editor;
use crate::ports::MenuEntry;
use cart_model::levels::LEVELS;

/// The File menu, top to bottom (None: a separator). An entry's id is its place here + 1.
const FILE_MENU: [Option<Action>; 8] =
    [Some(Action::Save), None, Some(Action::ExportLevel), Some(Action::ImportLevel), Some(Action::ReplaceLevel), None, Some(Action::ExportProject), Some(Action::ImportProject)];

fn label(es: bool, a: Action) -> &'static str {
    match a {
        Action::Save => tr(es, "Save", "Guardar"),
        Action::ExportLevel => tr(es, "Export level...", "Exportar nivel..."),
        Action::ImportLevel => tr(es, "Import level as a new level...", "Importar nivel como nivel nuevo..."),
        Action::ReplaceLevel => tr(es, "Replace this level from a file...", "Reemplazar este nivel con un fichero..."),
        Action::ExportProject => tr(es, "Export project...", "Exportar proyecto..."),
        Action::ImportProject => tr(es, "Import project...", "Importar proyecto..."),
        _ => "",
    }
}

impl Editor {
    pub(super) fn build_menu(&mut self) {
        let entries: Vec<MenuEntry> = FILE_MENU
            .iter()
            .enumerate()
            .map(|(i, a)| match a {
                Some(a) => MenuEntry::Item { id: i as u16 + 1, label: label(self.es, *a), keys: keys(*a) },
                None => MenuEntry::Separator,
            })
            .collect();
        self.menu.add_menu(tr(self.es, "&File", "&Archivo"), &entries);
    }

    /// Run the entries chosen since the last frame and grey out those that can't run now.
    pub(super) fn update_menu(&mut self) {
        for id in self.menu.chosen() {
            if let Some(Some(a)) = FILE_MENU.get((id as usize).wrapping_sub(1)) {
                self.run(*a);
            }
        }
        for (i, a) in FILE_MENU.iter().enumerate() {
            let Some(a) = *a else { continue };
            let on = match a {
                Action::ExportLevel | Action::ImportLevel => !self.in_world,
                Action::ReplaceLevel => !self.in_world && self.cur >= LEVELS,
                _ => true,
            };
            if self.menu_enabled[i] != Some(on) {
                self.menu_enabled[i] = Some(on);
                self.menu.enable(i as u16 + 1, on);
            }
        }
    }
}

/// How many entries the File menu has, separators included.
pub(super) const FILE_MENU_LEN: usize = FILE_MENU.len();
