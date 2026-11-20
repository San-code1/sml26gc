//! The programs have no console window of their own (Windows subsystem): started from a
//! terminal, they print to it.

/// Print to the terminal the program was started from, if any; output already redirected (a
/// pipe, a file) is kept. Does nothing outside Windows.
pub fn attach() {
    #[cfg(windows)]
    imp::attach();
}

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};
    use windows_sys::Win32::System::Console::{AttachConsole, GetStdHandle, SetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE};

    pub fn attach() {
        unsafe {
            let out = GetStdHandle(STD_OUTPUT_HANDLE);
            if !out.is_null() && out != INVALID_HANDLE_VALUE {
                return;
            }
            if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
                return;
            }
            let name: Vec<u16> = "CONOUT$".encode_utf16().chain(Some(0)).collect();
            let con = CreateFileW(name.as_ptr(), GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, std::ptr::null(), OPEN_EXISTING, 0, std::ptr::null_mut());
            if con != INVALID_HANDLE_VALUE {
                SetStdHandle(STD_OUTPUT_HANDLE, con);
                SetStdHandle(STD_ERROR_HANDLE, con);
            }
        }
    }
}
