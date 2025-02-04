use std::sync::Arc;
use cgmath::Vector2;
use log::info;
use widestring::U16CString;
use crate::engine::{ApplicationHandler, Engine};
use crate::engine::os::event::OsEventHandler;
use crate::engine::os::window::{Window, WindowAttributes};

mod engine;

pub struct App {

}

impl OsEventHandler for App {
    fn on_close_request(&mut self, window_id: u32, engine: &Arc<Engine>) -> bool {
        true
    }
}

impl ApplicationHandler for App {}


fn main() -> anyhow::Result<()> {
    env_logger::init();
    info!("is_system_dark_mode(): {:?}", engine::os::is_system_dark_mode());

    let engine = Engine::new(App{})?;

    let window_id = Window::new(engine.clone(), WindowAttributes {
        title: U16CString::from_str("Hello!")?,
        ..Default::default()
    })?;

    let window_id_2 = Window::new(engine.clone(), WindowAttributes {
        title: U16CString::from_str("Hello 2!")?,
        size: Some(Vector2::new(800, 600)),
        ..Default::default()
    })?;

    info!("window_id: {:?}", window_id);
    info!("window_id_2: {:?}", window_id_2);

    Engine::simple_message_loop(engine);

    Ok(())
}
