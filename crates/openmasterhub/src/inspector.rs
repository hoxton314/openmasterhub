//! The right-hand panel: edits whatever is selected on the hub picture.

use std::rc::Rc;

use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;
use masterhub_core::config::{Action, Color, Face, Side, Widget};
use masterhub_core::render::{KEY_SIZE, Renderer};
use masterhub_proto::Rgb;

use crate::app::App;
use crate::model::{Model, Selection};

/// Rebuild the inspector for the current selection.
pub fn build(app: &Rc<App>) -> gtk::Widget {
    let selection = app.model.borrow().selection.clone();
    let page = adw::PreferencesPage::new();
    match selection {
        Selection::None => return empty_state(),
        Selection::Key(i) => {
            let page_name = app.model.borrow().page().map(|p| p.name.clone()).unwrap_or_default();
            page.add(&face_group(app, &format!("Key {} · {page_name}", i + 1), FaceTarget::Key(i)));
            let g = adw::PreferencesGroup::builder().title("Actions").build();
            g.add(&action_row(app, "On press", move |m| &mut m.key_mut(i).press));
            g.add(&action_row(app, "On release", move |m| &mut m.key_mut(i).release));
            page.add(&g);
            page.add(&clear_group(app, "Clear key", move |m| {
                let p = m.page_mut();
                p.keys.retain(|k| k.index != i);
            }));
        }
        Selection::Dial => {
            page.add(&face_group(app, "Dial screen", FaceTarget::Dial));
            let g = adw::PreferencesGroup::builder().title("Actions").build();
            g.add(&action_row(app, "Turn clockwise", |m| &mut m.dial_mut().cw));
            g.add(&action_row(app, "Turn counter-clockwise", |m| &mut m.dial_mut().ccw));
            g.add(&action_row(app, "Press", |m| &mut m.dial_mut().press));
            g.add(&action_row(app, "Release", |m| &mut m.dial_mut().release));
            page.add(&g);
        }
        Selection::Knob(i) => {
            let name = ["Bottom-left knob", "Bottom-right knob", "Top knob"][i as usize];
            let g = adw::PreferencesGroup::builder().title(name).build();
            g.add(&action_row(app, "Turn clockwise", move |m| &mut m.knob_mut(i).cw));
            g.add(&action_row(app, "Turn counter-clockwise", move |m| &mut m.knob_mut(i).ccw));
            g.add(&action_row(app, "Press", move |m| &mut m.knob_mut(i).press));
            g.add(&action_row(app, "Release", move |m| &mut m.knob_mut(i).release));
            page.add(&g);
        }
        Selection::Fader(i) => {
            let g = adw::PreferencesGroup::builder()
                .title(format!("Fader {}", i + 1))
                .description("Runs on every movement. {value} is replaced by the position, 0 (left) to 100 (right).")
                .build();
            g.add(&action_row(app, "On change", move |m| &mut m.fader_mut(i).change));
            page.add(&g);
        }
        Selection::Roller(side) => {
            let title = if side == Side::Left { "Left roller" } else { "Right roller" };
            let g = adw::PreferencesGroup::builder().title(title).build();
            g.add(&action_row(app, "Roll away from you", move |m| &mut m.roller_mut(side).away));
            g.add(&action_row(app, "Roll toward you", move |m| &mut m.roller_mut(side).toward));
            page.add(&g);
        }
        Selection::Module(serial) => {
            for g in module_groups(app, &serial) {
                page.add(&g);
            }
        }
    }
    page.upcast()
}

fn empty_state() -> gtk::Widget {
    adw::StatusPage::builder()
        .icon_name("input-keyboard-symbolic")
        .title("Select a control")
        .description("Click a key, knob, fader, roller or the dial on the hub to configure it. Click a module's body for its lighting and details.")
        .build()
        .upcast()
}

#[derive(Clone, Copy)]
enum FaceTarget {
    Key(u8),
    Dial,
}

fn get_face(m: &Model, t: FaceTarget) -> Face {
    match t {
        FaceTarget::Key(i) => m.key(i).map(|k| k.face.clone()).unwrap_or_default(),
        FaceTarget::Dial => m.profile().and_then(|p| p.dial.as_ref()).map(|d| d.face.clone()).unwrap_or_default(),
    }
}

fn face_mut(m: &mut Model, t: FaceTarget) -> &mut Face {
    match t {
        FaceTarget::Key(i) => &mut m.key_mut(i).face,
        FaceTarget::Dial => &mut m.dial_mut().face,
    }
}

