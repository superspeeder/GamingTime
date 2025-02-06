use log::info;
use neuron_engine::Engine;
use neuron_engine::os::window::WindowAttributes;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let engine = Engine::new()?;

    info!("Platform:  {}", engine.platform().name());
    info!("Headless:  {:?}", engine.platform().is_headless());
    info!("Dark Mode: {:?}", engine.platform().is_dark_mode());
    info!(
        "Supported Window Attributes: {:?}",
        engine.platform().supported_window_attributes()
    );

    let (window_id, window) = engine.create_window(WindowAttributes {
        title: Some("Hello!".to_string()),
        ..Default::default()
    })?;

    info!("Window ID: {:?}", window_id);

    while engine.window_manager().is_window_alive(window_id) {
        engine.process_events();
    }

    Ok(())
}
