use image::{GenericImageView, Luma, Pixel};
use std::io::Write;

const EPD_WIDTH: usize = 800;
const EPD_HEIGHT: usize = 480;

#[derive(Copy, Clone)]
#[repr(u8)]
enum Color {
    White = 0xFF,
    Black = 0x00,
}

fn main() {
    let mut image = EpdImage::new(EPD_WIDTH, EPD_HEIGHT);

    draw_dashboard(&mut image);

    image.to_file("output.bin");
    image.to_img_file("output.png");
}

fn draw_dashboard(image: &mut EpdImage) {
    const PADDING: usize = 4;
    let padded_v_split = (EPD_HEIGHT - (3 * PADDING)) / 2;

    let cal_area = Area::new(4, 4, 200, padded_v_split).with_fill(Color::Black);
    let weather_area =
        Area::new(4, 4 + padded_v_split + 4, 200, padded_v_split).with_fill(Color::Black);

    cal_area.render(image);
    weather_area.render(image);
}

struct Area {
    x: usize,
    y: usize,
    width: usize,
    height: usize,

    fill: Color,

    buf: Vec<Vec<Color>>,
}

impl Area {
    fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        Self {
            x,
            y,
            width,
            height,
            fill: Color::White,
            buf: vec![vec![Color::White; width]; height],
        }
    }

    fn with_fill(mut self, fill: Color) -> Self {
        self.fill = fill;
        self.buf.iter_mut().for_each(|v| v.fill(fill));
        self
    }

    fn render(&self, image: &mut EpdImage) {
        for y in self.y..(self.y + self.height) {
            for x in self.x..(self.x + self.width) {
                image.set_pixel(x, y, self.get_px(x, y));
            }
        }
    }

    fn set_px(&mut self, x: usize, y: usize, color: Color) {
        self.buf[y - self.y][x - self.x] = color;
    }

    fn get_px(&self, x: usize, y: usize) -> Color {
        self.buf[y - self.y][x - self.x]
    }
}

struct EpdImage {
    // 1 pixel ber bit
    data: Vec<u8>,
}

impl EpdImage {
    pub fn new(width: usize, height: usize) -> Self {
        let size = (width * height).div_ceil(8);
        EpdImage {
            data: vec![Color::White as u8; size],
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        let byte_index = (y * EPD_WIDTH + x) / 8;
        let bit_index = x % 8;
        match color {
            Color::White => self.data[byte_index] |= 1 << (7 - bit_index),
            Color::Black => self.data[byte_index] &= !(1 << (7 - bit_index)),
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> Color {
        let byte_index = (y * EPD_WIDTH + x) / 8;
        let bit_index = x % 8;
        let clr = (self.data[byte_index] >> (7 - bit_index)) & 0x1;
        if clr == 1 { Color::White } else { Color::Black }
    }

    pub fn to_file(&self, filename: &str) {
        let mut file = std::fs::File::create(filename).unwrap();
        file.write_all(&self.data).unwrap();
    }

    pub fn to_img_file(&self, filename: &str) {
        let mut image = image::GrayImage::new(EPD_WIDTH as u32, EPD_HEIGHT as u32);
        for y in 0..EPD_HEIGHT {
            for x in 0..EPD_WIDTH {
                image.put_pixel(
                    x as u32,
                    y as u32,
                    if matches!(self.get_pixel(x, y), Color::White) {
                        Luma([0xFF])
                    } else {
                        Luma([0x00])
                    },
                );
            }
        }
        image.save(filename).expect("Could not save image")
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
                let color = if avg > 127 {
                    Color::White
                } else {
                    Color::Black
                };
                self.set_pixel((x + offset_x) as usize, (y + offset_y) as usize, color);
            }
        }
    }
}
