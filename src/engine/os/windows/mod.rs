#![cfg(target_os = "windows")]

use std::alloc::Layout;
use std::ffi::c_void;
use std::sync;
use std::sync::Arc;
use widestring::{U16CString, U16Str};
use crate::engine::os::{Platform, WindowAttributes};
use windows::UI::ViewManagement::{UIColorType, UISettings};
use windows::Win32::Foundation::{BOOL, HINSTANCE, HWND, LPARAM, LRESULT, TRUE, WPARAM};
use windows::Win32::Graphics::Gdi::HBRUSH;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::core::PCWSTR;
use windows::Win32;
use windows::Win32::Graphics::Dwm::DWMWA_USE_IMMERSIVE_DARK_MODE;
use windows::Win32::UI::WindowsAndMessaging::{CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, SetWindowLongPtrW, ShowWindow, CREATESTRUCTW, CS_HREDRAW, CS_NOCLOSE, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, HMENU, SW_NORMAL, WM_CREATE, WNDCLASSEXW, WS_EX_OVERLAPPEDWINDOW, WS_OVERLAPPEDWINDOW};
use crate::engine::Engine;
use crate::engine::os::window::WindowRef;

#[inline]
fn is_color_light(color: windows::UI::Color) -> bool {
    ((5 * color.G as u32) + (2 * color.R as u32) + color.B as u32) > (8 * 128)
}

pub struct WindowsPlatform {
    hinstance: HINSTANCE,
    hbrush_background: HBRUSH,
}

impl WindowsPlatform {
    fn get_hinstance() -> anyhow::Result<HINSTANCE> {
        Ok(HINSTANCE(
            unsafe { GetModuleHandleW(PCWSTR(std::ptr::null())) }?.0,
        ))
    }

    fn is_dark_mode_impl() -> bool {
        let settings = UISettings::new();
        if let Ok(settings) = settings {
            if let Ok(color) = settings.GetColorValue(UIColorType::Foreground) {
                is_color_light(color)
            } else {
                false
            }
        } else {
            false
        }
    }
}

pub(self) struct WindowsWindow {
    handle: HWND,
    window_class: U16CString,
    engine: sync::Weak<Engine>,
    id: u32,
    internal_ref: *mut WindowRef,
}

impl Platform for WindowsPlatform {
    type WindowType = WindowsWindow;

    fn is_dark_mode(&self) -> bool {
        WindowsPlatform::is_dark_mode_impl() // needed for other stuff
    }

    fn create_window(&self, engine: &Arc<Engine>, window_attributes: WindowAttributes, window_id: u32) -> anyhow::Result<()> {
        WindowsWindow::new(engine.clone(), window_attributes, window_id)
    }
}

impl Drop for WindowsWindow {
    fn drop(&mut self) {
        unsafe {
            std::ptr::drop_in_place(self.internal_ref);

            _ = DestroyWindow(self.handle);
        }
    }
}

trait WindowAttributesExtWindows {
    fn generate_window_class_attributes(&self) -> WNDCLASSEXW;
}

impl WindowAttributesExtWindows for WindowAttributes {
    fn generate_window_class_attributes(&self) -> WNDCLASSEXW {
        let mut style = CS_VREDRAW | CS_HREDRAW;

        if self.no_close_button {
            style |= CS_NOCLOSE;
        }

        WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style,
            lpfnWndProc: Some(generic_window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: Default::default(),
            hIcon: Default::default(),
            hCursor: Default::default(),
            hbrBackground: Default::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR::null(),
            hIconSm: Default::default(),
        }
    }
}

pub(self) struct WindowRef {
    engine: sync::Weak<Engine>,
    window_id: u32,
}


impl WindowsWindow {
    pub fn new(engine: Arc<Engine>, window_attributes: WindowAttributes, id: u32) -> anyhow::Result<Arc<Self>> {
        let window_class = unsafe {
            engine
                .borrow_mut()
                .register_window_class(window_attributes.generate_window_class_attributes())
        };

        let mut ex_style = WS_EX_OVERLAPPEDWINDOW;
        let mut style = WS_OVERLAPPEDWINDOW;

        let (x, y) = window_attributes
            .position
            .map(|pos| (pos.x, pos.y))
            .unwrap_or((CW_USEDEFAULT, CW_USEDEFAULT));
        let (w, h) = window_attributes
            .size
            .map(|pos| (pos.x, pos.y))
            .unwrap_or((CW_USEDEFAULT, CW_USEDEFAULT));

        // we super break the rules here because we need a pointer to some data that we can give to CreateWindowExW so we can link the window procedure calls back to the window and engine
        let internal_ref: *mut WindowRef =
            unsafe { std::alloc::alloc(Layout::new::<WindowRef>()) as *mut WindowRef };

        unsafe {
            std::ptr::write(internal_ref, WindowRef {
                engine: Arc::downgrade(&engine),
                window_id: id,
            });
        }

        let title = U16CString::from_str(window_attributes.title.as_str())?;

        let handle = unsafe {
            CreateWindowExW(
                ex_style,
                PCWSTR(window_class.as_ptr()),
                PCWSTR(title.as_ptr()),
                style,
                x,
                y,
                w,
                h,
                HWND::default(),
                HMENU::default(),
                HINSTANCE::default(),
                Some(internal_ref as *const c_void),
            )
        }?;

        if WindowsPlatform::is_dark_mode_impl() {
            let mode: BOOL = TRUE;
            unsafe {
                _ = Win32::Graphics::Dwm::DwmSetWindowAttribute(
                    handle,
                    DWMWA_USE_IMMERSIVE_DARK_MODE,
                    (&mode as *const BOOL) as *const c_void,
                    size_of::<BOOL>() as u32,
                );
            }
        }

        unsafe {
            _ = ShowWindow(handle, SW_NORMAL);
        }

        let window = Arc::new(Self {
            handle,
            window_class,
            engine: Arc::downgrade(&engine),
            id,
            internal_ref,
        });

        Ok(window)
    }

    pub fn window_class(&self) -> &U16CString {
        &self.window_class
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}

pub unsafe extern "system" fn generic_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let user_data = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
    if user_data != 0 {
        let window_ref: *mut WindowRef = user_data as *mut WindowRef;
        let result = window_ref
            .as_ref()
            .unwrap()
            .engine
            .upgrade()
            .unwrap()
            .recv_window_event((*window_ref).window_id, hwnd, msg, wparam, lparam);
        if let Some(result_value) = result {
            return LRESULT(result_value);
        }
    }

    match msg {
        WM_CREATE => {
            let createstruct: *const CREATESTRUCTW = lparam.0 as *const CREATESTRUCTW;
            if !createstruct.is_null() {
                if !(*createstruct).lpCreateParams.is_null() {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*createstruct).lpCreateParams as isize);
                }
            }
        }
        _ => return DefWindowProcW(hwnd, msg, wparam, lparam),
    }

    LRESULT(0)
}
