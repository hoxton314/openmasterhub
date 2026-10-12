//! The hub picture: the Base and its modules where they physically sit, with live key/dial
//! images, LED colours, fader positions and input glow. Drawing happens in each module's
//! natural coordinate system (units = grid cells); one cairo matrix per module maps that to
//! the widget, so hit-testing just inverts it.

use std::f64::consts::PI;

use gtk4::cairo::{self, Context, Matrix};
use masterhub_core::config::{OneOrMany, Side};
use masterhub_core::dbus::{ModuleState, TARGET_DIAL};
use masterhub_core::layout::{BASE_PLACEMENT, GRID_COLS, GRID_ROWS, Placement};
use masterhub_proto::{ModuleKind, Rgb};

use crate::model::{Model, Selection};

/// Where a module was drawn, for hit-testing.
pub struct Hit {
    pub serial: String,
    pub kind: ModuleKind,
    /// Natural module coordinates → widget pixels.
    pub matrix: Matrix,
}

const ACCENT: (f64, f64, f64) = (0.21, 0.52, 0.89);
const GLOW: (f64, f64, f64) = (0.38, 0.63, 0.92);

fn kind_of(m: &ModuleState) -> Option<ModuleKind> {
    [ModuleKind::Base, ModuleKind::Keypad, ModuleKind::Fader, ModuleKind::Knob, ModuleKind::Roller, ModuleKind::Dial]
        .into_iter()
        .find(|k| k.name() == m.kind)
}

fn footprint(kind: ModuleKind) -> (f64, f64) {
    let (w, h) = masterhub_core::layout::footprint(kind);
    (w as f64, h as f64)
}

pub fn draw(model: &Model, cr: &Context, width: f64, height: f64) -> Vec<Hit> {
    // Rotation-only view: Base vertical frame (cells) → view frame (cells).
    let mut view = Matrix::identity();
    if model.hub.horizontal {
        view.translate(GRID_ROWS as f64, 0.0);
        view.rotate(PI / 2.0);
    }

    // Base first, then modules on top of it.
    let mut modules: Vec<(&ModuleState, ModuleKind)> = model.modules.iter().filter_map(|m| Some((m, kind_of(m)?))).collect();
    modules.sort_by_key(|(m, _)| m.kind != "base");

    // Modules at a known socket go where they sit (possibly overhanging the Base); modules at
    // an unknown socket go to a tray below, so they can still be seen and configured.
    let mut placed: Vec<(&ModuleState, ModuleKind, Matrix)> = Vec::new();
    let mut tray: Vec<(&ModuleState, ModuleKind)> = Vec::new();
    for &(m, kind) in &modules {
        match m.placement.or((kind == ModuleKind::Base).then_some(BASE_PLACEMENT)) {
            Some(p) => placed.push((m, kind, module_matrix(&view, p, kind))),
            None => tray.push((m, kind)),
        }
    }
    let mut bbox = BBox::default();
    for (_, kind, matrix) in &placed {
        bbox.add_rect(matrix, footprint(*kind));
    }
    if bbox.is_empty() {
        bbox.add_rect(&view, (GRID_COLS as f64, GRID_ROWS as f64));
    }
    let (mut tx, ty) = (bbox.x0, bbox.y1 + 0.7);
    for (m, kind) in &tray {
        let mut matrix = Matrix::identity();
        matrix.translate(tx, ty);
        bbox.add_rect(&matrix, footprint(*kind));
        tx += footprint(*kind).0 + 0.3;
        placed.push((m, *kind, matrix));
    }

    // Fit everything into the widget.
    let margin = 24.0;
    let (bw, bh) = (bbox.x1 - bbox.x0, bbox.y1 - bbox.y0);
    let cell = ((width - 2.0 * margin) / bw).min((height - 2.0 * margin) / bh).max(10.0);
    let mut fit = Matrix::identity();
    fit.translate((width - cell * bw) / 2.0, (height - cell * bh) / 2.0);
    fit.scale(cell, cell);
    fit.translate(-bbox.x0, -bbox.y0);

    let base_led = led_color(model, ModuleKind::Base, 0);
    let mut hits = Vec::new();
    let in_tray = placed.len() - tray.len();
    for (i, (m, kind, matrix)) in placed.into_iter().enumerate() {
        let matrix = Matrix::multiply(&matrix, &fit);
        cr.save().ok();
        cr.set_matrix(matrix);
        match kind {
            ModuleKind::Base => draw_base(cr, base_led),
            ModuleKind::Keypad => draw_keypad(cr, model, m),
            ModuleKind::Dial => draw_dial(cr, model, m),
            ModuleKind::Knob => draw_knob(cr, model, m),
            ModuleKind::Roller => draw_roller(cr, model, m),
            ModuleKind::Fader => draw_fader(cr, model, m),
        }
        if i >= in_tray {
            // Dashed outline and the socket, so it is clear the position is unknown.
            let (w, h) = footprint(kind);
            rounded(cr, 0.02, 0.02, w - 0.04, h - 0.04, 0.16);
            cr.set_dash(&[0.12, 0.08], 0.0);
            stroke_color(cr, (1.0, 0.75, 0.3), 0.8, 0.025);
            cr.set_dash(&[], 0.0);
            cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            cr.set_font_size(0.16);
            cr.set_source_rgba(1.0, 0.75, 0.3, 0.9);
            cr.move_to(0.05, -0.12);
            let socket = m.socket.as_deref().unwrap_or("?");
            cr.show_text(&format!("position unknown (socket {socket})")).ok();
        }
        if model.selection == Selection::Module(m.serial.clone()) {
            let (w, h) = footprint(kind);
            rounded(cr, 0.04, 0.04, w - 0.08, h - 0.08, 0.16);
            stroke_color(cr, ACCENT, 1.0, 0.05);
        }
        cr.restore().ok();
        hits.push(Hit { serial: m.serial.clone(), kind, matrix });
    }

    if model.modules.iter().all(|m| m.kind != "base") {
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.6);
        cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        cr.set_font_size(16.0);
        let text = "No MasterHUB connected";
        let tw = cr.text_extents(text).map_or(0.0, |e| e.width());
        cr.move_to((width - tw) / 2.0, height / 2.0);
        cr.show_text(text).ok();
    }
    hits
}