fn face_group(app: &Rc<App>, title: &str, target: FaceTarget) -> adw::PreferencesGroup {
    let face = get_face(&app.model.borrow(), target);
    let g = adw::PreferencesGroup::builder().title(title).build();

    // Local preview (widgets show placeholder data here; the hub picture shows the live one).
    let preview = gtk::Picture::builder().width_request(96).height_request(96).can_shrink(false).build();
    preview.add_css_class("face-preview");
    let refresh_preview = {
        let (app, preview) = (app.clone(), preview.clone());
        move || {
            let face = get_face(&app.model.borrow(), target);
            if let Some(tex) = preview_texture(&app.renderer, &face) {
                preview.set_paintable(Some(&tex));
            }
        }
    };
    refresh_preview();
    let preview_box = gtk::Box::builder().halign(gtk::Align::Center).margin_bottom(12).build();
    preview_box.append(&preview);
    g.add(&preview_box);

    let label = adw::EntryRow::builder().title("Label").text(face.label.clone().unwrap_or_default()).build();
    {
        let (app, rp) = (app.clone(), refresh_preview.clone());
        label.connect_changed(move |e| {
            let text = e.text().to_string();
            app.edit(|m| face_mut(m, target).label = (!text.is_empty()).then_some(text));
            rp();
        });
    }
    g.add(&label);

    g.add(&color_row(app, "Background", face.background, target, |f| &mut f.background, refresh_preview.clone()));
    g.add(&color_row(app, "Text colour", face.text_color, target, |f| &mut f.text_color, refresh_preview.clone()));

    // Image (PNG/JPEG/GIF).
    let image_row = adw::ActionRow::builder()
        .title("Image")
        .subtitle(face.image.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "None".into()))
        .subtitle_lines(1)
        .build();
    let choose = gtk::Button::builder().icon_name("document-open-symbolic").valign(gtk::Align::Center).tooltip_text("Choose image or GIF").build();
    choose.add_css_class("flat");
    let clear = gtk::Button::builder().icon_name("edit-clear-symbolic").valign(gtk::Align::Center).tooltip_text("Remove image").build();
    clear.add_css_class("flat");
    image_row.add_suffix(&choose);
    image_row.add_suffix(&clear);
    {
        let (app, row, rp) = (app.clone(), image_row.clone(), refresh_preview.clone());
        choose.connect_clicked(move |b| {
            let filter = gtk::FileFilter::new();
            filter.set_name(Some("Images"));
            for mime in ["image/png", "image/jpeg", "image/gif"] {
                filter.add_mime_type(mime);
            }
            let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&filter);
            let dialog = gtk::FileDialog::builder().title("Choose an image").filters(&filters).build();
            let (app, row, rp) = (app.clone(), row.clone(), rp.clone());
            let window = b.root().and_downcast::<gtk::Window>();
            dialog.open(window.as_ref(), gtk::gio::Cancellable::NONE, move |res| {
                if let Ok(file) = res
                    && let Some(path) = file.path()
                {
                    row.set_subtitle(&path.display().to_string());
                    app.edit(|m| face_mut(m, target).image = Some(path));
                    rp();
                }
            });
        });
    }
    {
        let (app, row, rp) = (app.clone(), image_row.clone(), refresh_preview.clone());
        clear.connect_clicked(move |_| {
            row.set_subtitle("None");
            app.edit(|m| face_mut(m, target).image = None);
            rp();
        });
    }
    g.add(&image_row);

    // Live widget.
    let names: Vec<&str> = std::iter::once("None").chain(Widget::ALL.iter().map(|w| w.title())).collect();
    let widget_row = adw::ComboRow::builder().title("Live widget").model(&gtk::StringList::new(&names)).build();
    let selected = face
        .widget
        .as_ref()
        .and_then(|w| Widget::ALL.iter().position(|x| std::mem::discriminant(x) == std::mem::discriminant(w)))
        .map_or(0, |i| i + 1);
    widget_row.set_selected(selected as u32);
    {
        let (app, rp) = (app.clone(), refresh_preview.clone());
        widget_row.connect_selected_notify(move |r| {
            let w = (r.selected() as usize).checked_sub(1).map(|i| Widget::ALL[i].clone());
            app.edit(|m| face_mut(m, target).widget = w);
            rp();
        });
    }
    g.add(&widget_row);
    g
}

