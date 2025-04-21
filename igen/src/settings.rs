use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
pub struct GoogleConfig {
    pub token_path: String,
    pub client_id: String,
    pub client_secret: String,
    pub auth_uri: String,
    pub redirect_uri: String,
    pub token_uri: String,
    pub calendar_list: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct QuoteConfig {
    pub quotes_path: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub google: GoogleConfig,
    pub quote: QuoteConfig,
}