/// Bounding box in view cells.
struct BBox {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl Default for BBox {
    fn default() -> Self {
        Self { x0: f64::MAX, y0: f64::MAX, x1: f64::MIN, y1: f64::MIN }
    }
}

impl BBox {
    fn is_empty(&self) -> bool {
        self.x0 > self.x1
    }

    /// Add the natural rectangle (0,0)-(w,h) mapped through `m`.
    fn add_rect(&mut self, m: &Matrix, (w, h): (f64, f64)) {
        for (x, y) in [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)] {
            let (px, py) = m.transform_point(x, y);
            self.x0 = self.x0.min(px);
            self.y0 = self.y0.min(py);
            self.x1 = self.x1.max(px);
            self.y1 = self.y1.max(py);
        }
    }
}

/// Natural module coords → widget pixels for a module at `placement`.
fn module_matrix(view: &Matrix, p: Placement, kind: ModuleKind) -> Matrix {
    let (w, h) = footprint(kind);
    let mut m = Matrix::identity();
    m.translate(p.col as f64, p.row as f64);
    match p.rotation {
        90 => {
            m.translate(h, 0.0);
            m.rotate(PI / 2.0);
        }
        180 => {
            m.translate(w, h);
            m.rotate(PI);
        }
        270 => {
            m.translate(0.0, w);
            m.rotate(-PI / 2.0);
        }
        _ => {}
    }
    Matrix::multiply(&m, view)
}

/// Which control (or the module itself) lies under widget point (x, y).
pub fn hit_test(hits: &[Hit], x: f64, y: f64) -> Option<Selection> {
    for hit in hits.iter().rev() {
        let Ok(inv) = hit.matrix.try_invert() else { continue };
        let (u, v) = inv.transform_point(x, y);
        let (w, h) = footprint(hit.kind);
        if u < 0.0 || v < 0.0 || u > w || v > h {
            continue;
        }
        let control = match hit.kind {
            ModuleKind::Keypad => (0..15u8).find(|&i| inside(key_rect(i), u, v)).map(Selection::Key),
            ModuleKind::Dial => (dist(u, v, 1.0, 1.0) < 0.8).then_some(Selection::Dial),
            ModuleKind::Knob => (0..3u8).find(|&i| {
                let (cx, cy) = knob_center(i);
                dist(u, v, cx, cy) < 0.36
            }).map(Selection::Knob),
            ModuleKind::Roller => [Side::Left, Side::Right].into_iter().find(|&s| inside(roller_rect(s), u, v)).map(Selection::Roller),
            ModuleKind::Fader => (0..5u8).find(|&i| (u - fader_x(i)).abs() < 0.35 && (0.15..1.85).contains(&v)).map(Selection::Fader),
            ModuleKind::Base => None,
        };
        return Some(control.unwrap_or(Selection::Module(hit.serial.clone())));
    }
    None
}

