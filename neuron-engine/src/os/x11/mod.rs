//! X11 interop

#![cfg(target_os = "linux")]

use crate::os::PlatformKind;
use crate::os::window::SupportedWindowAttributes;
use anyhow::bail;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, RawDisplayHandle, XlibDisplayHandle,
};
use std::ffi::c_void;
use std::ptr::NonNull;
use x11_dl::xlib;
use x11_dl::xlib::Xlib;

pub(super) struct X11Platform {
    xlib: Xlib,
    display: *mut xlib::Display,
    default_screen: i32,
    root_window: xlib::Window,
}

impl X11Platform {
    pub fn new() -> anyhow::Result<X11Platform> {
        let xlib = Xlib::open()?;
        let display = unsafe { (xlib.XOpenDisplay)(std::ptr::null()) };

        if display.is_null() {
            bail!("Failed to connect to X server.");
        }

        let default_screen = unsafe { (xlib.XDefaultScreen)(display) };

        let root_window = unsafe { (xlib.XRootWindow)(display, default_screen) };

        Ok(X11Platform {
            xlib,
            display,
            default_screen,
            root_window,
        })
    }

    pub fn display(&self) -> *mut xlib::Display {
        self.display
    }

    pub fn default_screen(&self) -> i32 {
        self.default_screen
    }

    pub fn root_window(&self) -> xlib::Window {
        self.root_window
    }
}

impl Drop for X11Platform {
    fn drop(&mut self) {
        unsafe {
            (self.xlib.XCloseDisplay)(self.display);
        }
    }
}

impl HasDisplayHandle for X11Platform {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        unsafe {
            Ok(DisplayHandle::borrow_raw(RawDisplayHandle::Xlib(
                XlibDisplayHandle::new(
                    Some(NonNull::new_unchecked(self.display as *mut c_void)),
                    self.default_screen,
                ),
            )))
        }
    }
}

impl super::Platform for X11Platform {
    fn name(&self) -> &'static str {
        super::names::LINUX_X11
    }

    fn kind(&self) -> PlatformKind {
        PlatformKind::LinuxX11
    }

    fn is_headless(&self) -> bool {
        false
    }

    fn is_dark_mode(&self) -> Option<bool> {
        None
    }

    fn supported_window_attributes(&self) -> &'static SupportedWindowAttributes {
        &SupportedWindowAttributes {
            title: true,
            size: true,
            position: true,
            has_close_button: false,
            has_minimize_button: false,
            has_maximize_button: false,
            show_drop_shadow: false,
            show_border: false,
            show_title_bar: false,
            initially_disabled: false,
            is_dialog_box: false,
            initially_minimized: false,
            resizable: true,
            has_system_menu: false,
            initially_visible: true,
        }
    }
}
