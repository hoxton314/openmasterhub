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

/// A few faces with every kind of content, for the press-style tests.
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

