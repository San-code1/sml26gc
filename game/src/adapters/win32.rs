//! Keeping the check window out of the way: it goes behind the others and focus returns to
//! whatever had it. Does nothing outside Windows.

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicIsize, Ordering};
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    static PREV: AtomicIsize = AtomicIsize::new(0);

    pub fn remember_foreground() {
        unsafe { PREV.store(GetForegroundWindow() as isize, Ordering::SeqCst) }
    }

    pub fn send_to_back(title: &str) {
        let w: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
        unsafe {
            let hwnd = FindWindowW(std::ptr::null(), w.as_ptr());
            if !hwnd.is_null() {
                SetWindowPos(hwnd, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
            let prev = PREV.load(Ordering::SeqCst);
            if prev != 0 {
                SetForegroundWindow(prev as _);
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn remember_foreground() {}
    pub fn send_to_back(_: &str) {}
}

pub use imp::{remember_foreground, send_to_back};
