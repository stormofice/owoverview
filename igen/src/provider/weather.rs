use crate::settings::Config;
use chrono::{NaiveDate, TimeDelta};
use serde::Deserialize;
use std::collections::HashMap;
use std::ops::Add;

#[derive(Deserialize, Debug)]
struct WeatherData {
    current: Current,
    daily: Daily,
}

#[derive(Deserialize, Debug)]
struct Current {
    #[serde(rename(deserialize = "temperature_2m"))]
    temperature: f64,
    #[serde(rename(deserialize = "relative_humidity_2m"))]
    humidity: f64,
    weather_code: usize,
}

#[derive(Deserialize, Debug)]
struct Daily {
    sunshine_duration: Vec<f64>,
    #[serde(rename(deserialize = "temperature_2m_max"))]
    temperature_max: Vec<f64>,
    #[serde(rename(deserialize = "temperature_2m_min"))]
    temperature_min: Vec<f64>,
    weather_code: Vec<usize>,
}

pub struct NiceDaily {
    pub sunshine: f64,
    pub temp_min: f64,
    pub temp_max: f64,
    pub weather_code: usize,
}

pub struct NiceCurrent {
    pub temperature: f64,
    pub humidity: f64,
    pub weather_code: usize,
}

pub struct NiceWeatherData {
    pub current: NiceCurrent,
    pub days: HashMap<NaiveDate, NiceDaily>,
}

impl From<WeatherData> for NiceWeatherData {
    fn from(value: WeatherData) -> Self {
        let nc = NiceCurrent {
            temperature: value.current.temperature,
            humidity: value.current.humidity,
            weather_code: value.current.weather_code,
        };
        let mut nd: HashMap<NaiveDate, NiceDaily> = HashMap::new();
        let today = chrono::Local::now().date_naive();
        for i in 0..value.daily.weather_code.len() {
            nd.insert(
                today.add(TimeDelta::days(i as i64)),
                NiceDaily {
                    temp_min: value.daily.temperature_min[i],
                    temp_max: value.daily.temperature_max[i],
                    sunshine: value.daily.sunshine_duration[i],
                    weather_code: value.daily.weather_code[i],
                },
            );
        }
        NiceWeatherData {
            current: nc,
            days: nd,
        }
    }
}

// See https://open-meteo.com/en/docs -> WMO Weather interpretation codes (WW)
// and https://www.nodc.noaa.gov/archive/arc0021/0002199/1.1/data/0-data/HTML/WMO-CODE/WMO4677.HTM
pub fn wmo_weather_code_to_str(code: usize) -> &'static str {
    match code {
        0 => "Clear sky",
        1 => "Mainly clear",
        2 => "Partly cloudy",
        3 => "Overcast",
        45 => "Fog",
        48 => "Depositing rime fog",
        51 => "Light drizzle",
        53 => "Moderate drizzle",
        55 => "Dense drizzle",
        56 => "Light freezing drizzle",
        57 => "Dense freezing drizzle",
        61 => "Slight rain",
        63 => "Moderate rain",
        65 => "Heavy rain",
        66 => "Light freezing rain",
        67 => "Heavy freezing rain",
        71 => "Slight snow fall",
        73 => "Moderate snow fall",
        75 => "Heavy snow fall",
        77 => "Snow grains",
        80 => "Slight rain showers",
        81 => "Moderate rain showers",
        82 => "Violent rain showers",
        85 => "Slight snow showers",
        86 => "Heavy snow showers",
        95 => "Slight or moderate thunderstorm",
        96 => "Thunderstorm with slight hail",
        99 => "Thunderstorm with heavy hail",
        _ => panic!("Unknown WMO code: {}", code),
    }
}

pub struct WeatherProvider {
    config: Config,
    http_client: reqwest::blocking::Client,
}

impl WeatherProvider {
    pub fn new(config: Config) -> WeatherProvider {
        WeatherProvider {
            config,
            http_client: reqwest::blocking::Client::new(),
        }
    }

    pub fn check_sky(&self) -> NiceWeatherData {
        let base_url = format!(
            "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&daily=sunshine_duration,temperature_2m_max,temperature_2m_min,\
        weather_code&models=best_match&current=temperature_2m,relative_humidity_2m,\
        weather_code&timezone={}&forecast_days=3",
            self.config.weather.latitude,
            self.config.weather.longitude,
            self.config.weather.timezone
        );

        let weather: WeatherData = self
            .http_client
            .get(base_url)
            .send()
            .expect("Could not fetch weather")
            .json::<WeatherData>()
            .expect("Weather data was invalid JSON");

        weather.into()
    }
}
