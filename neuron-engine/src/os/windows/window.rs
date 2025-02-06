use crate::os::Platform;
use crate::os::window::{Resolution, Window, WindowAttributes, WindowId};
use crate::os::windows::{WindowClassAttributes, WindowsPlatform};
use raw_window_handle::{
    HandleError, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
};
use std::ffi::c_void;
use std::num::NonZeroIsize;
use std::sync::Arc;
use widestring::U16CString;
use windows::Win32::Foundation::{HINSTANCE, HWND, POINT, RECT};
use windows::Win32::UI::WindowsAndMessaging::{CW_USEDEFAULT, CreateWindowExW, HMENU, WINDOW_EX_STYLE, WINDOW_STYLE, WS_EX_OVERLAPPEDWINDOW, WS_MINIMIZEBOX, WS_OVERLAPPEDWINDOW, AdjustWindowRectEx};
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTOPRIMARY};
use windows::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForMonitor, MDT_EFFECTIVE_DPI};

pub(super) struct WindowsWindow {
    handle: HWND,
    id: WindowId,
    reference_block: Box<WindowReferenceBlock>,
}

pub(super) struct WindowReferenceBlock {
    pub id: WindowId,
    pub platform: Arc<WindowsPlatform>,
}

impl HasWindowHandle for WindowsWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        unsafe {
            Ok(WindowHandle::borrow_raw(RawWindowHandle::Win32(
                Win32WindowHandle::new(NonZeroIsize::new_unchecked(self.handle.0 as _)),
            )))
        }
    }
}

impl Window for WindowsWindow {}

fn r2s(res: Resolution<u32>, style: WINDOW_STYLE, ex_style: WINDOW_EX_STYLE, position: (i32, i32)) -> (i32, i32) {
    match res {
        Resolution::Physical {width, height } => {
            unsafe {
                let mut r = RECT {
                    top: 0,
                    left: 0,
                    bottom: height as i32,
                    right: width as i32,
                };

                _ = AdjustWindowRectEx(&mut r, style, false, ex_style);
                (r.right - r.left, r.bottom - r.top)
            }
        },
        Resolution::Logical {width, height} => {
            unsafe {
                let mut r = RECT {
                    top: 0,
                    left: 0,
                    bottom: height as i32,
                    right: width as i32,
                };

                let mut pt = POINT { x: position.0, y: position.1 };
                let monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTOPRIMARY);

                let mut dpix: u32 = 96;
                let mut dpiy: u32 = 0;

                _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpix, &mut dpiy);

                _ = AdjustWindowRectExForDpi(&mut r, style, false, ex_style, dpix);
                (r.right - r.left, r.bottom - r.top)
            }
        }
    }
}

impl WindowsWindow {
    pub(super) fn new(
        platform: Arc<WindowsPlatform>,
        window_attributes: WindowAttributes,
        id: WindowId,
    ) -> anyhow::Result<Self> {
        let wc = platform.get_window_class(WindowClassAttributes {
            allow_close: window_attributes.allow_close,
            show_drop_shadow: window_attributes.show_drop_shadow,
        })?;

        let title = U16CString::from_str(window_attributes.title.unwrap_or("Window".to_string()))?;

        let reference_block = Box::new(WindowReferenceBlock {
            id,
            platform: platform.clone(),
        });

        let mut ex_style = WINDOW_EX_STYLE::default();
        let mut style = WINDOW_STYLE::default();

        if window_attributes.has_minimize_button {
            style |= WS_MINIMIZEBOX;
        }

        if window_attributes.has_maximize_button {
            style |= WS_MINIMIZEBOX;
        }

        let (x, y) = window_attributes
            .position
            .map_or((CW_USEDEFAULT, CW_USEDEFAULT), |p| (p.x, p.y));

        let (width, height) = window_attributes
            .size
            .map_or((CW_USEDEFAULT, CW_USEDEFAULT), |s| { r2s(s, style, ex_style, (x, y))});

        let handle = unsafe {
            CreateWindowExW(
                ex_style,
                PCWSTR(wc.as_ptr()),
                PCWSTR(title.as_ptr()),
                style,
                x,
                y,
                width,
                height,
                HWND::default(),
                HMENU::default(),
                platform.hinstance,
                Some((&*reference_block as *const WindowReferenceBlock) as *const c_void),
            )?
        };

        Ok(Self {
            handle,
            id,
            reference_block,
        })
    }
}
