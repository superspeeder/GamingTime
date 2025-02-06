//! # Neuron Engine

pub mod os;

#[cfg(target_os = "linux")]
pub extern crate x11_dl;

#[cfg(windows)]
pub extern crate windows;

use crate::os::window::{Window, WindowAttributes, WindowId, WindowManager};
use crate::os::{OsLoopInputs, Platform, new_platform};
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock, Weak};

pub struct Engine {
    platform: Arc<dyn Platform>,
    window_manager: Arc<WindowManager>,
    exit_manager: Arc<ExitManager>,
}

impl Engine {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            platform: new_platform()?,
            window_manager: Arc::new(WindowManager::new()),
            exit_manager: Arc::new(ExitManager::new()),
        })
    }

    pub fn platform(&self) -> &Arc<dyn Platform> {
        &self.platform
    }

    pub fn window_manager(&self) -> &Arc<WindowManager> {
        &self.window_manager
    }

    pub fn create_window(
        &self,
        window_attributes: WindowAttributes,
    ) -> anyhow::Result<(WindowId, Weak<dyn Window>)> {
        self.window_manager
            .create_window(window_attributes, &self.platform)
    }

    pub fn process_events(&self) -> ExitState {
        self.platform.process_events(&OsLoopInputs {
            window_manager: self.window_manager.clone(),
            exit_manager: self.exit_manager.clone(),
        });

        self.window_manager.update();

        self.exit_manager.take_exit_state()
    }
}

#[derive(Default)]
pub enum ExitState {
    #[default]
    Running,
    ExitSuccess,
    ExitError(anyhow::Error),
    ExitErrorGeneric,
}

pub struct ExitManager {
    exit_state: RwLock<ExitState>,
}

impl ExitManager {
    fn new() -> ExitManager {
        Self {
            exit_state: RwLock::new(ExitState::Running),
        }
    }

    pub fn should_exit(&self) -> bool {
        let value = self.exit_state.read();
        if let Ok(value) = value {
            match &*value {
                &ExitState::Running => false,
                _ => true,
            }
        } else {
            false
        }
    }

    fn set(&self, value: ExitState) {
        let es = self.exit_state.write();
        if let Ok(mut es) = es {
            *es = value
        }
    }

    fn take_exit_state(&self) -> ExitState {
        let Ok(mut l) = self.exit_state.write() else {
            return ExitState::Running;
        };

        let mut es = match *l {
            ExitState::Running => ExitState::Running,
            ExitState::ExitSuccess => ExitState::ExitSuccess,
            ExitState::ExitError(_) => ExitState::ExitErrorGeneric,
            ExitState::ExitErrorGeneric => ExitState::ExitErrorGeneric,
        };

        std::mem::swap(&mut *l, &mut es);

        es
    }
}
