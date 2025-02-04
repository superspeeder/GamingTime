#![cfg(target_os = "linux")]

use std::sync::Arc;
use anyhow::bail;
use crate::engine::os::{Platform, WindowAttributes};

use x11::xlib;
use crate::engine::os::window::WindowManager;

pub(super) struct X11Platform {
    display: *mut xlib::Display,
    screen: i32,
    root: xlib::Window,
}

impl X11Platform {
    pub(super) fn new() -> anyhow::Result<Self> {
        let display = unsafe { xlib::XOpenDisplay(std::ptr::null()) };
        if display.is_null() {
            bail!("Failed to open connection to X window server");
        }

        let screen = unsafe { xlib::XDefaultScreen(display) };
        let root = unsafe { xlib::XRootWindow(display, screen) };

        Ok(X11Platform { display, root, screen })
    }
}

struct X11Window {
    handle: xlib::Window,
    window_manager: Arc<WindowManager>
}

impl Platform for X11Platform {
    fn is_dark_mode(&self) -> bool {
        false
    }

    fn create_window(&self, engine: &Arc<Engine>, window_attributes: WindowAttributes, window_id: u32) -> anyhow::Result<()> {}
}

pub(self) trait X11EngineExt {
    fn
}