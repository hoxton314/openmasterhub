//! Follows the focused window on KDE Plasma (Wayland or X11).
//!
//! Wayland apps cannot see other windows, but KWin can. A small KWin script
//! (`openmasterhub-focus`) calls `ActiveWindow(class)` on this service every time the focus
//! changes. The class is passed to the controller exactly like the Hyprland backend does.

use std::sync::mpsc::Sender;
use std::thread;

use crate::controller::Msg;

const BUS_NAME: &str = "org.openmasterhub.Focus";
const OBJECT_PATH: &str = "/org/openmasterhub/Focus";

struct Focus {
    controller: Sender<Msg>,
}

#[zbus::interface(name = "org.openmasterhub.Focus1")]
impl Focus {
    /// Called by the KWin script. An empty class means no window is focused.
    fn active_window(&self, class: &str) {
        log::debug!("kde focus: {class:?}");
        let _ = self.controller.send(Msg::ActiveApp(class.to_owned()));
    }
}

pub fn spawn(controller: Sender<Msg>) {
    thread::Builder::new()
        .name("kde-focus".into())
        .spawn(move || match serve(controller) {
            Ok(_connection) => {
                log::info!("listening for KWin focus reports on {BUS_NAME}");
                // The connection serves requests on its own threads; keep it alive.
                loop {
                    thread::park();
                }
            }
            Err(e) => log::warn!("KDE focus service unavailable: {e}"),
        })
        .expect("spawn kde focus thread");
}

fn serve(controller: Sender<Msg>) -> zbus::Result<zbus::blocking::Connection> {
    zbus::blocking::connection::Builder::session()?
        .name(BUS_NAME)?
        .serve_at(OBJECT_PATH, Focus { controller })?
        .build()
}
