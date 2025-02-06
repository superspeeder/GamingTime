#![cfg(windows)]

mod window;

use crate::ExitState;
use crate::os::window::{SupportedWindowAttributes, Window, WindowAttributes, WindowId};
use crate::os::windows::window::{WindowReferenceBlock, WindowsWindow};
use crate::os::{OsLoopInputs, Platform, PlatformKind};
use hashbrown::{HashMap, HashSet};
use log::debug;
use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle};
use std::cell::RefCell;
use std::hash::Hash;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Weak};
use widestring::{U16CStr, U16CString};
use windows::UI::ViewManagement::{UIColorType, UISettings};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{CreateSolidBrush, DeleteObject, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::{CREATESTRUCTW, CS_DROPSHADOW, CS_HREDRAW, CS_NOCLOSE, CS_VREDRAW, DefWindowProcW, DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, RegisterClassExW, TranslateMessage, UnregisterClassW, WM_CREATE, WM_QUIT, WNDCLASS_STYLES, WNDCLASSEXW, WS_OVERLAPPEDWINDOW, GWLP_USERDATA, SetWindowLongPtrW, GetWindowLongPtrW, WM_DESTROY};
use windows::core::PCWSTR;

pub(super) struct WindowsPlatform {
    hinstance: HINSTANCE,
    window_class_counter: AtomicU32,
    dark_mode: bool,
    window_background_brush: HBRUSH,
    registered_window_classes: RefCell<HashMap<WindowClassAttributes, U16CString>>,
    weak: Weak<Self>,
}

#[inline]
fn is_color_light(color: windows::UI::Color) -> bool {
    ((5 * color.G as u32) + (2 * color.R as u32) + color.B as u32) > (8 * 128)
}

fn is_dark_mode_internal() -> bool {
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

impl WindowsPlatform {
    pub(super) fn new(weak: Weak<WindowsPlatform>) -> anyhow::Result<Self> {
        let hinstance = HINSTANCE(unsafe { GetModuleHandleW(PCWSTR::null()) }?.0);

        unsafe {
            _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }

        let dark_mode = is_dark_mode_internal();

        let window_background_color =
            unsafe { UISettings::new()?.GetColorValue(UIColorType::Background)? };

        debug!(
            "Default Window Background Color: rgb({:02x},{:02x},{:02x})",
            window_background_color.R, window_background_color.G, window_background_color.B
        );

        let window_background_brush = unsafe {
            CreateSolidBrush(make_colorref(
                window_background_color.R,
                window_background_color.G,
                window_background_color.B,
            ))
        };

        Ok(Self {
            hinstance,
            window_class_counter: AtomicU32::new(0),
            dark_mode,
            window_background_brush,
            registered_window_classes: RefCell::new(HashMap::new()),
            weak,
        })
    }

    fn get_window_class(&self, attributes: WindowClassAttributes) -> anyhow::Result<U16CString> {
        if let Some(name) = self
            .registered_window_classes
            .borrow()
            .get(&attributes)
            .cloned()
        {
            Ok(name)
        } else {
            let name = U16CString::from_str(format!(
                "neuron_windowclass_{:?}",
                self.window_class_counter.fetch_add(1, Ordering::SeqCst)
            ))?;
            let mut wc = WNDCLASSEXW::default();
            wc.cbSize = size_of::<WNDCLASSEXW>() as u32;
            wc.hbrBackground = self.window_background_brush;
            wc.lpfnWndProc = Some(generic_window_proc);
            wc.lpszClassName = PCWSTR(name.as_ptr());
            wc.style = attributes.style();
            wc.hInstance = self.hinstance;

            unsafe {
                _ = RegisterClassExW(&wc);
            }

            self.registered_window_classes
                .borrow_mut()
                .insert(attributes, name.clone());

            Ok(name)
        }
    }
}

impl HasDisplayHandle for WindowsPlatform {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::windows())
    }
}

#[inline]
fn make_colorref(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF(((b as u32) << 16) | ((g as u32) << 8) | (r as u32))
}

impl Platform for WindowsPlatform {
    fn name(&self) -> &'static str {
        super::names::WINDOWS
    }

    fn kind(&self) -> PlatformKind {
        PlatformKind::Windows
    }

    fn is_headless(&self) -> bool {
        false
    }

    fn is_dark_mode(&self) -> Option<bool> {
        Some(self.dark_mode)
    }

    fn supported_window_attributes(&self) -> &'static SupportedWindowAttributes {
        &SupportedWindowAttributes {
            title: true,
            size: true,
            position: true,
            has_close_button: true,
            has_minimize_button: true,
            has_maximize_button: true,
            show_drop_shadow: true,
            show_border: true,
            show_title_bar: true,
            initially_disabled: true,
            is_dialog_box: true,
            initially_minimized: true,
            resizable: true,
            has_system_menu: true,
            initially_visible: true,
        }
    }

    fn create_window(
        &self,
        window_attributes: WindowAttributes,
        window_id: WindowId,
    ) -> anyhow::Result<Arc<dyn Window>> {
        Ok(Arc::new(WindowsWindow::new(
            self.weak.upgrade().unwrap(),
            window_attributes,
            window_id,
        )?))
    }

    fn process_events(&self, inputs: &OsLoopInputs) {
        unsafe {
            #[allow(invalid_value)]
            let mut msg = MaybeUninit::<MSG>::uninit().assume_init();

            while PeekMessageW(&mut msg, HWND::default(), 0, 0, PM_REMOVE).0 > 0 {
                if msg.message == WM_QUIT {
                    inputs.exit_manager.set(ExitState::ExitSuccess);
                }

                _ = DispatchMessageW(&msg);
                _ = TranslateMessage(&msg);
            }
        }
    }
}

impl Drop for WindowsPlatform {
    fn drop(&mut self) {
        for (_, string) in self.registered_window_classes.borrow().iter() {
            unsafe {
                _ = UnregisterClassW(PCWSTR(string.as_ptr()), self.hinstance);
            }
        }

        unsafe {
            _ = DeleteObject(self.window_background_brush);
        }
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
struct WindowClassAttributes {
    allow_close: bool,
    show_drop_shadow: bool,
}

impl WindowClassAttributes {
    fn style(&self) -> WNDCLASS_STYLES {
        let mut style: WNDCLASS_STYLES = CS_HREDRAW | CS_VREDRAW;
        if !self.allow_close {
            style |= CS_NOCLOSE;
        }

        if self.show_drop_shadow {
            style |= CS_DROPSHADOW;
        }
        style
    }
}

unsafe extern "system" fn generic_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let wptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
    if wptr != 0 {
        let reference_block = wptr as *const WindowReferenceBlock;
        if let Some(block) = reference_block.as_ref() {
            match message {
                WM_DESTROY => {
                    todo!("Find a way to pass the OsLoopInputs data to this function from the processing function. Not sure how just yet but will find a way (maybe setting it at the start of each loop on every living window's reference block).");
                },
                _ => ()
            }


        }
    }

    match message {
        WM_CREATE => {
            let cs = lparam.0 as *const CREATESTRUCTW;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*cs).lpCreateParams as isize);

            return LRESULT(0);
        }
        _ => ()
    }

    DefWindowProcW(hwnd, message, wparam, lparam)
}
