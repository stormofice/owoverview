use crate::settings::Config;
use chrono::{DateTime, Duration, Utc};
use image::DynamicImage;
use std::collections::VecDeque;
use std::fs;
use std::path::Path;

pub struct ImageProvider {
    config: Config,
    image_paths: VecDeque<String>,
    cache: Option<(DateTime<Utc>, DynamicImage)>,
}

impl ImageProvider {
    fn load_images(&mut self) {
        let image_paths: Vec<String> = serde_json::from_str(
            fs::read_to_string(&self.config.image.images_path)
                .expect("Could not read image json")
                .as_str(),
        )
        .expect("Could not deserialize image json");

        image_paths
            .iter()
            .for_each(|ip| assert!(Path::new(ip).exists(), "Image {} doesn't exist", ip));

        self.image_paths = VecDeque::from(image_paths);
    }

    pub fn new(config: Config) -> Self {
        ImageProvider {
            config,
            image_paths: VecDeque::new(),
            cache: None,
        }
    }

    pub fn get_image(&mut self) -> DynamicImage {
        if let Some((last_refresh, last_image)) = self.cache.as_ref() {
            if Utc::now().signed_duration_since(last_refresh) > Duration::minutes(60) {
            } else {
                return last_image.clone();
            }
        }

        match self.image_paths.pop_front() {
            None => {
                self.load_images();
                if self.image_paths.is_empty() {
                    panic!("No images available");
                }
                self.get_image()
            }
            Some(d) => {
                let img = image::open(d).expect("Could not load image");
                self.cache = Some((Utc::now(), img.clone()));
                img
            }
        }
    }
}
