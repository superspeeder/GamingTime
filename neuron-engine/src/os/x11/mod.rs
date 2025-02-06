//! X11 interop

#![cfg(target_os = "linux")]

mod window;

use crate::os::window::{SupportedWindowAttributes, Window, WindowAttributes, WindowId};
use crate::os::x11::window::X11Window;
use crate::os::{OsLoopInputs, PlatformKind};
use anyhow::bail;
use hashbrown::HashMap;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, RawDisplayHandle, XlibDisplayHandle,
};
use std::cell::RefCell;
use std::ffi::{CStr, c_long, c_ulong, c_void};
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::sync::{Arc, Weak};
use log::debug;
use x11_dl::xlib;
use x11_dl::xlib::{XEvent, Xlib};

pub(super) struct X11Platform {
    pub(self) xlib: Xlib,
    pub(self) display: *mut xlib::Display,
    pub(self) default_screen: i32,
    pub(self) root_window: xlib::Window,
    pub(self) xa_wm_delete_window: xlib::Atom,
    pub(self) xa_wm_protocols: xlib::Atom,
    window_map: RefCell<HashMap<xlib::Window, WindowId>>,
    weak: Weak<X11Platform>,
}

impl X11Platform {
    pub fn new(weak: Weak<X11Platform>) -> anyhow::Result<X11Platform> {
        let xlib = Xlib::open()?;
        let display = unsafe { (xlib.XOpenDisplay)(std::ptr::null()) };

        if display.is_null() {
            bail!("Failed to connect to X server.");
        }

        let default_screen = unsafe { (xlib.XDefaultScreen)(display) };

        let root_window = unsafe { (xlib.XRootWindow)(display, default_screen) };

        let xa_wm_delete_window_name = CStr::from_bytes_with_nul(b"WM_DELETE_WINDOW\0")?;
        let xa_wm_protocols_name = CStr::from_bytes_with_nul(b"WM_PROTOCOLS\0")?;

        let xa_wm_delete_window =
            unsafe { (xlib.XInternAtom)(display, xa_wm_delete_window_name.as_ptr(), xlib::False) };
        let xa_wm_protocols =
            unsafe { (xlib.XInternAtom)(display, xa_wm_protocols_name.as_ptr(), xlib::False) };

        Ok(X11Platform {
            xlib,
            display,
            default_screen,
            root_window,
            xa_wm_delete_window,
            xa_wm_protocols,
            weak,
            window_map: RefCell::new(HashMap::new()),
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

    pub fn notify_window_destroy(&self, window: xlib::Window) {
        self.window_map.borrow_mut().remove(&window);
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

    fn create_window(
        &self,
        window_attributes: WindowAttributes,
        window_id: WindowId,
    ) -> anyhow::Result<Arc<dyn Window>> {
        let win = Arc::new(X11Window::new(
            self.weak.upgrade().unwrap(),
            window_attributes,
            window_id,
        )?);
        self.window_map.borrow_mut().insert(win.window, window_id);
        Ok(win)
    }

    fn process_events(&self, inputs: &OsLoopInputs) {
        #[allow(invalid_value)]
        let mut event = unsafe { MaybeUninit::<XEvent>::uninit().assume_init() };

        unsafe {
            while (self.xlib.XPending)(self.display) > 0 {
                (self.xlib.XNextEvent)(self.display, &mut event);

                match event.type_ {
                    xlib::ClientMessage => {
                        if event.client_message.message_type == self.xa_wm_protocols
                            && event.client_message.format == 32
                        {
                            if event.client_message.data.as_longs()[0]
                                == (self.xa_wm_delete_window as c_long)
                            {
                                if let Some(wid) = self.window_map.borrow().get(&event.any.window) {
                                    inputs.window_manager.begin_closing_window(wid.clone());
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }
        }

        inputs.window_manager.update();
    }
}
