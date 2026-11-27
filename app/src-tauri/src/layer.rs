//! wlr layer-shell for the switch toast on Wayland (`libgtk-layer-shell`, loaded at run time).
//!
//! A normal toplevel cannot choose its place or stay on top on Wayland, so compositors treat the
//! toast as an ordinary window (tiling it, blurring it, drawing borders). As a layer-shell
//! surface it sits on the overlay layer, anchored bottom-right, never takes keyboard focus and
//! reserves no space. The library is optional: when it, or the compositor's protocol, is
//! missing, `init` fails and the toast is simply not shown.

use std::ffi::{CString, c_char, c_int, c_void};

use gtk::glib::translate::ToGlibPtr;
use gtk::prelude::*;
use libloading::{Library, Symbol};

type Win = *mut c_void;

const LAYER_OVERLAY: c_int = 3;
const EDGE_RIGHT: c_int = 1;
const EDGE_BOTTOM: c_int = 3;
const KEYBOARD_NONE: c_int = 0;

/// Turn the unmapped `window` into an overlay-layer surface at the bottom-right corner.
pub fn init(window: &gtk::ApplicationWindow, margin: i32) -> Result<(), String> {
    // SAFETY: loading a plain C library whose initialisers only register GObject types; the
    // symbols are used with the signatures of gtk-layer-shell.h and the library is never unloaded.
    unsafe {
        let lib = Library::new("libgtk-layer-shell.so.0").map_err(|e| format!("libgtk-layer-shell is not installed: {e}"))?;
        let lib: &'static Library = Box::leak(Box::new(lib));
        let sym = |name: &str| -> Result<*const c_void, String> {
            let s: Symbol<*const c_void> = lib.get(name.as_bytes()).map_err(|e| format!("{name}: {e}"))?;
            Ok(*s)
        };
        let is_supported: extern "C" fn() -> c_int = std::mem::transmute(sym("gtk_layer_is_supported\0")?);
        if is_supported() == 0 {
            return Err("the compositor does not offer wlr-layer-shell".to_owned());
        }
        if window.is_realized() {
            return Err("the window was already mapped".to_owned());
        }
        let init_for_window: extern "C" fn(Win) = std::mem::transmute(sym("gtk_layer_init_for_window\0")?);
        let set_layer: extern "C" fn(Win, c_int) = std::mem::transmute(sym("gtk_layer_set_layer\0")?);
        let set_anchor: extern "C" fn(Win, c_int, c_int) = std::mem::transmute(sym("gtk_layer_set_anchor\0")?);
        let set_margin: extern "C" fn(Win, c_int, c_int) = std::mem::transmute(sym("gtk_layer_set_margin\0")?);
        let set_keyboard_mode: extern "C" fn(Win, c_int) = std::mem::transmute(sym("gtk_layer_set_keyboard_mode\0")?);
        let set_exclusive_zone: extern "C" fn(Win, c_int) = std::mem::transmute(sym("gtk_layer_set_exclusive_zone\0")?);
        let set_namespace: extern "C" fn(Win, *const c_char) = std::mem::transmute(sym("gtk_layer_set_namespace\0")?);

        let gw: &gtk::Window = window.upcast_ref();
        let stash: gtk::glib::translate::Stash<*mut gtk::ffi::GtkWindow, gtk::Window> = gw.to_glib_none();
        let ptr: Win = stash.0 as Win;
        init_for_window(ptr);
        set_layer(ptr, LAYER_OVERLAY);
        // Anchored to two edges and without a size of its own, the surface is as big as the toast.
        for edge in [EDGE_RIGHT, EDGE_BOTTOM] {
            set_anchor(ptr, edge, 1);
            set_margin(ptr, edge, margin);
        }
        set_keyboard_mode(ptr, KEYBOARD_NONE);
        set_exclusive_zone(ptr, 0);
        let ns = CString::new("openmasterhub-switch").expect("no NUL");
        set_namespace(ptr, ns.as_ptr());
    }
    Ok(())
}
