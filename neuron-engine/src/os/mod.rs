//! Platform interface & platform specific code

pub mod window;

#[cfg(target_os="linux")]
mod x11;

#[cfg(windows)]
mod windows;

use crate::os::window::{SupportedWindowAttributes, Window, WindowAttributes, WindowId, WindowManager};
use raw_window_handle::HasDisplayHandle;
use std::sync::Arc;

/// Generic access to platform specific functions.
/// Also requires [`raw_window_handle::HasDisplayHandle`] to be implemented.
pub trait Platform: HasDisplayHandle {
    /// Get the name of the current platform
    ///
    /// # Common Names
    /// | OS               | Return Value       | Constant                                   |
    /// |------------------|--------------------|--------------------------------------------|
    /// | Windows          | `"windows"`          | [`neuron_engine::os::names::WINDOWS`]          |
    /// | Windows/Headless | `"windows/headless"` | [`neuron_engine::os::names::WINDOWS_HEADLESS`] |
    /// | Linux/X11        | `"linux-x11"`        | [`neuron_engine::os::names::LINUX_X11`]        |
    /// | Linux/Wayland    | `"linux-wayland"`    | [`neuron_engine::os::names::LINUX_WAYLAND`]    |
    /// | Linux/Headless   | `"linux-headless"`   | [`neuron_engine::os::names::LINUX_HEADLESS`]   |
    ///
    fn name(&self) -> &'static str;

    /// Get a more machine-nice identifier for the platform.
    /// On all standard platforms, this is not backed by a string. On non-standard platforms, this is required to be [`PlatformKind::Custom`], which relies on a `&'static str`.
    fn kind(&self) -> PlatformKind;

    /// Check if a platform is headless (does it support surfaces or are we going to only be able to render to offscreen targets).
    fn is_headless(&self) -> bool;

    /// Check if the system is in dark or light mode.
    ///
    /// This function should return None if the platform does not allow this to be configured.
    ///
    /// # Implementation Suggestions
    /// When implementing a platform, you might want to store this value (as this generally requires querying some system settings that might not be the most performant, and a tri-state bool isn't taking up that much space.
    fn is_dark_mode(&self) -> Option<bool>;

    /// Get information about which window attributes are actually supported on this system.
    fn supported_window_attributes(&self) -> &'static SupportedWindowAttributes;

    fn create_window(&self, window_attributes: WindowAttributes, window_id: WindowId) -> anyhow::Result<Arc<dyn Window>>;

    /// Process OS events (most operating systems have some sort of event polling loop that we have to run to actually handle those events, otherwise the window will stop responding).
    fn process_events(&self, inputs: &OsLoopInputs);
}

/// Identifier for platforms.
///
/// Non-standard platforms **must** use [`PlatformKind::Custom`].
#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
#[allow(missing_docs)]
pub enum PlatformKind {
    Windows,
    WindowsHeadless,
    LinuxX11,
    LinuxWayland,
    LinuxHeadless,
    Custom(&'static str),
}

/// Constants for standard platform names.
#[allow(missing_docs)]
pub mod names {

    pub const WINDOWS: &'static str = "windows";
    pub const LINUX_X11: &'static str = "linux-x11";
    pub const LINUX_WAYLAND: &'static str = "linux-wayland";
    pub const WINDOWS_HEADLESS: &'static str = "windows-headless";
    pub const LINUX_HEADLESS: &'static str = "linux-headless";
}


pub fn new_platform() -> anyhow::Result<Arc<dyn Platform>> {
    #[cfg(target_os="windows")]
    {
        Ok(Arc::new_cyclic(|weak| windows::WindowsPlatform::new(weak.clone()).expect("windows platform initialization failed")))
    }

    #[cfg(target_os="linux")]
    {
        Ok(Arc::new_cyclic(|weak| x11::X11Platform::new(weak.clone()).expect("X11 platform initialization failed")))
    }

    #[cfg(not(any(target_os="windows", target_os="linux")))]
    {
        unimplemented!("This platform is not supported")
    }
}

pub(super) struct OsLoopInputs {
    pub window_manager: Arc<WindowManager>,
}