fn preview_texture(renderer: &Renderer, face: &Face) -> Option<gtk::gdk::Texture> {
    use masterhub_core::render::{LiveData, load_picture, Picture};
    let picture = face.image.as_ref().and_then(|p| load_picture(p, KEY_SIZE).ok());
    let frame = picture.as_ref().map(|p| match p {
        Picture::Still(i) => i,
        Picture::Animated(f) => &f[0].0,
    });
    let live = face.widget.as_ref().map(|w| match w {
        Widget::Clock { .. } => LiveData::Text { text: "12:34".into(), caption: Some("Mon 12".into()) },
        Widget::NowPlaying => LiveData::Media { title: "Song title".into(), artist: "Artist".into(), art: None },
        other => LiveData::Gauge { fraction: 0.42, text: "42".into(), caption: other.title().split(' ').next().unwrap_or("").to_owned() },
    });
    let jpeg = renderer.render(face, KEY_SIZE, frame, live.as_ref()).ok()?;
    gtk::gdk::Texture::from_bytes(&gtk::glib::Bytes::from_owned(jpeg)).ok()
}

fn color_row(
    app: &Rc<App>,
    title: &str,
    value: Option<Color>,
    target: FaceTarget,
    field: fn(&mut Face) -> &mut Option<Color>,
    on_change: impl Fn() + Clone + 'static,
) -> adw::ActionRow {
    let row = adw::ActionRow::builder().title(title).build();
    let button = gtk::ColorDialogButton::builder()
        .dialog(&gtk::ColorDialog::builder().with_alpha(false).build())
        .valign(gtk::Align::Center)
        .build();
    let rgb = value.map(|c| c.0).unwrap_or(Rgb::new(0, 0, 0));
    button.set_rgba(&gtk::gdk::RGBA::new(rgb.r as f32 / 255.0, rgb.g as f32 / 255.0, rgb.b as f32 / 255.0, 1.0));
    let reset = gtk::Button::builder().icon_name("edit-undo-symbolic").valign(gtk::Align::Center).tooltip_text("Default").build();
    reset.add_css_class("flat");
    row.add_suffix(&button);
    row.add_suffix(&reset);
    {
        let (app, on_change) = (app.clone(), on_change.clone());
        button.connect_rgba_notify(move |b| {
            let c = b.rgba();
            let rgb = Rgb::new((c.red() * 255.0).round() as u8, (c.green() * 255.0).round() as u8, (c.blue() * 255.0).round() as u8);
            app.edit(|m| *field(face_mut(m, target)) = Some(Color(rgb)));
            on_change();
        });
    }
    {
        let app = app.clone();
        reset.connect_clicked(move |_| {
            app.edit(|m| *field(face_mut(m, target)) = None);
            on_change();
        });
    }
    row
}

fn clear_group(app: &Rc<App>, label: &str, clear: impl Fn(&mut Model) + 'static) -> adw::PreferencesGroup {
    let g = adw::PreferencesGroup::new();
    let button = gtk::Button::builder().label(label).halign(gtk::Align::Start).build();
    button.add_css_class("destructive-action");
    let app = app.clone();
    button.connect_clicked(move |_| {
        app.edit(&clear);
        app.rebuild_inspector();
    });
    g.add(&button);
    g
}

// ───────────────────────────── action editor ─────────────────────────────

const KINDS: [&str; 9] = [
    "Nothing",
    "Shell command",
    "Key combination",
    "Next page",
    "Previous page",
    "Go to page",
    "Open folder",
    "Back (leave folder)",
    "Switch profile",
];

fn kind_index(a: &Option<Action>) -> u32 {
    match a {
        None => 0,
        Some(Action::Cmd(_)) => 1,
        Some(Action::Keys(_)) => 2,
        Some(Action::Page(p)) if p == "next" => 3,
        Some(Action::Page(p)) if p == "prev" => 4,
        Some(Action::Page(_)) => 5,
        Some(Action::Folder(_)) => 6,
        Some(Action::Back(_)) => 7,
        Some(Action::Profile(_)) => 8,
    }
}

fn arg_of(a: &Option<Action>) -> String {
    match a {
        Some(Action::Cmd(s) | Action::Keys(s) | Action::Folder(s) | Action::Profile(s)) => s.clone(),
        Some(Action::Page(p)) if p != "next" && p != "prev" => p.clone(),
        _ => String::new(),
    }
}

