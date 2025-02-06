use std::ffi::{c_ulong, CStr, CString};
use crate::os::window::{Resolution, Window, WindowAttributes, WindowId};
use crate::os::x11::X11Platform;
use raw_window_handle::{
    HandleError, HasWindowHandle, RawWindowHandle, WindowHandle, XlibWindowHandle,
};
use std::mem::MaybeUninit;
use std::sync::Arc;
use x11_dl::xlib;
use x11_dl::xlib::{
    ButtonMotionMask, ButtonPressMask, ButtonReleaseMask, CWEventMask, ColormapChangeMask,
    EnterWindowMask, ExposureMask, FocusChangeMask, InputOutput, KeyPressMask, KeyReleaseMask,
    KeymapStateMask, LeaveWindowMask, OwnerGrabButtonMask, PMaxSize, PMinSize, PPosition, PSize,
    PointerMotionMask, PropertyChangeMask, StructureNotifyMask, SubstructureNotifyMask,
    VisibilityChangeMask, XSetWindowAttributes, XSizeHints,
};

pub(super) struct X11Window {
    pub(super) window: xlib::Window,
    id: WindowId,
    visual_id: u64,
    platform: Arc<X11Platform>,
}

impl HasWindowHandle for X11Window {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        unsafe {
            let mut wh = XlibWindowHandle::new(self.window);
            wh.visual_id = self.visual_id;

            Ok(WindowHandle::borrow_raw(RawWindowHandle::Xlib(wh)))
        }
    }
}

impl Window for X11Window {}

impl X11Window {
    pub(super) fn new(
        platform: Arc<X11Platform>,
        window_attributes: WindowAttributes,
        id: WindowId,
    ) -> anyhow::Result<Self> {
        unsafe {
            #[allow(invalid_value)]
            let mut swa = MaybeUninit::<XSetWindowAttributes>::uninit().assume_init();
            swa.event_mask = KeyPressMask
                | KeyReleaseMask
                | ButtonPressMask
                | ButtonReleaseMask
                | EnterWindowMask
                | LeaveWindowMask
                | PointerMotionMask
                | ButtonMotionMask
                | KeymapStateMask
                | ExposureMask
                | VisibilityChangeMask
                | StructureNotifyMask
                | SubstructureNotifyMask
                | FocusChangeMask
                | PropertyChangeMask
                | ColormapChangeMask
                | OwnerGrabButtonMask;

            let cw_mask = CWEventMask;

            let (x, y) = window_attributes.position.map_or((0, 0), |p| (p.x, p.y));

            let (width, height) = window_attributes.size.map_or((800, 600), |s| match s {
                Resolution::Physical { width, height } | Resolution::Logical { width, height } => {
                    (width, height)
                }
            });

            let depth = (platform.xlib.XDefaultDepth)(platform.display, platform.default_screen);

            let visual = (platform.xlib.XDefaultVisual)(platform.display, platform.default_screen);

            let window = (platform.xlib.XCreateWindow)(
                platform.display,
                platform.root_window,
                x,
                y,
                width,
                height,
                0,
                depth,
                InputOutput as u32,
                visual,
                cw_mask,
                &mut swa,
            );

            let title = CString::new(window_attributes.title.unwrap_or_else(|| "Window".to_string()))?;

            (platform.xlib.XStoreName)(platform.display, window, title.as_ptr());

            #[allow(invalid_value)]
            let mut size_hints = MaybeUninit::<XSizeHints>::uninit().assume_init();
            size_hints.flags = PSize | PPosition;
            size_hints.x = x;
            size_hints.y = x;
            size_hints.width = width as i32;
            size_hints.height = height as i32;

            if window_attributes.resizable {
                size_hints.flags |= PMinSize | PMaxSize;
                size_hints.min_width = width as i32;
                size_hints.max_width = width as i32;
                size_hints.min_height = height as i32;
                size_hints.max_height = height as i32;
            }

            (platform.xlib.XSetWMNormalHints)(platform.display, window, &mut size_hints);

            if window_attributes.initially_visible {
                (platform.xlib.XMapWindow)(platform.display, window);
            }

            let visual_id = (platform.xlib.XVisualIDFromVisual)(visual);

            let mut protocols = [platform.xa_wm_delete_window];

            (platform.xlib.XSetWMProtocols)(platform.display, window, protocols.as_ptr() as *mut c_ulong, 1);

            Ok(Self {
                window,
                id,
                visual_id,
                platform,
            })
        }
    }
}

impl Drop for X11Window {
    fn drop(&mut self) {
        self.platform.notify_window_destroy(self.window);
    }
}