use super::*;
use std::time::Instant;

fn r() -> Renderer {
    Renderer::new().expect("fonts load")
}

#[test]
fn renders_every_sample_key() {
    let r = r();
    for face in samples::keys() {
        for p in [0.0, 0.3, 1.0] {
            for t in [0.0, 1.7, 1000.0] {
                let img = r.key(&face, t, p);
                assert_eq!(img.dimensions(), (KEY_SIZE, KEY_SIZE));
            }
        }
    }
}

#[test]
fn renders_odd_faces() {
    let r = r();
    let faces = [
        KeyFace::default(),
        KeyFace { label: "ONLY A LABEL".into(), ..Default::default() },
        KeyFace { icon: Some(glyphs::GLOBE), ..Default::default() },
        KeyFace { ring: Some(1.0), minor: Some("DONE".into()), ..Default::default() },
        KeyFace { ring: Some(f32::NAN), icon: Some('x'), orb: Some(Orb::Attention), ..Default::default() },
        KeyFace { orb: Some(Orb::Ready), minor: Some("A VERY LONG MINOR LINE THAT SCROLLS".into()), ..Default::default() },
    ];
    for f in &faces {
        for p in [0.0, 0.5, 1.0, -1.0, 2.0, f32::NAN] {
            assert_eq!(r.key(f, 3.0, p).dimensions(), (KEY_SIZE, KEY_SIZE));
        }
    }
}

#[test]
fn ground_is_plum_deep_when_idle() {
    let img = r().key(&KeyFace::default(), 0.0, 0.0);
    assert!(img.pixels().all(|p| p.0 == [0x12, 0x09, 0x1f]));
}

#[test]
fn press_lifts_ground() {
    let r = r();
    let f = KeyFace::default();
    let idle = r.key(&f, 0.0, 0.0).get_pixel(70, 70).0;
    let pressed = r.key(&f, 0.0, 1.0).get_pixel(70, 70).0;
    assert!(pressed[2] > idle[2] + 10, "{idle:?} {pressed:?}");
}

#[test]
fn dial_is_round_and_sized() {
    let r = r();
    let img = r.dial(&samples::dial(), 0.5);
    assert_eq!(img.dimensions(), (DIAL_SIZE, DIAL_SIZE));
    assert_eq!(img.get_pixel(0, 0).0, [0, 0, 0]);
    assert_eq!(img.get_pixel(239, 239).0, [0, 0, 0]);
    assert_eq!(img.get_pixel(120, 45).0, [0x23, 0x1b, 0x2c]);
    let empty = r.dial(&DialFace::default(), 0.0);
    assert_eq!(empty.dimensions(), (DIAL_SIZE, DIAL_SIZE));
}

#[test]
fn key_moves_logic() {
    let r = r();
    let base = KeyFace { label: "CHROME".into(), icon: Some(glyphs::GLOBE), ..Default::default() };
    assert!(!r.key_moves(&base));
    assert!(r.key_moves(&KeyFace { orb: Some(Orb::Ready), ..base.clone() }));
    let long = KeyFace { label: "GROOVER FIX · EDIT RENDER.RS".into(), ..base.clone() };
    assert!(r.key_moves(&long));
    // "SAVE CLIP" fits the full width but would it fit next to an orb: either way an orb moves.
    let clip = KeyFace { label: "SAVE CLIP".into(), ..base.clone() };
    assert!(!r.key_moves(&clip));
    assert!(!r.key_moves(&KeyFace { ring: Some(0.4), minor: Some("62% · SHAPE".into()), ..base.clone() }));
    assert!(r.key_moves(&KeyFace { minor: Some("A VERY LONG MINOR LINE THAT SCROLLS".into()), ..base }));
}

#[test]
fn still_faces_do_not_change_over_time() {
    let r = r();
    let f = KeyFace { label: "KILN".into(), icon: Some(glyphs::OVEN), ring: Some(0.62), minor: Some("62%".into()), ..Default::default() };
    assert_eq!(r.key(&f, 0.0, 0.0), r.key(&f, 5.3, 0.0));
}

#[test]
fn scrolling_label_moves() {
    let r = r();
    let f = &samples::keys()[0];
    assert_ne!(r.key(f, 0.0, 0.0), r.key(f, 0.5, 0.0));
}

/// Faces whose default (Glare) press frames are pinned by [`GLARE_REFERENCE`].
fn glare_reference_faces() -> Vec<KeyFace> {
    let mut faces = samples::keys();
    faces.push(KeyFace {
        label: "CHROME".into(),
        logo: Some("googlechrome".into()),
        ground: Some([0x10, 0x19, 0x30]),
        accent: Some([0x6A, 0x9B, 0xD8]),
        ..Default::default()
    });
    faces.push(KeyFace { logo: Some("blender".into()), ring: Some(0.3), minor: Some("RENDER".into()), ..Default::default() });
    faces
}

const GLARE_PS: [f32; 6] = [1.0, 0.85, 0.6, 0.3, 0.05, 0.0];

fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
}

fn glare_hashes(r: &Renderer) -> Vec<u64> {
    let mut out = Vec::new();
    for f in glare_reference_faces() {
        for p in GLARE_PS {
            out.push(fnv(r.key(&f, 0.4, p).as_raw()));
        }
    }
    out
}

