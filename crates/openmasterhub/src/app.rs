//! Main window: header with profile picker, the hub picture, the page/folder bar and the
//! inspector. Every edit is applied live: the config is sent to the service (debounced),
//! which saves it and updates the hardware; the hub picture mirrors the hardware.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use gtk4 as gtk;
use gtk4::glib;
use libadwaita as adw;
use masterhub_core::config::{Config, Page, Profile};
use masterhub_core::render::Renderer;

use crate::canvas::{self, Hit};
use crate::client::{Client, Event};
use crate::inspector;
use crate::model::{FLASH, Model, Selection, jpeg_surface};

type PageOp = fn(&Rc<App>, &str);
type ProfileOp = fn(&Rc<App>);

pub struct App {
    pub model: RefCell<Model>,
    pub renderer: Renderer,
    client: Client,
    window: adw::ApplicationWindow,
    toasts: adw::ToastOverlay,
    stack: gtk::Stack,
    title: adw::WindowTitle,
    canvas: gtk::DrawingArea,
    hits: RefCell<Vec<Hit>>,
    inspector: adw::Bin,
    pages: gtk::Box,
    profiles: gtk::DropDown,
    updating: Cell<bool>,
    save_timer: RefCell<Option<glib::SourceId>>,
}

impl App {
    pub fn new(application: &adw::Application) -> anyhow::Result<Rc<Self>> {
        let (tx, rx) = async_channel::unbounded();
        let client = Client::connect(tx)?;

        let window = adw::ApplicationWindow::builder()
            .application(application)
            .title("OpenMasterHUB")
            .default_width(1240)
            .default_height(800)
            .build();
        let title = adw::WindowTitle::new("OpenMasterHUB", "");
        let header = adw::HeaderBar::builder().title_widget(&title).build();
        let profiles = gtk::DropDown::builder().tooltip_text("Active profile").build();
        header.pack_start(&profiles);

        let canvas = gtk::DrawingArea::builder().hexpand(true).vexpand(true).build();
        canvas.add_css_class("hub-canvas");
        let pages = gtk::Box::builder().spacing(6).margin_start(18).margin_end(18).margin_bottom(14).margin_top(6).build();
        pages.add_css_class("page-bar");
        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.append(&canvas);
        content.append(&pages);

        let inspector = adw::Bin::new();
        let sidebar = gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).child(&inspector).build();
        let split = adw::OverlaySplitView::builder()
            .content(&content)
            .sidebar(&sidebar)
            .sidebar_position(gtk::PackType::End)
            .min_sidebar_width(360.0)
            .max_sidebar_width(440.0)
            .sidebar_width_fraction(0.34)
            .show_sidebar(true)
            .build();

