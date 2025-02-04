use log::info;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let platform = neuron_engine::os::new_platform()?;

    info!("Platform:  {}", platform.name());
    info!("Headless:  {:?}", platform.is_headless());
    info!("Dark Mode: {:?}", platform.is_dark_mode());
    info!(
        "Supported Window Attributes: {:?}",
        platform.supported_window_attributes()
    );

    Ok(())
}