/// An expander row choosing an action kind plus its argument.
fn action_row(app: &Rc<App>, title: &str, slot: impl Fn(&mut Model) -> &mut Option<Action> + Clone + 'static) -> adw::ExpanderRow {
    let current = slot(&mut app.model.borrow_mut()).clone();
    // Reading through `slot` may create an empty entry; drop it again.
    app.model.borrow_mut().tidy();

    let row = adw::ExpanderRow::builder().title(title).subtitle(describe(&current)).build();
    let kind = adw::ComboRow::builder().title("Action").model(&gtk::StringList::new(&KINDS)).build();
    kind.set_selected(kind_index(&current));
    let text = adw::EntryRow::builder().title("Command").text(arg_of(&current)).build();
    let names = adw::ComboRow::builder().title("Target").build();
    row.add_row(&kind);
    row.add_row(&text);
    row.add_row(&names);

    let update_visibility = {
        let (app, text, names) = (app.clone(), text.clone(), names.clone());
        move |k: u32| {
            text.set_visible(k == 1 || k == 2);
            text.set_title(if k == 2 { "Keys, e.g. ctrl+shift+m or playpause" } else { "Command" });
            names.set_visible(matches!(k, 5 | 6 | 8));
            let list: Vec<String> = {
                let m = app.model.borrow();
                let p = m.profile();
                match k {
                    5 => p.map(|p| p.pages.iter().map(|x| x.name.clone()).collect()).unwrap_or_default(),
                    6 => p.map(|p| p.folders.iter().map(|x| x.name.clone()).collect()).unwrap_or_default(),
                    8 => m.config.profiles.iter().map(|x| x.name.clone()).collect(),
                    _ => Vec::new(),
                }
            };
            let refs: Vec<&str> = list.iter().map(String::as_str).collect();
            names.set_model(Some(&gtk::StringList::new(&refs)));
        }
    };
    update_visibility(kind.selected());
    if matches!(kind.selected(), 5 | 6 | 8)
        && let Some(model) = names.model().and_downcast::<gtk::StringList>()
    {
        let arg = arg_of(&current);
        if let Some(i) = (0..model.n_items()).find(|&i| model.string(i).is_some_and(|s| s == arg)) {
            names.set_selected(i);
        }
    }

    let commit = {
        let (app, kind, text, names, row) = (app.clone(), kind.clone(), text.clone(), names.clone(), row.clone());
        move || {
            let name = names.selected_item().and_downcast::<gtk::StringObject>().map(|s| s.string().to_string());
            let t = text.text().to_string();
            let action = match kind.selected() {
                1 if !t.is_empty() => Some(Action::Cmd(t)),
                2 if !t.is_empty() => Some(Action::Keys(t)),
                3 => Some(Action::Page("next".into())),
                4 => Some(Action::Page("prev".into())),
                5 => name.map(Action::Page),
                6 => name.map(Action::Folder),
                7 => Some(Action::Back(true)),
                8 => name.map(Action::Profile),
                _ => None,
            };
            row.set_subtitle(&describe(&action));
            let slot = slot.clone();
            app.edit(move |m| *slot(m) = action);
        }
    };
    {
        let (commit, uv) = (commit.clone(), update_visibility.clone());
        kind.connect_selected_notify(move |k| {
            uv(k.selected());
            commit();
        });
    }
    {
        let commit = commit.clone();
        text.connect_changed(move |_| commit());
    }
    names.connect_selected_notify(move |_| commit());
    row
}

fn describe(a: &Option<Action>) -> String {
    match a {
        None => "Nothing".into(),
        Some(Action::Cmd(c)) => format!("Run: {c}"),
        Some(Action::Keys(k)) => format!("Keys: {k}"),
        Some(Action::Page(p)) if p == "next" => "Next page".into(),
        Some(Action::Page(p)) if p == "prev" => "Previous page".into(),
        Some(Action::Page(p)) => format!("Go to page “{p}”"),
        Some(Action::Folder(f)) => format!("Open folder “{f}”"),
        Some(Action::Back(_)) => "Leave folder".into(),
        Some(Action::Profile(p)) => format!("Switch to profile “{p}”"),
    }
}

// ───────────────────────────── module details & lighting ─────────────────────────────