// ───────────────────────────── geometry (natural coords) ─────────────────────────────

fn key_rect(i: u8) -> (f64, f64, f64, f64) {
    let (size, gap) = (0.6, 0.13);
    let x0 = (4.0 - (5.0 * size + 4.0 * gap)) / 2.0;
    let (c, r) = ((i % 5) as f64, (i / 5) as f64);
    (x0 + c * (size + gap), 0.24 + r * (size + gap), size, size)
}

fn knob_center(i: u8) -> (f64, f64) {
    match i {
        0 => (0.56, 1.38),
        1 => (1.44, 1.38),
        _ => (1.0, 0.62),
    }
}

fn roller_rect(side: Side) -> (f64, f64, f64, f64) {
    match side {
        Side::Left => (0.18, 0.2, 1.72, 0.6),
        Side::Right => (2.1, 0.2, 1.72, 0.6),
    }
}

fn fader_x(i: u8) -> f64 {
    0.4 + i as f64 * 0.8
}

fn inside((x, y, w, h): (f64, f64, f64, f64), u: f64, v: f64) -> bool {
    u >= x && u <= x + w && v >= y && v <= y + h
}

fn dist(x: f64, y: f64, cx: f64, cy: f64) -> f64 {
    ((x - cx).powi(2) + (y - cy).powi(2)).sqrt()
}

// ───────────────────────────── drawing ─────────────────────────────

fn rounded(cr: &Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -PI / 2.0, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, PI / 2.0);
    cr.arc(x + r, y + h - r, r, PI / 2.0, PI);
    cr.arc(x + r, y + r, r, PI, 1.5 * PI);
    cr.close_path();
}

fn stroke_color(cr: &Context, (r, g, b): (f64, f64, f64), a: f64, width: f64) {
    cr.set_source_rgba(r, g, b, a);
    cr.set_line_width(width);
    cr.stroke().ok();
}

/// Glossy dark module body.
fn body(cr: &Context, w: f64, h: f64) {
    rounded(cr, 0.05, 0.05, w - 0.1, h - 0.1, 0.16);
    let grad = cairo::LinearGradient::new(0.0, 0.0, 0.0, h);
    grad.add_color_stop_rgb(0.0, 0.17, 0.17, 0.19);
    grad.add_color_stop_rgb(1.0, 0.09, 0.09, 0.10);
    cr.set_source(&grad).ok();
    cr.fill_preserve().ok();
    stroke_color(cr, (1.0, 1.0, 1.0), 0.08, 0.02);
}

fn led_color(model: &Model, kind: ModuleKind, index: usize) -> Option<Rgb> {
    let mut leds = model.config.leds.clone();
    if let Some(o) = model.profile().and_then(|p| p.leds.as_ref()) {
        leds.base = o.base.or(leds.base);
        leds.fader = o.fader.clone().or(leds.fader);
        leds.knob = o.knob.clone().or(leds.knob);
    }
    let many: Option<OneOrMany<_>> = match kind {
        ModuleKind::Base => leds.base.map(OneOrMany::One),
        ModuleKind::Fader => leds.fader,
        ModuleKind::Knob => leds.knob,
        _ => None,
    };
    many.and_then(|m| m.get(index)).map(|c| c.0)
}

fn draw_base(cr: &Context, led: Option<Rgb>) {
    let (w, h) = (GRID_COLS as f64, GRID_ROWS as f64);
    if let Some(c) = led {
        // LED glow around the base edge.
        for i in 0..6 {
            let g = 0.04 + i as f64 * 0.035;
            rounded(cr, -g, -g, w + 2.0 * g, h + 2.0 * g, 0.2 + g);
            cr.set_source_rgba(c.r as f64 / 255.0, c.g as f64 / 255.0, c.b as f64 / 255.0, 0.10 - i as f64 * 0.015);
            cr.set_line_width(0.05);
            cr.stroke().ok();
        }
    }
    rounded(cr, 0.0, 0.0, w, h, 0.2);
    cr.set_source_rgb(0.06, 0.06, 0.07);
    cr.fill().ok();
    // Faint grid of sockets.
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.035);
    for c in 0..GRID_COLS {
        for r in 0..GRID_ROWS {
            rounded(cr, c as f64 + 0.08, r as f64 + 0.08, 0.84, 0.84, 0.1);
            cr.fill().ok();
        }
    }
}

