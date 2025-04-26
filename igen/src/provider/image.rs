use crate::settings::Config;
use image::DynamicImage;
use image::imageops::FilterType;
use std::collections::VecDeque;
use std::fs;
use std::path::Path;

pub struct ImageProvider {
    config: Config,
    image_paths: VecDeque<String>,
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
        }
    }

    pub fn get_image(&mut self) -> DynamicImage {
        match self.image_paths.pop_front() {
            None => {
                self.load_images();
                if self.image_paths.is_empty() {
                    panic!("No images available");
                }
                self.get_image()
            }
            Some(d) => {
                let base = image::open(d).expect("Could not load image").resize_exact(
                    800,
                    480,
                    FilterType::Nearest,
                );
                let dithered = dither::error_diffusion_quantise(
                    &base,
                    &dither::diffusion_matrices::JARVIS_JUDICE_NINKE,
                    false,
                );
                DynamicImage::from(dithered)
            }
        }
    }
}
