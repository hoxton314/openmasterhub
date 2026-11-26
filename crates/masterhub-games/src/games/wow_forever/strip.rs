//! The PULPIT add-on's strip: a row of coloured cells in the game window's top-left corner,
//! re-drawn five times a second (games/wow_forever/addon/PULPIT/PULPIT.lua, "PULPIT strip").
//! WoW Forever writes its log files only every few minutes, so the screen is the one live
//! channel. Display only: nothing goes back into the game. The user chose this (2026-11-24).
//!
//! Layout: [magenta][cyan][seq][60 symbols][checksum]; a symbol is 6 bits, 2 per colour
//! channel at levels 0/85/170/255. The 60 symbols carry 45 bytes (see [`Live`]).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const SYMBOLS: usize = 60;
const DATA: usize = 45;

/// What the strip says.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Live {
    pub level: u32,
    pub xp: u32,
    pub xp_max: u32,
    pub rested: u32,
    pub money: u64,
    pub map: u32,
    /// 0..1 on the map, `None` where the game gives no position (instances).
    pub pos: Option<(f32, f32)>,
    pub combat: bool,
    pub dead: bool,
    pub resting: bool,
    pub mounted: bool,
    pub health: u8,
    pub power: u8,
    pub target: Option<u8>,
    /// Seconds of cooldown left on bar 1 buttons 1-12, then bar 2 buttons 1-6 (`None`: empty).
    pub cooldowns: [Option<u8>; 18],
    /// ChrClasses id (2 = Paladin) and faction (0 Horde, 1 Alliance, 2 other): the character
    /// in play, so the panel can tell a character switch before the add-on's save catches up.
    pub class: u8,
    pub faction: u8,
}

/// The latest reading and when it came.
#[derive(Default)]
pub struct Reading {
    pub live: Option<Live>,
    pub at: Option<Instant>,
}

/// Reads the strip on its own thread while `want` says someone looks.
pub fn start(want: Arc<std::sync::atomic::AtomicBool>) -> Arc<Mutex<Reading>> {
    let out = Arc::new(Mutex::new(Reading::default()));
    let o = out.clone();
    std::thread::Builder::new()
        .name("wow-strip".into())
        .spawn(move || {
            let mut window: Option<(i32, i32)> = None;
            let mut window_at = Instant::now() - Duration::from_secs(60);
            loop {
                std::thread::sleep(Duration::from_millis(250));
                if !want.load(std::sync::atomic::Ordering::Relaxed) {
                    continue;
                }
                if window.is_none() || window_at.elapsed() > Duration::from_secs(5) {
                    window = game_window();
                    window_at = Instant::now();
                }
                let Some((x, y)) = window else { continue };
                if let Some(live) = capture(x, y).and_then(|img| decode(&img))
                    && let Ok(mut r) = o.lock() {
                        r.live = Some(live);
                        r.at = Some(Instant::now());
                    }
            }
        })
        .expect("spawn wow-strip thread");
    out
}

/// The WoW window's top-left in layout coordinates (Hyprland).
#[cfg(not(windows))]
fn game_window() -> Option<(i32, i32)> {
    let out = std::process::Command::new("hyprctl").args(["clients", "-j"]).output().ok()?;
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    v.as_array()?.iter().find(|c| c["title"] == "World of Warcraft").and_then(|c| Some((c["at"][0].as_i64()? as i32, c["at"][1].as_i64()? as i32)))
}

/// An RGB picture: width, height, pixels.
pub struct Img {
    pub w: usize,
    pub h: usize,
    pub px: Vec<[u8; 3]>,
}

/// A small screen grab of the corner (grim, PPM to stdout).
#[cfg(not(windows))]
fn capture(x: i32, y: i32) -> Option<Img> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    let mut child = Command::new("grim").args(["-t", "ppm", "-g", &format!("{x},{y} 440x14"), "-"]).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let mut buf = Vec::new();
    child.stdout.take()?.read_to_end(&mut buf).ok()?;
    let _ = child.wait();
    ppm(&buf)
}

/// The WoW window's client area top-left on the screen.
#[cfg(windows)]
fn game_window() -> Option<(i32, i32)> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
    use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;
    let title: Vec<u16> = "World of Warcraft".encode_utf16().chain([0]).collect();
    // SAFETY: a null-terminated title; the handle is only used for the call below.
    let hwnd = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    if hwnd.is_null() {
        return None;
    }
    let mut p = POINT { x: 0, y: 0 };
    // SAFETY: hwnd is a live window handle and p a valid out pointer.
    (unsafe { ClientToScreen(hwnd, &mut p) } != 0).then_some((p.x, p.y))
}

