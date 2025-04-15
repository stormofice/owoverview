use crate::dash::Dash;
use std::time::Duration;

mod calendar;
mod dash;
mod epd;
mod graphics;

fn main() {
    let mut dash = Dash::new();

    dash.draw();
}
