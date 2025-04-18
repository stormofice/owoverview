#![allow(dead_code)]
#![allow(clippy::needless_range_loop)]
use crate::dash::Dash;

mod calendar;
mod dash;
mod epd;
mod fonts;
mod graphics;
mod settings;

fn main() {
    let raw_config = config::Config::builder()
        .add_source(config::File::with_name("config.toml").required(true))
        .build()
        .expect("Could not load config");
    let config: settings::Config = raw_config
        .try_deserialize()
        .expect("Could not deserialize settings");
    println!("{:?}", config);

    let mut dash = Dash::new(config);

    dash.draw();
}
