use std::sync::Arc;
use cgmath::Vector2;
use widestring::U16CString;
use crate::engine::Engine;

pub mod window;
pub mod event;

#[cfg(target_os="windows")]
mod windows;

pub struct WindowAttributes {
    pub title: String,
    pub size: Option<Vector2<i32>>,
    pub position: Option<Vector2<i32>>,
    pub no_close_button: bool,
}

pub trait Window {

}

pub trait Platform {
    fn is_dark_mode(&self) -> bool;

    fn create_window(&self, engine: &Arc<Engine>, window_attributes: WindowAttributes, window_id: u32) -> anyhow::Result<()>;
}

impl Default for WindowAttributes {
    fn default() -> Self {
        Self {
            title: "Window".to_string(),
            size: None,
            position: None,
            no_close_button: false,
        }
    }
}

pub trait OsEventHandler {
    fn on_close_request(&mut self, window_id: u32, engine: &Arc<Engine>) -> bool;
}

