//! The editor window's own menu bar (Win32): built on the window found by its title, whose
//! window procedure is wrapped to queue the ids of the entries chosen. Off Windows, or if the
//! window is not found, there is no bar and nothing is ever chosen.
use crate::ports::{MenuBar, MenuEntry};

pub struct WindowMenu {
    bar: Option<imp::Bar>,
}

impl WindowMenu {
    /// The menu bar of the window titled `title`.
    pub fn new(title: &str) -> WindowMenu {
        WindowMenu { bar: imp::Bar::attach(title) }
    }
}

impl MenuBar for WindowMenu {
    fn add_menu(&mut self, title: &str, entries: &[MenuEntry]) {
        if let Some(b) = &self.bar {
            b.add_menu(title, entries);
        }
    }

    fn chosen(&mut self) -> Vec<u16> {
        if self.bar.is_some() {
            imp::take_chosen()
        } else {
            Vec::new()
        }
    }

    fn enable(&mut self, id: u16, on: bool) {
        if let Some(b) = &self.bar {
            b.enable(id, on);
        }
    }

    fn clear(&mut self) {
        if let Some(b) = &self.bar {
            b.clear();
        }
        let _ = self.chosen();
    }
}

#[cfg(windows)]
mod imp {
    use crate::ports::MenuEntry;
    use std::sync::atomic::{AtomicIsize, Ordering};
    use std::sync::Mutex;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    /// The window procedure the menu's wraps, and the entries chosen not yet taken.
    static PREVIOUS: AtomicIsize = AtomicIsize::new(0);
    static CHOSEN: Mutex<Vec<u16>> = Mutex::new(Vec::new());

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// A menu entry chosen: WM_COMMAND with notification code 0 and no control.
    unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        if msg == WM_COMMAND && (w >> 16) & 0xFFFF == 0 && l == 0 {
            if let Ok(mut c) = CHOSEN.lock() {
                c.push((w & 0xFFFF) as u16);
            }
            return 0;
        }
        let previous: WNDPROC = std::mem::transmute(PREVIOUS.load(Ordering::SeqCst));
        CallWindowProcW(previous, hwnd, msg, w, l)
    }

    pub fn take_chosen() -> Vec<u16> {
        CHOSEN.lock().map(|mut c| std::mem::take(&mut *c)).unwrap_or_default()
    }

    pub struct Bar {
        hwnd: HWND,
        menu: HMENU,
    }

    impl Bar {
        pub fn attach(title: &str) -> Option<Bar> {
            unsafe {
                let hwnd = FindWindowW(std::ptr::null(), wide(title).as_ptr());
                if hwnd.is_null() || PREVIOUS.load(Ordering::SeqCst) != 0 {
                    return None;
                }
                let menu = CreateMenu();
                if menu.is_null() {
                    return None;
                }
                PREVIOUS.store(SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize), Ordering::SeqCst);
                Some(Bar { hwnd, menu })
            }
        }

        pub fn add_menu(&self, title: &str, entries: &[MenuEntry]) {
            unsafe {
                let popup = CreatePopupMenu();
                for e in entries {
                    match e {
                        MenuEntry::Item { id, label, keys } => {
                            AppendMenuW(popup, MF_STRING, *id as usize, wide(&format!("{label}\t{keys}")).as_ptr());
                        }
                        MenuEntry::Separator => {
                            AppendMenuW(popup, MF_SEPARATOR, 0, std::ptr::null());
                        }
                    }
                }
                AppendMenuW(self.menu, MF_POPUP, popup as usize, wide(title).as_ptr());
                SetMenu(self.hwnd, self.menu);
                DrawMenuBar(self.hwnd);
            }
        }

        pub fn enable(&self, id: u16, on: bool) {
            unsafe {
                EnableMenuItem(self.menu, id as u32, MF_BYCOMMAND | if on { MF_ENABLED } else { MF_GRAYED });
            }
        }

        pub fn clear(&self) {
            unsafe {
                while GetMenuItemCount(self.menu) > 0 {
                    DeleteMenu(self.menu, 0, MF_BYPOSITION);
                }
                DrawMenuBar(self.hwnd);
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use crate::ports::MenuEntry;

    pub struct Bar;

    impl Bar {
        pub fn attach(_: &str) -> Option<Bar> {
            None
        }
        pub fn add_menu(&self, _: &str, _: &[MenuEntry]) {}
        pub fn enable(&self, _: u16, _: bool) {}
        pub fn clear(&self) {}
    }

    pub fn take_chosen() -> Vec<u16> {
        Vec::new()
    }
}
