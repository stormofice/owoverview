#![allow(dead_code)]
#![allow(clippy::needless_range_loop)]

use crate::render::dash::RenderAction;
use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::response::Response;
use axum::routing::get;
use log::debug;
use render::dash::Dash;
use std::sync::Arc;
use tokio::sync::Mutex;

mod provider;
mod render;
mod settings;

#[derive(Clone)]
pub struct AppState {
    // ballin
    dash: Arc<Mutex<Dash>>,
}

#[tokio::main]
async fn main() {
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

    // well not a fun of awaiting a constructor
    let dash = Dash::new(config).await;

    let state = AppState {
        dash: Arc::new(Mutex::new(dash)),
    };

    let app = Router::new()
        .route("/", get(root))
        .route("/image", get(image))
        .route("/nice_image", get(nice_image))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:7676")
        .await
        .expect("Could not bind tcp listener");

    axum::serve(listener, app)
        .await
        .expect("Error while serving smh")
}

async fn root() -> &'static str {
    "🦕"
}

async fn image(State(state): State<AppState>) -> Response {
    // probability of correct concurrency: 40%
    let mut dash = { state.dash.lock().await };

    // yolo protocol

    let img_data = match dash.render(false).await {
        RenderAction::Full(data) => {
            let mut bv = 0x00u32.to_le_bytes().to_vec();
            bv.extend(data);
            bv
        }
        RenderAction::Partial(bbox, data) => {
            let mut bv = [
                0x01u32,
                bbox.x as u32,
                bbox.y as u32,
                bbox.width as u32,
                bbox.height as u32,
            ]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<u8>>();
            bv.extend(data);
            bv
        }
    };

    let bytes = Bytes::from(img_data);
    Response::builder()
        .header("Content-Length", bytes.len().to_string())
        .body(bytes.into())
        .unwrap()
}

async fn nice_image(State(state): State<AppState>) -> Response {
    match tokio::fs::read("output.png").await {
        Ok(img_data) => Response::builder()
            .header("Content-Type", "image/png")
            .body(Bytes::from(img_data).into())
            .unwrap(),
        Err(_) => Response::builder()
            .status(404)
            .body("uh uh".into())
            .unwrap(),
    }
}
