//! Polls the foreground window and reports its program's exe name when it changes.

use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

use crate::controller::Msg;

pub fn spawn(controller: Sender<Msg>) {
    thread::Builder::new()
        .name("focus".into())
        .spawn(move || {
            let mut last: Option<String> = None;
            loop {
                let current = foreground_exe();
                if current.is_some() && current != last {
                    if controller.send(Msg::ActiveApp(current.clone().unwrap_or_default())).is_err() {
                        return;
                    }
                    last = current;
                }
                thread::sleep(Duration::from_millis(250));
            }
        })
        .expect("spawn focus thread");
    log::info!("following the focused application");
}

/// File name of the program owning the foreground window, e.g. `obs64.exe`.
fn foreground_exe() -> Option<String> {
    // SAFETY: plain Win32 calls; the process handle is closed before returning and the buffer
    // outlives the call that fills it.
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(process, 0, buf.as_mut_ptr(), &mut len);
        CloseHandle(process);
        if ok == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        path.rsplit('\\').next().map(str::to_owned)
    }
}
