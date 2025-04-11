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
    let mut total = Area::new(
        0,
        0,
        EPD_WIDTH,
        EPD_HEIGHT,
        Color::White,
        Padding { dx: 4, dy: 4 },
    );

    let mut left_column = Area::new(
        0,
        0,
        200,
        total.get_available_vspace(),
        Color::Black,
        Padding { dx: 0, dy: 0 },
    );

    let calendar_area = Area::new(
        0,
        0,
        left_column.get_available_hspace(),
        left_column.get_available_vspace() / 2,
        Color::White,
        Padding { dx: 4, dy: 4 },
    );

    let weather_area = Area::new(
        0,
        left_column.get_available_vspace() / 2,
        left_column.get_available_hspace(),
        left_column.get_available_vspace() / 2,
        Color::White,
        Padding { dx: 4, dy: 4 },
    );

    left_column.add_sub_area(calendar_area);
    left_column.add_sub_area(weather_area);

    total.add_sub_area(left_column);

    total.draw(image);
}

struct Padding {
    dx: usize,
    dy: usize,
}

#[derive(Copy, Clone)]
struct Rect {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl Rect {
    fn set_px(&self, buf: &mut [Vec<Color>], x: usize, y: usize, color: Color) {
        buf[self.y + y][self.x + x] = color;
    }

    fn get_px(&self, buf: &[Vec<Color>], x: usize, y: usize) -> Color {
        buf[self.y + y][self.x + x]
    }
}

struct Offset {
    x: usize,
    y: usize,
}

struct Area {
    // In relative coordinates (total area)
    space: Rect,
    // In relative coordinates (drawable area (-padding, decorations, ...))
    canvas: Rect,
    // True x,y offset of space to image
    offset: Offset,

    fill: Color,
    padding: Padding,

    buf: Vec<Vec<Color>>,
    children: Vec<Area>,
}

impl Area {
    fn new(x: usize, y: usize, width: usize, height: usize, fill: Color, padding: Padding) -> Self {
        let space = Rect {
            x: 0,
            y: 0,
            width,
            height,
        };
        let dr = Rect {
            x: padding.dx,
            y: padding.dy,
            width: width - (2 * padding.dx),
            height: height - (2 * padding.dy),
        };

        let mut buf = vec![vec![Color::White; width]; height];

        for x in 0..dr.width {
            for y in 0..dr.height {
                dr.set_px(&mut buf, x, y, fill)
            }
        }

        Self {
            space,
            canvas: dr,
            offset: Offset { x, y },
            fill,
            padding,
            buf,
            children: vec![],
        }
    }

    fn add_sub_area(&mut self, mut area: Area) {
        area.offset.x += (self.offset.x + self.canvas.x);
        area.offset.y += (self.offset.y + self.canvas.y);
        area.children.iter_mut().for_each(|area| {
            area.offset.x += (self.offset.x + self.canvas.x);
            area.offset.y += (self.offset.y + self.canvas.y);
        });

        self.children.push(area)
    }

    fn draw(&self, image: &mut EpdImage) {
        self.render(image);

        self.children.iter().for_each(|c| c.draw(image));
    }

    fn render(&self, image: &mut EpdImage) {
        for y in 0..self.canvas.height {
            for x in 0..self.canvas.width {
                image.set_pixel(
                    x + self.offset.x + self.canvas.x,
                    y + self.offset.y + self.canvas.y,
                    self.space.get_px(&self.buf, x, y),
                );
            }
        }
    }

    fn get_available_vspace(&self) -> usize {
        self.canvas.height
    }

    fn get_available_hspace(&self) -> usize {
        self.canvas.width
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
