//! OpenMasterHUB: desktop app for the Cooler Master MasterHUB (GTK4 + libadwaita).

mod app;
mod canvas;
mod client;
mod inspector;
mod model;

use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;
use masterhub_core::dbus::GUI_APP_ID;

fn main() -> gtk::glib::ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let application = adw::Application::builder().application_id(GUI_APP_ID).build();
    application.connect_startup(|_| {
        let css = gtk::CssProvider::new();
        css.load_from_string(include_str!("style.css"));
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(&display, &css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        }
    });
    // Keep the window alive for the lifetime of the app.
    let window = std::rc::Rc::new(std::cell::RefCell::new(None));
    application.connect_activate(move |application| {
        if let Some(w) = application.active_window() {
            w.present();
            return;
        }
        match app::App::new(application) {
            Ok(a) => *window.borrow_mut() = Some(a),
            Err(e) => {
                eprintln!("openmasterhub: {e:#}");
                application.quit();
            }
        }
    });
    application.run()
}