/// A small screen grab of the corner (GDI).
#[cfg(windows)]
fn capture(x: i32, y: i32) -> Option<Img> {
    use windows_sys::Win32::Graphics::Gdi::*;
    const W: i32 = 440;
    const H: i32 = 14;
    // SAFETY: plain GDI calls on handles created and released here; the buffer holds W×H BGRA pixels.
    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        let mem = CreateCompatibleDC(screen);
        let bmp = CreateCompatibleBitmap(screen, W, H);
        let old = SelectObject(mem, bmp);
        let ok = BitBlt(mem, 0, 0, W, H, screen, x, y, SRCCOPY) != 0;
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = W;
        info.bmiHeader.biHeight = -H;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB;
        let mut buf = vec![0u8; (W * H * 4) as usize];
        let lines = GetDIBits(mem, bmp, 0, H as u32, buf.as_mut_ptr().cast(), &mut info, DIB_RGB_COLORS);
        SelectObject(mem, old);
        DeleteObject(bmp);
        DeleteDC(mem);
        ReleaseDC(std::ptr::null_mut(), screen);
        (ok && lines == H).then(|| Img { w: W as usize, h: H as usize, px: buf.as_chunks::<4>().0.iter().map(|c| [c[2], c[1], c[0]]).collect() })
    }
}

#[cfg_attr(windows, allow(dead_code))]
fn ppm(b: &[u8]) -> Option<Img> {
    // "P6\n<w> <h>\n<max>\n" then RGB bytes.
    let mut fields = Vec::new();
    let mut i = 0;
    while fields.len() < 4 && i < b.len() {
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        let s = i;
        while i < b.len() && !b[i].is_ascii_whitespace() {
            i += 1;
        }
        fields.push(std::str::from_utf8(&b[s..i]).ok()?.to_owned());
    }
    i += 1;
    if fields.first()? != "P6" {
        return None;
    }
    let (w, h): (usize, usize) = (fields[1].parse().ok()?, fields[2].parse().ok()?);
    let data = b.get(i..i + w * h * 3)?;
    Some(Img { w, h, px: data.as_chunks::<3>().0.iter().map(|c| [c[0], c[1], c[2]]).collect() })
}

fn magenta(p: [u8; 3]) -> bool {
    p[0] > 190 && p[1] < 70 && p[2] > 190
}

fn cyan(p: [u8; 3]) -> bool {
    p[0] < 70 && p[1] > 190 && p[2] > 190
}