        let stack = gtk::Stack::builder().transition_type(gtk::StackTransitionType::Crossfade).build();
        stack.add_named(&split, Some("main"));
        let start_button = gtk::Button::builder().label("Start service").halign(gtk::Align::Center).build();
        start_button.add_css_class("pill");
        start_button.add_css_class("suggested-action");
        let offline = adw::StatusPage::builder()
            .icon_name("network-offline-symbolic")
            .title("The MasterHUB service isn’t running")
            .description("The app edits the configuration of <b>masterhubd</b>, which drives the hardware.\nStart it, or run <tt>masterhubd</tt> in a terminal.")
            .child(&start_button)
            .build();
        stack.add_named(&offline, Some("offline"));

        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&stack));
        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&toolbar));
        window.set_content(Some(&toasts));

        let app = Rc::new(Self {
            model: RefCell::new(Model::new()),
            renderer: Renderer::new(None),
            client,
            window,
            toasts,
            stack,
            title,
            canvas,
            hits: RefCell::new(Vec::new()),
            inspector,
            pages,
            profiles,
            updating: Cell::new(false),
            save_timer: RefCell::new(None),
        });

        app.add_profile_menu(&header);
        app.connect_canvas();
        {
            let a = app.clone();
            app.profiles.connect_selected_notify(move |d| {
                if a.updating.get() {
                    return;
                }
                let Some(name) = d.selected_item().and_downcast::<gtk::StringObject>().map(|s| s.string().to_string()) else { return };
                a.model.borrow_mut().hub.profile = name.clone();
                a.model.borrow_mut().hub.page.clear();
                if let Err(e) = a.client.set_profile(&name) {
                    a.toast(&format!("{e:#}"));
                }
                a.refresh_all();
            });
        }
        {
            let a = app.clone();
            start_button.connect_clicked(move |_| {
                let ok = std::process::Command::new("systemctl").args(["--user", "start", "masterhubd"]).status().is_ok_and(|s| s.success());
                if !ok {
                    a.toast("Could not start masterhubd.service. Run masterhubd in a terminal instead.");
                }
            });
        }
        {
            let a = app.clone();
            glib::spawn_future_local(async move {
                while let Ok(event) = rx.recv().await {
                    a.handle(event);
                }
            });
        }

        if app.client.is_online() {
            app.reload();
        } else {
            app.set_online(false);
        }
        app.window.present();
        Ok(app)
    }

    // ───────────────────────────── service sync ─────────────────────────────

    fn set_online(&self, online: bool) {
        self.model.borrow_mut().online = online;
        self.stack.set_visible_child_name(if online { "main" } else { "offline" });
        if !online {
            self.title.set_subtitle("service offline");
        }
    }

    /// Fetch everything from the service.
    fn reload(self: &Rc<Self>) {
        let result = (|| -> anyhow::Result<()> {
            let text = self.client.config()?;
            let path = PathBuf::from(self.client.config_path()?);
            let dir = path.parent().unwrap_or(Path::new(".")).to_owned();
            let config = Config::parse(&text, &dir)?;
            let modules = self.client.modules()?;
            let hub = self.client.state()?;
            let displays = self.client.displays()?;
            let mut m = self.model.borrow_mut();
            m.config = config;
            m.config_dir = dir;
            m.modules = modules;
            m.hub = hub;
            m.displays = displays.into_iter().filter_map(|(s, t, j)| Some(((s, t), jpeg_surface(&j)?))).collect();
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.set_online(true);
                self.refresh_all();
            }
            Err(e) => {
                log::warn!("reload: {e:#}");
                self.set_online(self.client.is_online());
            }
        }
    }

    fn handle(self: &Rc<Self>, event: Event) {
        match event {
            Event::Online(true) => {
                // Give the service a moment to finish starting up.
                let a = self.clone();
                glib::timeout_add_local_once(Duration::from_millis(600), move || a.reload());
            }
            Event::Online(false) => self.set_online(false),
            Event::Input(input) => {
                {
                    let mut m = self.model.borrow_mut();
                    m.flash(&input.serial, &input.control, input.index);
                    if let Some(v) = input.value {
                        m.fader_pos.insert((input.serial.clone(), input.index), v);
                    }
                }
                self.canvas.queue_draw();
                let canvas = self.canvas.clone();
                glib::timeout_add_local_once(FLASH + Duration::from_millis(20), move || canvas.queue_draw());
            }
            Event::State(state) => {
                let changed = {
                    let mut m = self.model.borrow_mut();
                    let changed = m.hub.profile != state.profile || m.hub.page != state.page;
                    m.hub = state;
                    changed
                };
                if changed {
                    self.refresh_all();
                } else {
                    self.update_title();
                    self.canvas.queue_draw();
                }
            }
            Event::Modules(modules) => {
                self.model.borrow_mut().modules = modules;
                self.canvas.queue_draw();
            }
            Event::Display { serial, target, jpeg } => {
                if let Some(s) = jpeg_surface(&jpeg) {
                    self.model.borrow_mut().displays.insert((serial, target), s);
                    self.canvas.queue_draw();
                }
            }
            Event::ConfigChanged => {
                // Ignore the echo of our own save; reload edits made elsewhere (e.g. by hand).
                let ours = match self.client.config() {
                    Ok(text) => self.model.borrow().last_sent.as_deref() == Some(text.as_str()),
                    Err(_) => false,
                };
                if !ours && self.save_timer.borrow().is_none() {
                    self.reload();
                }
            }
        }
    }

    /// Apply an edit to the config, redraw, and save soon.
    pub fn edit(self: &Rc<Self>, f: impl FnOnce(&mut Model)) {
        {
            let mut m = self.model.borrow_mut();
            f(&mut m);
            m.tidy();
        }
        self.canvas.queue_draw();
        if let Some(id) = self.save_timer.borrow_mut().take() {
            id.remove();
        }
        let a = self.clone();
        let id = glib::timeout_add_local_once(Duration::from_millis(350), move || {
            a.save_timer.borrow_mut().take();
            a.save();
        });
        *self.save_timer.borrow_mut() = Some(id);
    }

    /// Save immediately (cancelling a pending debounced save).
    fn save_now(&self) {
        if let Some(id) = self.save_timer.borrow_mut().take() {
            id.remove();
        }
        self.save();
    }

    fn save(&self) {
        let text = match self.model.borrow().config.to_toml() {
            Ok(t) => t,
            Err(e) => return self.toast(&format!("Could not serialise config: {e}")),
        };
        self.model.borrow_mut().last_sent = Some(text.clone());
        if let Err(e) = self.client.set_config(&text) {
            self.toast(&format!("Not saved: {e:#}"));
        }
    }

    pub fn toast(&self, text: &str) {
        self.toasts.add_toast(adw::Toast::builder().title(glib::markup_escape_text(text)).timeout(5).build());
    }

    // ───────────────────────────── UI refresh ─────────────────────────────

    fn refresh_all(self: &Rc<Self>) {
        self.update_title();
        self.rebuild_profiles();
        self.rebuild_pages();
        self.rebuild_inspector();
        self.canvas.queue_draw();
    }

    fn update_title(&self) {
        let m = self.model.borrow();
        let n = m.modules.iter().filter(|x| x.kind != "base").count();
        let mut sub = format!("{n} module{}", if n == 1 { "" } else { "s" });
        if let Some(app) = m.hub.active_app.as_deref().filter(|a| !a.is_empty()) {
            sub.push_str(&format!(" · focused: {app}"));
        }
        self.title.set_subtitle(&sub);
    }

    pub fn rebuild_inspector(self: &Rc<Self>) {
        self.inspector.set_child(Some(&inspector::build(self)));
    }

    fn rebuild_profiles(&self) {
        self.updating.set(true);
        let m = self.model.borrow();
        let names: Vec<&str> = m.config.profiles.iter().map(|p| p.name.as_str()).collect();
        let names = if names.is_empty() { vec!["Default"] } else { names };
        self.profiles.set_model(Some(&gtk::StringList::new(&names)));
        self.profiles.set_selected(m.profile_index().unwrap_or(0) as u32);
        drop(m);
        self.updating.set(false);
    }

    fn rebuild_pages(self: &Rc<Self>) {
        while let Some(child) = self.pages.first_child() {
            self.pages.remove(&child);
        }
        let (pages, folders, current) = {
            let m = self.model.borrow();
            let p = m.profile();
            (
                p.map(|p| p.pages.iter().map(|x| x.name.clone()).collect::<Vec<_>>()).unwrap_or_default(),
                p.map(|p| p.folders.iter().map(|x| x.name.clone()).collect::<Vec<_>>()).unwrap_or_default(),
                m.page().map(|p| p.name.clone()),
            )
        };

        let section = |label: &str| {
            let l = gtk::Label::new(Some(label));
            l.add_css_class("dim-label");
            l.add_css_class("caption-heading");
            l
        };
        self.pages.append(&section("PAGES"));
        let page_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        page_box.add_css_class("linked");
        let mut group: Option<gtk::ToggleButton> = None;
        for name in pages.iter().map(|n| (n, false)).chain(folders.iter().map(|n| (n, true))) {
            let (name, is_folder) = name;
            let first_folder = is_folder && folders.first() == Some(name);
            let b = gtk::ToggleButton::new();
            let inner = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            if is_folder {
                inner.append(&gtk::Image::from_icon_name("folder-symbolic"));
            }
            inner.append(&gtk::Label::new(Some(name)));
            b.set_child(Some(&inner));
            b.set_active(current.as_deref() == Some(name.as_str()));
            if let Some(g) = &group {
                b.set_group(Some(g));
            } else {
                group = Some(b.clone());
            }
            let (a, name) = (self.clone(), name.clone());
            b.connect_clicked(move |b| {
                if b.is_active() {
                    a.show_page(&name);
                }
            });
            if first_folder {
                page_box.append(&gtk::Separator::new(gtk::Orientation::Vertical));
            }
            page_box.append(&b);
        }
        self.pages.append(&page_box);

        let add = gtk::MenuButton::builder().icon_name("list-add-symbolic").tooltip_text("Add page or folder").build();
        add.add_css_class("flat");
        let pop = gtk::Popover::new();
        let pbox = gtk::Box::new(gtk::Orientation::Vertical, 2);
        for (label, folder) in [("New page", false), ("New folder", true)] {
            let b = gtk::Button::builder().label(label).build();
            b.add_css_class("flat");
            let (a, pop) = (self.clone(), pop.clone());
            b.connect_clicked(move |_| {
                pop.popdown();
                a.add_page(folder);
            });
            pbox.append(&b);
        }
        pop.set_child(Some(&pbox));
        add.set_popover(Some(&pop));
        self.pages.append(&add);

        let spacer = gtk::Box::builder().hexpand(true).build();
        self.pages.append(&spacer);

        if let Some(current) = current {
            let more = gtk::MenuButton::builder().icon_name("view-more-symbolic").tooltip_text(format!("“{current}”")).build();
            more.add_css_class("flat");
            let pop = gtk::Popover::new();
            let pbox = gtk::Box::new(gtk::Orientation::Vertical, 2);
            let items: [(&str, PageOp); 4] = [
                ("Rename…", |a, n| a.rename_page(n)),
                ("Move left", |a, n| a.move_page(n, -1)),
                ("Move right", |a, n| a.move_page(n, 1)),
                ("Delete", |a, n| a.delete_page(n)),
            ];
            for (label, f) in items {
                let b = gtk::Button::builder().label(label).build();
                b.add_css_class("flat");
                if label == "Delete" {
                    b.add_css_class("destructive-action");
                }
                let (a, pop, n) = (self.clone(), pop.clone(), current.clone());
                b.connect_clicked(move |_| {
                    pop.popdown();
                    f(&a, &n);
                });
                pbox.append(&b);
            }
            pop.set_child(Some(&pbox));
            more.set_popover(Some(&pop));
            self.pages.append(&more);
        }
    }

    // ───────────────────────────── canvas ─────────────────────────────

    fn connect_canvas(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.canvas.set_draw_func(move |_, cr, w, h| {
            if let Some(a) = weak.upgrade() {
                let hits = canvas::draw(&a.model.borrow(), cr, w as f64, h as f64);
                *a.hits.borrow_mut() = hits;
            }
        });
        let click = gtk::GestureClick::new();
        let a = self.clone();
        click.connect_pressed(move |_, _, x, y| {
            let sel = canvas::hit_test(&a.hits.borrow(), x, y).unwrap_or(Selection::None);
            a.model.borrow_mut().selection = sel;
            a.rebuild_inspector();
            a.canvas.queue_draw();
        });
        self.canvas.add_controller(click);
    }

    // ───────────────────────────── pages & folders ─────────────────────────────

    fn show_page(self: &Rc<Self>, name: &str) {
        self.model.borrow_mut().hub.page = name.to_owned();
        if let Err(e) = self.client.show_page(name) {
            self.toast(&format!("{e:#}"));
        }
        if matches!(self.model.borrow().selection, Selection::Key(_)) {
            self.rebuild_inspector();
        }
        self.canvas.queue_draw();
    }

    fn add_page(self: &Rc<Self>, folder: bool) {
        let name = {
            let m = self.model.borrow();
            let existing: Vec<String> = m.profile().map(|p| p.pages.iter().chain(&p.folders).map(|x| x.name.clone()).collect()).unwrap_or_default();
            let base = if folder { "Folder" } else { "Page" };
            (1..).map(|i| format!("{base} {i}")).find(|n| !existing.contains(n)).expect("infinite")
        };
        {
            let mut m = self.model.borrow_mut();
            let p = m.profile_mut();
            let page = Page { name: name.clone(), keys: Vec::new() };
            if folder {
                p.folders.push(page);
            } else {
                p.pages.push(page);
            }
        }
        self.save_now();
        self.show_page(&name);
        self.rebuild_pages();
    }

    fn rename_page(self: &Rc<Self>, old: &str) {
        let a = self.clone();
        let old = old.to_owned();
        self.ask_name("Rename", &old.clone(), move |new| {
            if new == old {
                return;
            }
            let mut m = a.model.borrow_mut();
            let p = m.profile_mut();
            if p.pages.iter().chain(&p.folders).any(|x| x.name == new) {
                drop(m);
                return a.toast("A page or folder with that name already exists");
            }
            for page in p.pages.iter_mut().chain(p.folders.iter_mut()) {
                if page.name == old {
                    page.name = new.clone();
                }
                // Keep actions pointing at the renamed page/folder.
                for k in &mut page.keys {
                    for act in [&mut k.press, &mut k.release].into_iter().flatten() {
                        match act {
                            masterhub_core::config::Action::Page(n) | masterhub_core::config::Action::Folder(n) if *n == old => *n = new.clone(),
                            _ => {}
                        }
                    }
                }
            }
            m.hub.page = new.clone();
            drop(m);
            a.save_now();
            a.show_page(&new);
            a.rebuild_pages();
        });
    }

    fn move_page(self: &Rc<Self>, name: &str, delta: isize) {
        {
            let mut m = self.model.borrow_mut();
            let p = m.profile_mut();
            for list in [&mut p.pages, &mut p.folders] {
                if let Some(i) = list.iter().position(|x| x.name == name) {
                    let j = (i as isize + delta).clamp(0, list.len() as isize - 1) as usize;
                    list.swap(i, j);
                }
            }
        }
        self.save_now();
        self.rebuild_pages();
    }

    fn delete_page(self: &Rc<Self>, name: &str) {
        let referenced = {
            let m = self.model.borrow();
            m.profile().is_some_and(|p| {
                p.actions().any(|a| matches!(a, masterhub_core::config::Action::Folder(n) | masterhub_core::config::Action::Page(n) if n == name))
            })
        };
        if referenced {
            return self.toast("Remove the keys that open this page or folder first");
        }
        let a = self.clone();
        let name = name.to_owned();
        self.confirm(&format!("Delete “{name}”?"), "Its keys are deleted too.", move || {
            {
                let mut m = a.model.borrow_mut();
                let p = m.profile_mut();
                p.pages.retain(|x| x.name != name);
                p.folders.retain(|x| x.name != name);
                let first = p.pages.first().map(|x| x.name.clone()).unwrap_or_default();
                m.hub.page = first.clone();
            }
            a.save_now();
            let first = a.model.borrow().hub.page.clone();
            if !first.is_empty() {
                a.show_page(&first);
            }
            a.refresh_all();
        });
    }

    // ───────────────────────────── profiles ─────────────────────────────

    fn add_profile_menu(self: &Rc<Self>, header: &adw::HeaderBar) {
        let menu = gtk::MenuButton::builder().icon_name("document-properties-symbolic").tooltip_text("Profile settings").build();
        let pop = gtk::Popover::new();
        let pbox = gtk::Box::new(gtk::Orientation::Vertical, 2);
        let items: [(&str, ProfileOp); 4] = [
            ("New profile", |a| a.new_profile()),
            ("Rename profile…", |a| a.rename_profile()),
            ("Activate for apps…", |a| a.edit_profile_apps()),
            ("Delete profile", |a| a.delete_profile()),
        ];
        for (label, f) in items {
            let b = gtk::Button::builder().label(label).build();
            b.add_css_class("flat");
            let (a, pop) = (self.clone(), pop.clone());
            b.connect_clicked(move |_| {
                pop.popdown();
                f(&a);
            });
            pbox.append(&b);
        }
        pop.set_child(Some(&pbox));
        menu.set_popover(Some(&pop));
        header.pack_start(&menu);
    }

    fn new_profile(self: &Rc<Self>) {
        let a = self.clone();
        self.ask_name("New profile", "", move |name| {
            if name.is_empty() || a.model.borrow().config.profiles.iter().any(|p| p.name == name) {
                return a.toast("Choose a new, unique name");
            }
            {
                let mut m = a.model.borrow_mut();
                if m.config.profiles.is_empty() {
                    m.config.profiles.push(Profile { name: "Default".into(), ..Default::default() });
                }
                m.config.profiles.push(Profile {
                    name: name.clone(),
                    pages: vec![Page { name: "Main".into(), keys: Vec::new() }],
                    ..Default::default()
                });
                m.hub.profile = name.clone();
                m.hub.page = "Main".into();
            }
            a.save_now();
            let _ = a.client.set_profile(&name);
            a.refresh_all();
        });
    }

    fn rename_profile(self: &Rc<Self>) {
        let Some(old) = self.model.borrow().profile().map(|p| p.name.clone()) else { return };
        let a = self.clone();
        self.ask_name("Rename profile", &old.clone(), move |new| {
            if new.is_empty() || new == old {
                return;
            }
            {
                let mut m = a.model.borrow_mut();
                if m.config.profiles.iter().any(|p| p.name == new) {
                    drop(m);
                    return a.toast("A profile with that name already exists");
                }
                for p in &mut m.config.profiles {
                    if p.name == old {
                        p.name = new.clone();
                    }
                    for page in p.pages.iter_mut().chain(p.folders.iter_mut()) {
                        for k in &mut page.keys {
                            for act in [&mut k.press, &mut k.release].into_iter().flatten() {
                                if let masterhub_core::config::Action::Profile(n) = act
                                    && *n == old
                                {
                                    *n = new.clone();
                                }
                            }
                        }
                    }
                }
                m.hub.profile = new.clone();
            }
            a.save_now();
            let _ = a.client.set_profile(&new);
            a.refresh_all();
        });
    }

    fn delete_profile(self: &Rc<Self>) {
        let (name, index) = {
            let m = self.model.borrow();
            (m.profile().map(|p| p.name.clone()), m.profile_index())
        };
        let (Some(name), Some(index)) = (name, index) else { return };
        if index == 0 {
            return self.toast("The default (first) profile can’t be deleted");
        }
        let a = self.clone();
        self.confirm(&format!("Delete profile “{name}”?"), "All its pages, folders and bindings are deleted.", move || {
            {
                let mut m = a.model.borrow_mut();
                m.config.profiles.remove(index);
                m.hub.profile = m.config.profiles[0].name.clone();
            }
            a.save_now();
            let first = a.model.borrow().hub.profile.clone();
            let _ = a.client.set_profile(&first);
            a.refresh_all();
        });
    }

    /// Edit which window classes activate the profile, with open windows as suggestions.
    fn edit_profile_apps(self: &Rc<Self>) {
        let Some((name, apps, index)) = ({
            let m = self.model.borrow();
            m.profile().map(|p| (p.name.clone(), p.apps.clone(), m.profile_index().unwrap_or(0)))
        }) else {
            return;
        };
        if index == 0 {
            return self.toast("The default profile is used when no other profile matches; give apps to another profile");
        }
        let dialog = adw::AlertDialog::new(Some(&format!("Apps for “{name}”")), Some("The profile activates when a window with one of these classes is focused. Separate with commas; * is a wildcard."));
        let entry = gtk::Entry::builder().text(apps.join(", ")).activates_default(true).build();
        let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
        content.append(&entry);
        let open = open_window_classes();
        if !open.is_empty() {
            let label = gtk::Label::builder().label("Open windows").halign(gtk::Align::Start).build();
            label.add_css_class("dim-label");
            content.append(&label);
            let flow = gtk::FlowBox::builder().selection_mode(gtk::SelectionMode::None).max_children_per_line(3).build();
            for class in open {
                let b = gtk::Button::builder().label(&class).build();
                b.add_css_class("pill");
                b.add_css_class("small");
                let e = entry.clone();
                b.connect_clicked(move |_| {
                    let mut items: Vec<String> = e.text().split(',').map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()).collect();
                    if !items.contains(&class) {
                        items.push(class.clone());
                    }
                    e.set_text(&items.join(", "));
                });
                flow.insert(&b, -1);
            }
            content.append(&flow);
        }
        dialog.set_extra_child(Some(&content));
        dialog.add_responses(&[("cancel", "Cancel"), ("save", "Save")]);
        dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("save"));
        let a = self.clone();
        dialog.connect_response(None, move |_, r| {
            if r != "save" {
                return;
            }
            let apps: Vec<String> = entry.text().split(',').map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()).collect();
            a.edit(|m| m.profile_mut().apps = apps);
        });
        dialog.present(Some(&self.window));
    }

    // ───────────────────────────── dialogs ─────────────────────────────

    fn ask_name(&self, title: &str, initial: &str, done: impl Fn(String) + 'static) {
        let dialog = adw::AlertDialog::new(Some(title), None);
        let entry = gtk::Entry::builder().text(initial).activates_default(true).build();
        dialog.set_extra_child(Some(&entry));
        dialog.add_responses(&[("cancel", "Cancel"), ("ok", "OK")]);
        dialog.set_response_appearance("ok", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("ok"));
        dialog.connect_response(None, move |_, r| {
            if r == "ok" {
                done(entry.text().trim().to_owned());
            }
        });
        dialog.present(Some(&self.window));
    }

    fn confirm(&self, title: &str, body: &str, yes: impl Fn() + 'static) {
        let dialog = adw::AlertDialog::new(Some(title), Some(body));
        dialog.add_responses(&[("cancel", "Cancel"), ("delete", "Delete")]);
        dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
        dialog.connect_response(None, move |_, r| {
            if r == "delete" {
                yes();
            }
        });
        dialog.present(Some(&self.window));
    }
}

/// Window classes of open windows (Hyprland), for the profile-apps suggestions.
fn open_window_classes() -> Vec<String> {
    let Ok(out) = std::process::Command::new("hyprctl").args(["clients", "-j"]).output() else { return Vec::new() };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else { return Vec::new() };
    let mut classes: Vec<String> = v
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| c.get("class")?.as_str().map(str::to_owned))
        .filter(|c| !c.is_empty() && c != masterhub_core::dbus::GUI_APP_ID)
        .collect();
    classes.sort();
    classes.dedup();
    classes
}
