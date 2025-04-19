use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GoogleConfig {
    pub token_path: String,
    pub client_id: String,
    pub client_secret: String,
    pub auth_uri: String,
    pub redirect_uri: String,
    pub token_uri: String,
    pub calendar_list: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub google: GoogleConfig,
}
