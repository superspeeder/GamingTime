use std::collections::HashMap;
use crate::engine::os::Window;

#[repr(transparent)]
#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct WindowId(u32);

pub struct WindowManager {
    active_windows: HashMap<WindowId, Arc<dyn Window>>
}

pub trait Window {
    fn close(&mut self);
    fn is_visible(&self) -> bool;

    fn show(&mut self);
    fn hide(&mut self);


    fn is_maximized(&self) -> bool;
    fn maximize(&mut self);

    fn is_minimized(&self) -> bool;
    fn minimize(&mut self);

    fn restore(&mut self);

    fn size(&self) -> ;
}