/// Finds the strip in the picture and reads it; `None` unless the markers and the checksum agree.
pub fn decode(img: &Img) -> Option<Live> {
    let at = |x: usize, y: usize| img.px[y * img.w + x];
    // The magenta start cell: its width is the cell size, its middle row the reading row.
    let (x0, y0, cw) = (0..img.h).find_map(|y| {
        let x = (0..img.w).find(|&x| magenta(at(x, y)))?;
        let run = (x..img.w).take_while(|&xx| magenta(at(xx, y))).count();
        (run >= 2).then_some((x, y, run))
    })?;
    let rows = (y0..img.h).take_while(|&y| magenta(at(x0 + cw / 2, y))).count();
    let ym = y0 + rows / 2;
    let cell = |i: usize| -> Option<[u8; 3]> {
        let x = x0 + i * cw + cw / 2;
        (x < img.w).then(|| at(x, ym))
    };
    if !cyan(cell(1)?) {
        return None;
    }
    let q = |v: u8| ((v as u32 + 42) / 85).min(3) as u8;
    let sym = |i: usize| -> Option<u8> {
        let p = cell(i)?;
        Some(q(p[0]) * 16 + q(p[1]) * 4 + q(p[2]))
    };
    let mut syms = Vec::with_capacity(SYMBOLS);
    for i in 0..SYMBOLS {
        syms.push(sym(3 + i)?);
    }
    let check = sym(3 + SYMBOLS)?;
    let sum = syms.iter().enumerate().map(|(i, &s)| s as u32 * (i as u32 + 1)).sum::<u32>() % 64;
    if sum != check as u32 {
        return None;
    }
    // 6-bit symbols back to bytes.
    let mut bytes = Vec::with_capacity(DATA);
    let (mut acc, mut nbits) = (0u32, 0u32);
    for &s in &syms {
        acc = (acc << 6) | s as u32;
        nbits += 6;
        if nbits >= 8 {
            nbits -= 8;
            bytes.push((acc >> nbits) as u8);
            acc &= (1 << nbits) - 1;
        }
    }
    if bytes.len() < DATA {
        return None;
    }
    let u = |a: usize, n: usize| bytes[a..a + n].iter().fold(0u64, |v, &b| (v << 8) | b as u64);
    let (x, y) = (u(16, 2), u(18, 2));
    let mut cooldowns = [None; 18];
    for (i, c) in cooldowns.iter_mut().enumerate() {
        let v = bytes[24 + i];
        *c = (v != 255).then_some(v);
    }
    let flags = bytes[20];
    Some(Live {
        level: bytes[0] as u32,
        xp: u(1, 3) as u32,
        xp_max: u(4, 3) as u32,
        rested: u(7, 3) as u32,
        money: u(10, 4),
        map: u(14, 2) as u32,
        pos: (x != 0 || y != 0).then(|| (x as f32 / 65535.0, y as f32 / 65535.0)),
        combat: flags & 1 != 0,
        dead: flags & 2 != 0,
        resting: flags & 4 != 0,
        mounted: flags & 8 != 0,
        health: bytes[21],
        power: bytes[22],
        target: (bytes[23] != 255).then_some(bytes[23]),
        cooldowns,
        class: bytes[42],
        faction: bytes[43],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Draws a strip the way the add-on does (same packing), with a little colour noise.
    fn draw(bytes: &[u8; DATA], cw: usize, noise: i16) -> Img {
        let mut syms = Vec::new();
        let (mut acc, mut nbits) = (0u32, 0u32);
        for &b in bytes {
            acc = (acc << 8) | b as u32;
            nbits += 8;
            while nbits >= 6 {
                nbits -= 6;
                syms.push((acc >> nbits) as u8 & 63);
                acc &= (1 << nbits) - 1;
            }
        }
        let sum = syms.iter().enumerate().map(|(i, &s)| s as u32 * (i as u32 + 1)).sum::<u32>() % 64;
        let col = |s: u8| [(s >> 4) & 3, (s >> 2) & 3, s & 3].map(|v| (v as i16 * 85 + noise).clamp(0, 255) as u8);
        let mut cells = vec![[255, 0, 255], [0, 255, 255], col(7)];
        cells.extend(syms.iter().map(|&s| col(s)));
        cells.push(col(sum as u8));
        let (w, h) = (520, 14);
        let mut px = vec![[20, 20, 30]; w * h];
        for (i, c) in cells.iter().enumerate() {
            for y in 1..1 + cw {
                for x in 3 + i * cw..3 + (i + 1) * cw {
                    if x < w {
                        px[y * w + x] = *c;
                    }
                }
            }
        }
        Img { w, h, px }
    }

    #[test]
    fn reads_what_the_add_on_writes() {
        let mut b = [0u8; DATA];
        b[0] = 9;
        b[1..4].copy_from_slice(&[0, 0x04, 0xA5]); // 1189 xp
        b[4..7].copy_from_slice(&[0, 0x11, 0x94]); // 4500
        b[10..14].copy_from_slice(&[0, 0, 0, 213]);
        b[14..16].copy_from_slice(&[0x05, 0x8C]); // 1420
        b[16..18].copy_from_slice(&[0x63, 0x0A]);
        b[18..20].copy_from_slice(&[0x8F, 0xF2]);
        b[20] = 1 | 4;
        b[21] = 87;
        b[23] = 255;
        for c in b[24..42].iter_mut() {
            *c = 255;
        }
        b[24] = 5;
        b[25] = 0;
        b[42] = 2;
        b[43] = 0;
        let img = draw(&b, 7, 9);
        let live = decode(&img).expect("decoded");
        assert_eq!((live.level, live.xp, live.xp_max, live.money, live.map), (9, 1189, 4500, 213, 1420));
        assert!(live.combat && live.resting && !live.mounted);
        assert_eq!(live.health, 87);
        assert_eq!(live.target, None);
        assert_eq!(live.cooldowns[0], Some(5));
        assert_eq!(live.cooldowns[1], Some(0));
        assert_eq!(live.cooldowns[2], None);
        assert_eq!((live.class, live.faction), (2, 0));
        let (x, y) = live.pos.unwrap();
        assert!((x - 0.3869).abs() < 0.001 && (y - 0.5623).abs() < 0.001);
    }

    #[test]
    fn rejects_a_broken_strip() {
        let b = [3u8; DATA];
        let mut img = draw(&b, 6, 0);
        // One cell wrong: the checksum catches it.
        let w = img.w;
        for y in 1..7 {
            img.px[y * w + 3 + 20 * 6 + 3] = [255, 255, 255];
        }
        assert!(decode(&img).is_none());
        assert!(decode(&Img { w: 10, h: 2, px: vec![[0, 0, 0]; 20] }).is_none());
    }
}
