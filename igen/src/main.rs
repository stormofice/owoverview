#![allow(dead_code)]
#![allow(clippy::needless_range_loop)]

use log::debug;
use render::dash::Dash;

mod provider;
mod render;
mod settings;

fn main() {
    pretty_env_logger::formatted_timed_builder()
        .filter(Some("igen"), log::LevelFilter::Debug)
        .init();

    let raw_config = config::Config::builder()
        .add_source(config::File::with_name("config.toml").required(true))
        .build()
        .expect("Could not load config");
    let config: settings::Config = raw_config
        .try_deserialize()
        .expect("Could not deserialize settings");
    debug!("Config: {:?}", config);

    let mut dash = Dash::new(config);

    dash.draw();
}
