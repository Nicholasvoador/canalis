//! Windows process plumbing. The binary is a GUI-subsystem app (no console window for the
//! window or the background service), so command-line output, the service log and the
//! one-service-at-a-time rule need a little help.

use std::fs::OpenOptions;
use std::os::windows::io::{IntoRawHandle, RawHandle};
use std::path::Path;
use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole, FreeConsole, GetStdHandle, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle};
use windows_sys::Win32::System::Threading::CreateMutexW;

const MAX_LOG_BYTES: u64 = 1024 * 1024;

/// Lets command-line output reach the terminal that started us, unless it is already redirected.
pub fn attach_parent_console() {
    let has_output = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
    if !has_output.is_null() && has_output != INVALID_HANDLE_VALUE {
        return;
    }
    if unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } == 0 {
        return;
    }
    if let Ok(console) = OpenOptions::new().write(true).open("CONOUT$") {
        set_output_handles(console.into_raw_handle());
    }
}

/// Lets go of the console attached by `attach_parent_console`. Windows ends every process still
/// attached to a console when its window closes, so the service must not keep the console of
/// the terminal or installer that started it.
pub fn detach_console() {
    unsafe { FreeConsole() };
}

/// Sends stdout and stderr to `log_file`, starting it afresh once it grows past 1 MB.
pub fn redirect_output_to(log_file: &Path) -> std::io::Result<()> {
    if let Some(directory) = log_file.parent() {
        std::fs::create_dir_all(directory)?;
    }
    if std::fs::metadata(log_file).is_ok_and(|metadata| metadata.len() > MAX_LOG_BYTES) {
        std::fs::remove_file(log_file)?;
    }
    let file = OpenOptions::new().create(true).append(true).open(log_file)?;
    set_output_handles(file.into_raw_handle());
    Ok(())
}

fn set_output_handles(handle: RawHandle) {
    unsafe {
        SetStdHandle(STD_OUTPUT_HANDLE, handle);
        SetStdHandle(STD_ERROR_HANDLE, handle);
    }
}

/// Takes a named lock held until this process exits; false when another process holds it.
pub fn claim_single_instance(lock_name: &str) -> bool {
    let wide_name: Vec<u16> = lock_name.encode_utf16().chain(std::iter::once(0)).collect();
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, wide_name.as_ptr()) };
    !handle.is_null() && unsafe { GetLastError() } != ERROR_ALREADY_EXISTS
}
