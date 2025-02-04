use std::sync::Arc;
use cgmath::Vector2;
use crate::engine::Engine;
use crate::engine::os::linux::X11Platform;

#[cfg(target_os="windows")]
mod windows;
mod linux;
mod event_handlers;
mod window;

pub struct WindowAttributes {
    pub title: String,
    pub size: Option<Vector2<i32>>,
    pub position: Option<Vector2<i32>>,
    pub no_close_button: bool,
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

#[cfg(not(any(target_os="windows", target_os="linux")))]
pub(super) fn create_platform() -> anyhow::Result<Arc<dyn Platform>> {
    unimplemented!()
}

#[cfg(target_os="linux")]
pub(super) fn create_platform() -> anyhow::Result<Arc<dyn Platform>> {
    Ok(Arc::new(X11Platform::new()?))
}