/// FNV-1a of each frame from [`glare_hashes`], taken from the renderer before press styles
/// existed (commit beeea6a): the approved Glare look must stay byte for byte the same.
const GLARE_REFERENCE: [u64; 42] = [
    0x76536532ea2e28c1, 0x39e91a2b071c96f5, 0x80ffce2d6a86171e, 0xfcbe2010adaf31e7, 0xeb76b68a31ea68ae,
    0xad396207ddc2acca, 0xa8382c1d5c09c7dc, 0x7e06130053e6f5a9, 0x49e4e9bc2c128774, 0x5e83384dca0b436b,
    0xf9bccfc8f8cbb44b, 0x782f2e582ba2641b, 0x54de40910b4d724e, 0x7b1857bc770616b0, 0xad8a83f22bfce40b,
    0x72d9d55a53c06914, 0x83cd76eed60b83c4, 0x8f09ce9cea08f776, 0x8cdf69f153d6bce7, 0xef4e3fc9fa6446ca,
    0xd932efc9eda05bbd, 0x38d4eee2bf53430d, 0xf9ad4b693b713d7d, 0xbddea8652970877c, 0xb256fc7fd0031c0f,
    0xef7f2151c90cbcb2, 0xa4bce2dee7c63806, 0xda7c861b4299e1ab, 0x7a087b3141655923, 0x0645fd878ee6b133,
    0x4bcab07d694d482b, 0x414fc8ba0fb18141, 0x60f03d9f4eb27db2, 0x9f8b9f0b3e5e3866, 0xb988905d7724bf22,
    0x73e46c9d46c672e8, 0xb42af4bc709c9667, 0xcd58b85f0ba48db6, 0x10592ed02abba284, 0x7564f49478609f26,
    0x18243dc6dd0ee729, 0xe62739962cf658a3,
];

#[test]
fn glare_is_unchanged() {
    assert_eq!(PressStyle::default(), PressStyle::Glare);
    assert_eq!(glare_hashes(&r()), GLARE_REFERENCE);
}

#[test]
fn idle_is_the_same_for_every_style() {
    let r = r();
    for f in glare_reference_faces() {
        let idle = r.key(&f, 0.4, 0.0);
        for &(style, _) in press_styles() {
            assert_eq!(r.key(&KeyFace { press_style: style, ..f.clone() }, 0.4, 0.0), idle, "{style:?}");
        }
    }
}

#[test]
fn every_style_renders_and_differs() {
    let r = r();
    let names: Vec<_> = press_styles().iter().map(|s| s.1).collect();
    assert_eq!(names, ["GLARE", "RIPPLE", "FLASH", "SPRING", "IMPACT", "SHUTTER"]);
    for f in glare_reference_faces().into_iter().chain([KeyFace::default()]) {
        let mut at_press = Vec::new();
        for &(style, _) in press_styles() {
            let f = KeyFace { press_style: style, ..f.clone() };
            for i in 0..=100 {
                let p = i as f32 / 100.0;
                assert_eq!(r.key(&f, 0.4, p).dimensions(), (KEY_SIZE, KEY_SIZE));
            }
            for p in [-1.0, 2.0, f32::NAN, f32::INFINITY, 1e-7] {
                assert_eq!(r.key(&f, 0.4, p).dimensions(), (KEY_SIZE, KEY_SIZE));
            }
            at_press.push(r.key(&f, 0.4, 0.8));
        }
        for i in 0..at_press.len() {
            for j in i + 1..at_press.len() {
                assert_ne!(at_press[i], at_press[j], "styles {i} and {j} look the same");
            }
        }
    }
}

/// `cargo test --release -p pulpit-render -- --ignored --nocapture bench`
#[test]
#[ignore]
fn bench_frames() {
    let r = r();
    let keys = samples::keys();
    for (name, p) in [("idle", 0.0), ("pressed", 0.6)] {
        let n = 2000;
        let start = Instant::now();
        for i in 0..n {
            let t = i as f32 / 60.0;
            std::hint::black_box(r.key(&keys[i % keys.len()], t, p));
        }
        let us = start.elapsed().as_secs_f64() * 1e6 / n as f64;
        println!("key {name}: {us:.0} µs/frame");
    }
    let chrome = &glare_reference_faces()[5];
    for &(style, name) in press_styles() {
        let f = KeyFace { press_style: style, ..chrome.clone() };
        let n = 2000;
        let start = Instant::now();
        for i in 0..n {
            // Walk through the whole press, as 40 fps frames would.
            let p = 1.0 - (i % 14) as f32 / 14.0;
            std::hint::black_box(r.key(&f, i as f32 / 40.0, p));
        }
        let us = start.elapsed().as_secs_f64() * 1e6 / n as f64;
        println!("press {name}: {us:.0} µs/frame");
    }
    let n = 300;
    let start = Instant::now();
    for i in 0..n {
        std::hint::black_box(r.dial(&samples::dial(), i as f32 / 60.0));
    }
    let us = start.elapsed().as_secs_f64() * 1e6 / n as f64;
    println!("dial: {us:.0} µs/frame");
}

