//! Polls the foreground window and reports its program's exe name when it changes.

use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use anyhow::{Result, bail};
use windows_sys::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VK_MENU};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GetForegroundWindow, GetWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible, SW_RESTORE, SetForegroundWindow, ShowWindow,
};

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

/// Focus the window of `pid`, or of its nearest ancestor that has one (Windows Terminal, the
/// Claude app). When that process has several windows, prefer the one whose title contains
/// `title`, else the frontmost.
pub fn raise(pid: u32, title: Option<&str>) -> Result<()> {
    let procs = crate::claude::process_table();
    let windows = top_level_windows();
    let title = title.map(str::to_lowercase).filter(|t| !t.is_empty());
    let mut p = pid;
    for _ in 0..16 {
        let own: Vec<&(HWND, u32, String)> = windows.iter().filter(|(_, wp, _)| *wp == p).collect();
        let pick = title.as_ref().and_then(|t| own.iter().find(|(_, _, wt)| wt.to_lowercase().contains(t))).or(own.first());
        if let Some((hwnd, _, _)) = pick {
            bring_to_front(*hwnd);
            return Ok(());
        }
        p = match procs.get(&p) {
            Some((parent, _)) if *parent != 0 => *parent,
            _ => break,
        };
    }
    bail!("no window found for process {pid} or its parents")
}

/// Visible, unowned, titled top-level windows with their process ids and titles, front to back.
fn top_level_windows() -> Vec<(HWND, u32, String)> {
    unsafe extern "system" fn collect(hwnd: HWND, out: LPARAM) -> i32 {
        // SAFETY: `out` is the Vec passed to EnumWindows below, alive for the whole call; the
        // title buffer outlives the call that fills it.
        unsafe {
            let len = GetWindowTextLengthW(hwnd);
            if IsWindowVisible(hwnd) != 0 && GetWindow(hwnd, GW_OWNER).is_null() && len > 0 {
                let mut pid = 0u32;
                GetWindowThreadProcessId(hwnd, &mut pid);
                let mut buf = vec![0u16; len as usize + 1];
                let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
                let title = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
                (*(out as *mut Vec<(HWND, u32, String)>)).push((hwnd, pid, title));
            }
        }
        1
    }
    let mut out: Vec<(HWND, u32, String)> = Vec::new();
    // SAFETY: the callback only touches `out` through the pointer, during this call.
    unsafe { EnumWindows(Some(collect), &mut out as *mut _ as LPARAM) };
    out
}

fn bring_to_front(hwnd: HWND) {
    let alt = |up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: VK_MENU, wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { 0 }, time: 0, dwExtraInfo: 0 },
        },
    };
    let inputs = [alt(false), alt(true)];
    // SAFETY: plain Win32 calls on a window handle from EnumWindows; `inputs` is a valid array.
    unsafe {
        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        }
        // Windows lets only the process with the last input take the foreground; a tap of Alt
        // counts as that input.
        SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
        if SetForegroundWindow(hwnd) == 0 {
            log::warn!("SetForegroundWindow refused");
        }
    }
}
