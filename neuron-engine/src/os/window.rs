//! Platform generic windows

use crate::os::Platform;
use hashbrown::{HashMap, HashSet};
use log::debug;
use raw_window_handle::HasWindowHandle;
use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Weak};

/// Generic access to a window.
/// Also requires [`raw_window_handle::HasWindowHandle`] to be implemented.
pub trait Window: HasWindowHandle {}

/// Set of attributes that control how a window is created.
///
/// > **Note:** Not all of these attributes are actually available on all platforms, however they are all defined.
/// > Furthermore, you can use the [Platform](super::Platform) object to query which attributes are actually available to be used. Setting unavailable attributes is never an error, however they will simply be ignored.
pub struct WindowAttributes {
    /// Window title (defaults to `"Window"`)
    pub title: Option<String>,

    /// Size of the window (default is platform-dependent)
    pub size: Option<Resolution<u32>>,

    /// Position of the window (default is platform-dependent)
    pub position: Option<WindowPosition>,

    /// Does the window have a close button?
    pub has_close_button: bool, // = true

    /// Does the window have a minimize button?
    pub has_minimize_button: bool, // = true

    /// Does the window have a maximize button?
    pub has_maximize_button: bool, // = true

    /// Does the window have a drop shadow (only windows)
    pub show_drop_shadow: bool, // = false

    /// Does the window have a border?
    pub show_border: bool, // = true

    /// Does the window have a title bar?
    pub show_title_bar: bool, // = true

    /// Is the window initially disabled?
    pub initially_disabled: bool, // = false

    /// Is the window a dialog box? (only windows)
    pub is_dialog_box: bool, // = false

    /// Is the window initially minimized?
    pub initially_minimized: bool, // = false

    /// Is the window resizable?
    pub resizable: bool, // = true

    /// Does the window have a menu bar? (windows only)
    pub has_system_menu: bool, // = false

    /// Is the window initially visible?
    pub initially_visible: bool, // = true
}

impl Default for WindowAttributes {
    fn default() -> Self {
        Self {
            title: None,
            size: None,
            position: None,
            has_close_button: true,
            has_minimize_button: true,
            has_maximize_button: true,
            show_drop_shadow: false,
            show_border: true,
            show_title_bar: true,
            initially_disabled: false,
            is_dialog_box: false,
            initially_minimized: false,
            resizable: true,
            has_system_menu: false,
            initially_visible: true,
        }
    }
}

/// Representation of resolutions on systems. Supports both physical resolutions (exact pixels) and logical resolutions (based on dpi).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Resolution<T> {
    /// Physical resolution (based on pixels).
    #[allow(missing_docs)]
    Physical { width: T, height: T },

    /// Logical resolution (based on dpi).
    #[allow(missing_docs)]
    Logical { width: T, height: T },
}

impl<T> Copy for Resolution<T> where T: Copy + Clone {}

/// Window position
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub struct WindowPosition {
    x: i32,
    y: i32,
}

/// Information about which window attributes are available
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct SupportedWindowAttributes {
    pub title: bool,
    pub size: bool,
    pub position: bool,
    pub has_close_button: bool,
    pub has_minimize_button: bool,
    pub has_maximize_button: bool,
    pub show_drop_shadow: bool,
    pub show_border: bool,
    pub show_title_bar: bool,
    pub initially_disabled: bool,
    pub is_dialog_box: bool,
    pub initially_minimized: bool,
    pub resizable: bool,
    pub has_system_menu: bool,
    pub initially_visible: bool,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
#[repr(transparent)]
pub struct WindowId(u32);

// TODO: restructure this so that some of the stuff here can be Send (i.e. create an async and multithreaded capable window management system which maintains the common os requirement of the main thread being the only one able to validly interact with the actual os calls).

pub struct WindowManager {
    window_id_counter: AtomicU32,
    window_sets: RefCell<WindowSets>, // interior mutability
}

struct WindowSets {
    windows: HashMap<WindowId, Arc<dyn Window>>,
    active_windows: HashSet<WindowId>,
    dying_windows: HashSet<WindowId>,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            window_id_counter: AtomicU32::new(0),
            window_sets: RefCell::new(WindowSets {
                windows: HashMap::new(),
                active_windows: HashSet::new(),
                dying_windows: HashSet::new(),
            }),
        }
    }

    pub fn create_window(
        &self,
        window_attributes: WindowAttributes,
        platform: Arc<dyn Platform>,
    ) -> anyhow::Result<(WindowId, Weak<dyn Window>)> {
        let id = WindowId(self.window_id_counter.fetch_add(1, Ordering::SeqCst));

        let window = platform.create_window(window_attributes, id)?;

        let weakref = Arc::downgrade(&window);

        _ = self.window_sets.borrow_mut().active_windows.insert(id);
        _ = self.window_sets.borrow_mut().windows.insert(id, window);

        Ok((id, weakref))
    }

    pub fn begin_closing_window(&self, id: WindowId) {
        self.window_sets.borrow_mut().active_windows.remove(&id);
        self.window_sets.borrow_mut().dying_windows.insert(id);
    }

    pub fn try_finish_closing_window(&self, id: WindowId) -> bool {
        if self.window_sets.borrow().dying_windows.contains(&id) {
            if let Some(window) = self.window_sets.borrow().windows.get(&id) {
                if Arc::strong_count(&window) > 1 {
                    debug!(
                        "Cannot finish close window {:?}: There are still outside references to this window.",
                        id
                    );
                    return false;
                }
            } else {
                return true;
            }

            self.window_sets.borrow_mut().dying_windows.remove(&id);
            self.window_sets.borrow_mut().windows.remove(&id);
        }

        true
    }

    pub fn get_window(&self, id: WindowId) -> Option<Arc<dyn Window>> {
        if self.window_sets.borrow().active_windows.contains(&id) {
            self.window_sets.borrow().windows.get(&id).cloned()
        } else {
            None
        }
    }

    pub fn is_window_active(&self, id: WindowId) -> bool {
        self.window_sets.borrow().active_windows.contains(&id)
    }

    /// Is a window alive? (meaning: is a window either active or dying, but not dead)
    pub fn is_window_alive(&self, id: WindowId) -> bool {
        self.window_sets.borrow().active_windows.contains(&id)
            || self.window_sets.borrow().dying_windows.contains(&id)
    }

    pub fn is_window_dying(&self, id: WindowId) -> bool {
        self.window_sets.borrow().dying_windows.contains(&id)
    }
}
