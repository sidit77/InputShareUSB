#![windows_subsystem = "windows"]

use anyhow::Result;
use log::LevelFilter;

fn main() -> Result<()>{
    error_tools::gui::set_gui_panic_hook();

    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .format_timestamp(None)
        //.format_target(false)
        .parse_default_env()
        .init();

    log::info!("hello world");
    Ok(())
}