fn draw_image(cr: &Context, model: &Model, serial: &str, target: u8, x: f64, y: f64, size: f64) -> bool {
    let Some(surface) = model.displays.get(&(serial.to_owned(), target)) else { return false };
    cr.save().ok();
    cr.translate(x, y);
    let s = size / surface.width() as f64;
    cr.scale(s, s);
    cr.set_source_surface(surface, 0.0, 0.0).ok();
    cr.source().set_filter(cairo::Filter::Good);
    cr.paint().ok();
    cr.restore().ok();
    true
}

fn draw_keypad(cr: &Context, model: &Model, m: &ModuleState) {
    body(cr, 4.0, 3.0);
    for i in 0..15u8 {
        let (x, y, w, h) = key_rect(i);
        rounded(cr, x - 0.03, y - 0.03, w + 0.06, h + 0.06, 0.09);
        cr.set_source_rgb(0.03, 0.03, 0.035);
        cr.fill().ok();
        cr.save().ok();
        rounded(cr, x, y, w, h, 0.07);
        cr.clip();
        if !draw_image(cr, model, &m.serial, i, x, y, w) {
            cr.set_source_rgb(0.12, 0.13, 0.15);
            cr.paint().ok();
        }
        cr.restore().ok();
        if model.is_flashing(&m.serial, "key", i) {
            rounded(cr, x - 0.02, y - 0.02, w + 0.04, h + 0.04, 0.08);
            stroke_color(cr, GLOW, 0.95, 0.05);
        }
        if model.selection == Selection::Key(i) {
            rounded(cr, x - 0.05, y - 0.05, w + 0.1, h + 0.1, 0.1);
            stroke_color(cr, ACCENT, 1.0, 0.04);
        }
    }
    // Brand strip.
    rounded(cr, 0.55, 2.5, 2.9, 0.26, 0.04);
    cr.set_source_rgb(0.13, 0.12, 0.12);
    cr.fill().ok();
    cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(0.13);
    cr.set_source_rgba(0.85, 0.75, 0.72, 0.8);
    let text = "C O O L E R   M A S T E R";
    let tw = cr.text_extents(text).map_or(0.0, |e| e.width());
    cr.move_to(2.0 - tw / 2.0, 2.68);
    cr.show_text(text).ok();
}

fn draw_dial(cr: &Context, model: &Model, m: &ModuleState) {
    body(cr, 2.0, 2.0);
    let (cx, cy) = (1.0, 1.0);
    // Knurled bezel.
    cr.arc(cx, cy, 0.8, 0.0, 2.0 * PI);
    let grad = cairo::RadialGradient::new(cx, cy - 0.3, 0.1, cx, cy, 0.85);
    grad.add_color_stop_rgb(0.0, 0.32, 0.32, 0.35);
    grad.add_color_stop_rgb(1.0, 0.12, 0.12, 0.13);
    cr.set_source(&grad).ok();
    cr.fill().ok();
    for i in 0..48 {
        let a = i as f64 / 48.0 * 2.0 * PI;
        cr.move_to(cx + 0.72 * a.cos(), cy + 0.72 * a.sin());
        cr.line_to(cx + 0.8 * a.cos(), cy + 0.8 * a.sin());
    }
    stroke_color(cr, (0.0, 0.0, 0.0), 0.5, 0.012);
    cr.save().ok();
    cr.arc(cx, cy, 0.62, 0.0, 2.0 * PI);
    cr.clip();
    if !draw_image(cr, model, &m.serial, TARGET_DIAL, cx - 0.62, cy - 0.62, 1.24) {
        cr.set_source_rgb(0.05, 0.05, 0.06);
        cr.paint().ok();
    }
    cr.restore().ok();
    if model.is_flashing(&m.serial, "dial", 0) {
        cr.arc(cx, cy, 0.82, 0.0, 2.0 * PI);
        stroke_color(cr, GLOW, 0.95, 0.05);
    }
    if model.selection == Selection::Dial {
        cr.arc(cx, cy, 0.86, 0.0, 2.0 * PI);
        stroke_color(cr, ACCENT, 1.0, 0.04);
    }
}

