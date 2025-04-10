use image::{GenericImageView, Pixel};
use std::io::Write;

const EPD_WIDTH: usize = 800;
const EPD_HEIGHT: usize = 480;

const WHITE: u8 = 0xFF;
const BLACK: u8 = 0x00;

fn main() {
    let mut image = EpdImage::new(EPD_WIDTH, EPD_HEIGHT);

    image.load_image("glad.webp");

    image.to_file("output.bin");
}

struct EpdImage {
    // 1 pixel ber bit
    data: Vec<u8>,
}

impl EpdImage {
    pub fn new(width: usize, height: usize) -> Self {
        let size = (width * height).div_ceil(8);
        EpdImage {
            data: vec![WHITE; size],
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: u8) {
        let byte_index = (y * EPD_WIDTH + x) / 8;
        let bit_index = x % 8;
        if color == WHITE {
            self.data[byte_index] |= 1 << (7 - bit_index);
        } else {
            self.data[byte_index] &= !(1 << (7 - bit_index));
        }
    }

    pub fn to_file(&self, filename: &str) {
        let mut file = std::fs::File::create(filename).unwrap();
        file.write_all(&self.data).unwrap();
    }

    pub fn load_image(&mut self, filename: &str) {
        let img = image::open(filename).expect("Failed to load image");
        if (img.width() > EPD_WIDTH as u32) || (img.height() > EPD_HEIGHT as u32) {
            panic!("Image is too large");
        }
        // center
        let offset_x = (EPD_WIDTH as u32 - img.width()) / 2;
        let offset_y = (EPD_HEIGHT as u32 - img.height()) / 2;

        for y in 0..img.height() {
            for x in 0..img.width() {
                let pixel = img.get_pixel(x, y);
                let rgb = pixel.to_rgb();

                let avg = (rgb[0] as u32 + rgb[1] as u32 + rgb[2] as u32) / 3;
                let color = if avg > 127 { WHITE } else { BLACK };
                self.set_pixel((x + offset_x) as usize, (y + offset_y) as usize, color);
            }
        }
    }
}
