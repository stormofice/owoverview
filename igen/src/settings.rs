use serde::Deserialize;

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
pub struct ImageConfig {
    pub images_path: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct WeatherConfig {
    pub latitude: String,
    pub longitude: String,
    pub timezone: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GeneralConfig {
    pub debug: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub general: GeneralConfig,
    pub google: GoogleConfig,
    pub quote: QuoteConfig,
    pub image: ImageConfig,
    pub weather: WeatherConfig,
}