fn draw_knob(cr: &Context, model: &Model, m: &ModuleState) {
    body(cr, 2.0, 2.0);
    for i in 0..3u8 {
        let (cx, cy) = knob_center(i);
        if let Some(c) = led_color(model, ModuleKind::Knob, i as usize) {
            cr.arc(cx, cy, 0.36, 0.0, 2.0 * PI);
            cr.set_source_rgba(c.r as f64 / 255.0, c.g as f64 / 255.0, c.b as f64 / 255.0, 0.85);
            cr.set_line_width(0.04);
            cr.stroke().ok();
        }
        cr.arc(cx, cy, 0.3, 0.0, 2.0 * PI);
        let grad = cairo::RadialGradient::new(cx - 0.08, cy - 0.1, 0.02, cx, cy, 0.32);
        grad.add_color_stop_rgb(0.0, 0.36, 0.36, 0.39);
        grad.add_color_stop_rgb(1.0, 0.13, 0.13, 0.14);
        cr.set_source(&grad).ok();
        cr.fill().ok();
        cr.move_to(cx, cy - 0.12);
        cr.line_to(cx, cy - 0.27);
        stroke_color(cr, (0.9, 0.9, 0.9), 0.7, 0.03);
        if model.is_flashing(&m.serial, "knob", i) {
            cr.arc(cx, cy, 0.4, 0.0, 2.0 * PI);
            stroke_color(cr, GLOW, 0.95, 0.04);
        }
        if model.selection == Selection::Knob(i) {
            cr.arc(cx, cy, 0.44, 0.0, 2.0 * PI);
            stroke_color(cr, ACCENT, 1.0, 0.035);
        }
    }
}

fn draw_roller(cr: &Context, model: &Model, m: &ModuleState) {
    body(cr, 4.0, 1.0);
    for (idx, side) in [(0u8, Side::Left), (1u8, Side::Right)] {
        let (x, y, w, h) = roller_rect(side);
        rounded(cr, x, y, w, h, 0.1);
        let grad = cairo::LinearGradient::new(0.0, y, 0.0, y + h);
        grad.add_color_stop_rgb(0.0, 0.10, 0.10, 0.11);
        grad.add_color_stop_rgb(0.5, 0.30, 0.30, 0.33);
        grad.add_color_stop_rgb(1.0, 0.08, 0.08, 0.09);
        cr.set_source(&grad).ok();
        cr.fill().ok();
        for k in 0..12 {
            let ry = y + 0.04 + k as f64 * (h - 0.08) / 11.0;
            cr.move_to(x + 0.1, ry);
            cr.line_to(x + w - 0.1, ry);
        }
        stroke_color(cr, (0.0, 0.0, 0.0), 0.35, 0.012);
        if model.is_flashing(&m.serial, "roller", idx) {
            rounded(cr, x - 0.03, y - 0.03, w + 0.06, h + 0.06, 0.12);
            stroke_color(cr, GLOW, 0.95, 0.04);
        }
        if model.selection == Selection::Roller(side) {
            rounded(cr, x - 0.06, y - 0.06, w + 0.12, h + 0.12, 0.14);
            stroke_color(cr, ACCENT, 1.0, 0.035);
        }
    }
}

fn draw_fader(cr: &Context, model: &Model, m: &ModuleState) {
    body(cr, 4.0, 2.0);
    for i in 0..5u8 {
        let x = fader_x(i);
        let (y0, y1) = (0.25, 1.75);
        if let Some(c) = led_color(model, ModuleKind::Fader, i as usize) {
            cr.move_to(x, y0);
            cr.line_to(x, y1);
            cr.set_source_rgba(c.r as f64 / 255.0, c.g as f64 / 255.0, c.b as f64 / 255.0, 0.6);
            cr.set_line_width(0.1);
            cr.stroke().ok();
        }
        cr.move_to(x, y0);
        cr.line_to(x, y1);
        stroke_color(cr, (0.0, 0.0, 0.0), 0.9, 0.05);
        let pos = model.fader_pos.get(&(m.serial.clone(), i)).copied().unwrap_or(50) as f64 / 100.0;
        let ky = y1 - pos * (y1 - y0);
        rounded(cr, x - 0.2, ky - 0.12, 0.4, 0.24, 0.05);
        cr.set_source_rgb(0.25, 0.25, 0.28);
        cr.fill().ok();
        if model.is_flashing(&m.serial, "fader", i) {
            rounded(cr, x - 0.24, ky - 0.16, 0.48, 0.32, 0.07);
            stroke_color(cr, GLOW, 0.95, 0.04);
        }
        if model.selection == Selection::Fader(i) {
            rounded(cr, x - 0.3, 0.12, 0.6, 1.76, 0.1);
            stroke_color(cr, ACCENT, 1.0, 0.035);
        }
    }
}
