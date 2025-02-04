use crate::engine::Engine;
use crate::engine::os::is_system_dark_mode;
use WindowsAndMessaging::WM_CREATE;
use cgmath::Vector2;
use std::alloc::Layout;
use std::cell::RefCell;
use std::ffi::c_void;
use std::pin::Pin;
use std::rc::Weak;
use std::sync;
use std::sync::Arc;
use widestring::U16CString;
use windows::Win32;
use windows::Win32::Foundation::{BOOL, HINSTANCE, HWND, LPARAM, LRESULT, TRUE, WPARAM};
use windows::Win32::Graphics::Dwm::DWMWA_USE_IMMERSIVE_DARK_MODE;
use windows::Win32::UI::WindowsAndMessaging;
use windows::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CS_HREDRAW, CS_NOCLOSE, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW,
    DefWindowProcW, DestroyWindow, GWLP_USERDATA, GetWindowLongPtrW, HMENU, SW_NORMAL,
    SetWindowLongPtrW, ShowWindow, UnregisterClassW, WNDCLASS_STYLES, WNDCLASSEXW,
    WS_EX_OVERLAPPEDWINDOW, WS_OVERLAPPEDWINDOW, WS_VSCROLL,
};
use windows::core::PCWSTR;

// Thing to remember: I can make a 'static lifetime wgpu::Surface and pass in an Arc<Window> to it. As a result, this *should* be fine, but I need to make sure that when I tell the system to destroy a window it destroys the surface object.

