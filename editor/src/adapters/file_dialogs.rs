//! The system's Save and Open dialogs for level and project files, starting in `exports\`
//! (`host::exports_dir`). Off Windows there is no dialog: every request is cancelled.
use crate::ports::{FileDialogs, FileKind};
use std::path::PathBuf;

pub struct SystemDialogs {
    pub es: bool,
    /// The editor window's title: the dialogs open over it.
    pub owner: &'static str,
}

impl SystemDialogs {
    fn names(&self, kind: FileKind) -> (&'static str, &'static str, &'static str) {
        let es = self.es;
        let pick = |en, sp| if es { sp } else { en };
        match kind {
            FileKind::Level => (pick("Level file", "Fichero de nivel"), pick("Export level", "Exportar nivel"), pick("Import level", "Importar nivel")),
            FileKind::Zone => (pick("Zone file", "Fichero de zona"), pick("Export zone", "Exportar zona"), pick("Import zone", "Importar zona")),
            FileKind::Project => (pick("Project file", "Fichero de proyecto"), pick("Export project", "Exportar proyecto"), pick("Import project", "Importar proyecto")),
        }
    }
}

impl FileDialogs for SystemDialogs {
    fn save_as(&self, kind: FileKind, name: &str) -> Option<PathBuf> {
        let (what, title, _) = self.names(kind);
        imp::dialog(self.owner, what, title, kind.extension(), Some(name))
    }

    fn open(&self, kind: FileKind) -> Option<PathBuf> {
        let (what, _, title) = self.names(kind);
        imp::dialog(self.owner, what, title, kind.extension(), None)
    }
}

#[cfg(windows)]
mod imp {
    use std::path::PathBuf;
    use windows_sys::Win32::UI::Controls::Dialogs::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// `save`: the suggested name, for a Save dialog; None for an Open dialog.
    pub fn dialog(owner: &str, what: &str, title: &str, ext: &str, save: Option<&str>) -> Option<PathBuf> {
        let dir = host::exports_dir();
        let _ = std::fs::create_dir_all(&dir);
        let owner = wide(owner);
        // pairs of (description, pattern), each ended by 0, the list by another 0
        let filter: Vec<u16> = format!("{what} (*.{ext})\0*.{ext}\0\0").encode_utf16().collect();
        let dir = wide(&dir.to_string_lossy());
        let title = wide(title);
        let ext_w = wide(ext);
        let mut file = vec![0u16; 4096];
        if let Some(name) = save {
            let name: Vec<u16> = name.encode_utf16().take(file.len() - 1).collect();
            file[..name.len()].copy_from_slice(&name);
        }
        let ok = unsafe {
            let mut ofn: OPENFILENAMEW = std::mem::zeroed();
            ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
            ofn.hwndOwner = FindWindowW(std::ptr::null(), owner.as_ptr());
            ofn.lpstrFilter = filter.as_ptr();
            ofn.nFilterIndex = 1;
            ofn.lpstrFile = file.as_mut_ptr();
            ofn.nMaxFile = file.len() as u32;
            ofn.lpstrInitialDir = dir.as_ptr();
            ofn.lpstrTitle = title.as_ptr();
            ofn.lpstrDefExt = ext_w.as_ptr();
            if save.is_some() {
                ofn.Flags = OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR | OFN_EXPLORER;
                GetSaveFileNameW(&mut ofn)
            } else {
                ofn.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR | OFN_EXPLORER;
                GetOpenFileNameW(&mut ofn)
            }
        };
        if ok == 0 {
            return None;
        }
        let n = file.iter().position(|&c| c == 0).unwrap_or(file.len());
        Some(PathBuf::from(String::from_utf16_lossy(&file[..n])))
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn dialog(_: &str, _: &str, _: &str, _: &str, _: Option<&str>) -> Option<std::path::PathBuf> {
        None
    }
}
