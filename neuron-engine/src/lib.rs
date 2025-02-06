//! # Neuron Engine

pub mod os;

#[cfg(target_os="linux")]
pub extern crate x11_dl;

#[cfg(windows)]
pub extern crate windows;

use std::sync::{Arc, Weak};
use crate::os::{new_platform, OsLoopInputs, Platform};
use crate::os::window::{Window, WindowAttributes, WindowId, WindowManager};

pub struct Engine {
    platform: Arc<dyn Platform>,
    window_manager: Arc<WindowManager>,
}

impl Engine {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            platform: new_platform()?,
            window_manager: Arc::new(WindowManager::new()),
        })
    }

    pub fn platform(&self) -> &Arc<dyn Platform> {
        &self.platform
    }

    pub fn window_manager(&self) -> &Arc<WindowManager> {
        &self.window_manager
    }

    pub fn create_window(&self, window_attributes: WindowAttributes) -> anyhow::Result<(WindowId, Weak<dyn Window>)> {
        self.window_manager.create_window(window_attributes, &self.platform)
    }

    pub fn process_events(&self) {
        self.platform.process_events(&OsLoopInputs {
            window_manager: self.window_manager.clone(),
        })
    }

}