fn module_groups(app: &Rc<App>, serial: &str) -> Vec<adw::PreferencesGroup> {
    let Some(module) = app.model.borrow().modules.iter().find(|m| m.serial == serial).cloned() else { return Vec::new() };
    let mut groups = Vec::new();

    let info = adw::PreferencesGroup::builder().title(title_of(&module.kind)).build();
    for (k, v) in [
        ("Serial number", module.serial.clone()),
        ("Firmware", module.firmware.clone().unwrap_or_else(|| "unknown".into())),
        ("Socket", module.socket.clone().unwrap_or_else(|| "unknown".into())),
    ] {
        let row = adw::ActionRow::builder().title(k).subtitle(v).subtitle_selectable(true).build();
        row.add_css_class("property");
        info.add(&row);
    }
    groups.push(info);

    let led_count = match module.kind.as_str() {
        "base" => 1,
        "fader" => 5,
        "knob" => 3,
        _ => 0,
    };
    if led_count > 0 {
        let g = adw::PreferencesGroup::builder()
            .title("Lighting")
            .description("Shared by all profiles. Static colours, applied immediately.")
            .build();
        let names: Vec<String> = match module.kind.as_str() {
            "base" => vec!["Base LED".into()],
            "knob" => vec!["Bottom-left knob".into(), "Bottom-right knob".into(), "Top knob".into()],
            _ => (1..=5).map(|i| format!("Fader {i}")).collect(),
        };
        for (i, name) in names.into_iter().enumerate() {
            g.add(&led_row(app, &module.kind, i, &name));
        }
        let brightness = adw::SpinRow::with_range(1.0, 100.0, 5.0);
        brightness.set_title("LED brightness");
        brightness.set_value(app.model.borrow().config.leds.brightness.unwrap_or(100) as f64);
        let a = app.clone();
        brightness.connect_value_notify(move |s| {
            let v = s.value() as u8;
            a.edit(|m| m.config.leds.brightness = Some(v));
        });
        g.add(&brightness);
        groups.push(g);
    }

    if module.kind == "base" || module.kind == "keypad" || module.kind == "dial" {
        let g = adw::PreferencesGroup::builder().title("Screens").build();
        let backlight = adw::SpinRow::with_range(0.0, 100.0, 5.0);
        backlight.set_title("Screen brightness");
        backlight.set_subtitle("Keypad and dial backlight");
        backlight.set_value(app.model.borrow().config.backlight as f64);
        let a = app.clone();
        backlight.connect_value_notify(move |s| {
            let v = s.value() as u8;
            a.edit(|m| m.config.backlight = v);
        });
        g.add(&backlight);
        groups.push(g);
    }
    groups
}

fn led_row(app: &Rc<App>, kind: &str, index: usize, title: &str) -> adw::ActionRow {
    use masterhub_core::config::OneOrMany;
    let get = |m: &Model| -> Option<Color> {
        match kind {
            "base" => m.config.leds.base,
            "fader" => m.config.leds.fader.as_ref().and_then(|v| v.get(index)),
            _ => m.config.leds.knob.as_ref().and_then(|v| v.get(index)),
        }
    };
    let row = adw::ActionRow::builder().title(title).build();
    let button = gtk::ColorDialogButton::builder()
        .dialog(&gtk::ColorDialog::builder().with_alpha(false).build())
        .valign(gtk::Align::Center)
        .build();
    let c = get(&app.model.borrow()).map(|c| c.0).unwrap_or(Rgb::new(255, 255, 255));
    button.set_rgba(&gtk::gdk::RGBA::new(c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0, 1.0));
    row.add_suffix(&button);
    let (app, kind) = (app.clone(), kind.to_owned());
    let count = if kind == "fader" { 5 } else { 3 };
    button.connect_rgba_notify(move |b| {
        let c = b.rgba();
        let color = Color(Rgb::new((c.red() * 255.0).round() as u8, (c.green() * 255.0).round() as u8, (c.blue() * 255.0).round() as u8));
        app.edit(|m| {
            let expand = |v: &Option<OneOrMany<Color>>| -> Vec<Color> {
                (0..count).map(|i| v.as_ref().and_then(|x| x.get(i)).unwrap_or(Color(Rgb::new(255, 255, 255)))).collect()
            };
            match kind.as_str() {
                "base" => m.config.leds.base = Some(color),
                "fader" => {
                    let mut v = expand(&m.config.leds.fader);
                    v[index] = color;
                    m.config.leds.fader = Some(OneOrMany::Many(v));
                }
                _ => {
                    let mut v = expand(&m.config.leds.knob);
                    v[index] = color;
                    m.config.leds.knob = Some(OneOrMany::Many(v));
                }
            }
        });
    });
    row
}

fn title_of(kind: &str) -> &'static str {
    match kind {
        "base" => "Base",
        "keypad" => "15-key display pad",
        "fader" => "5 faders",
        "knob" => "3 knobs",
        "roller" => "2 rollers",
        "dial" => "Dial with display",
        _ => "Module",
    }
}
