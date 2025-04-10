use image::{GenericImageView, Luma, Pixel};
use std::io::Write;

const EPD_WIDTH: usize = 800;
const EPD_HEIGHT: usize = 480;

#[derive(Copy, Clone, Debug)]
#[repr(u8)]
enum PixelColor {
    White = 0xFF,
    Black = 0x00,
}

#[derive(Copy, Clone, Debug)]
enum Color {
    White,
    Black,
    Gray,
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
        Padding::full(2),
        Outline::none(),
    );

    let mut left_column = Area::new(
        0,
        0,
        200,
        total.get_available_vspace(),
        Color::Black,
        Padding::full(0),
        Outline::none(),
    );

    let mut right_column = Area::new(
        left_column.offset.x + left_column.space.width,
        0,
        total.get_available_hspace() - left_column.space.width,
        total.get_available_vspace(),
        Color::Gray,
        Padding::full(0),
        Outline::none(),
    );

    let quote_area = Area::new(
        0,
        right_column.get_available_vspace() - 140,
        right_column.get_available_hspace(),
        140,
        Color::White,
        Padding::full(4),
        Outline {
            right: 2,
            bottom: 2,
            left: 0, // borders right column
            top: 2,
            color: Color::Black,
        },
    );

    let misc_column = Area::new(
        right_column.get_available_hspace() - 100,
        0,
        100,
        right_column.get_available_vspace() - quote_area.space.height,
        Color::Black,
        Padding::full(4),
        Outline::default(),
    );

    right_column.add_sub_area(quote_area);
    right_column.add_sub_area(misc_column);

    let calendar_area = Area::new(
        0,
        0,
        left_column.get_available_hspace(),
        left_column.get_available_vspace() / 2,
        Color::White,
        Padding::full(2),
        Outline {
            top: 2,
            bottom: 1,
            left: 2,
            right: 2,
            color: Color::Black,
        },
    );

    let weather_area = Area::new(
        0,
        left_column.get_available_vspace() / 2,
        left_column.get_available_hspace(),
        left_column.get_available_vspace() / 2,
        Color::White,
        Padding::full(2),
        Outline {
            top: 1,
            bottom: 2,
            left: 2,
            right: 2,
            color: Color::Black,
        },
    );

    left_column.add_sub_area(calendar_area);
    left_column.add_sub_area(weather_area);

    total.add_sub_area(left_column);
    total.add_sub_area(right_column);

    total.draw(image);
}

struct Padding {
    top: usize,
    bottom: usize,
    left: usize,
    right: usize,
}

impl Padding {
    fn full(pad: usize) -> Self {
        Padding {
            top: pad,
            bottom: pad,
            left: pad,
            right: pad,
        }
    }
}

#[derive(Copy, Clone)]
struct Rect {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl Rect {
    fn set_px(&self, buf: &mut [Vec<PixelColor>], x: usize, y: usize, color: PixelColor) {
        buf[self.y + y][self.x + x] = color;
    }

    fn get_px(&self, buf: &[Vec<PixelColor>], x: usize, y: usize) -> PixelColor {
        buf[self.y + y][self.x + x]
    }
}

struct Offset {
    x: usize,
    y: usize,
}

struct Outline {
    top: usize,
    bottom: usize,
    left: usize,
    right: usize,
    color: Color,
}

impl Outline {
    fn none() -> Self {
        Self {
            color: Color::Gray,
            top: 0,
            left: 0,
            bottom: 0,
            right: 0,
        }
    }
}

impl Default for Outline {
    fn default() -> Self {
        Self {
            top: 2,
            bottom: 2,
            left: 2,
            right: 2,
            color: Color::Black,
        }
    }
}

impl From<Color> for PixelColor {
    fn from(value: Color) -> Self {
        match value {
            Color::White => PixelColor::White,
            Color::Black => PixelColor::Black,
            Color::Gray => panic!(),
        }
    }
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
    outline: Outline,

    buf: Vec<Vec<PixelColor>>,
    children: Vec<Area>,
}

impl Area {
    fn new(
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        fill: Color,
        padding: Padding,
        outline: Outline,
    ) -> Self {
        let space = Rect {
            x: 0,
            y: 0,
            width,
            height,
        };
        let dr = Rect {
            x: padding.left + outline.left,
            y: padding.top + outline.top,
            width: width - padding.left - padding.right - outline.left - outline.right,
            height: height - padding.top - padding.bottom - outline.top - outline.bottom,
        };

        let mut buf = vec![vec![PixelColor::White; width]; height];

        // Draw outline (top)
        for y in 0..outline.top {
            for x in 0..space.width {
                buf[y][x] = outline.color.into();
            }
        }

        // (bottom)
        for y in (space.height - outline.bottom)..space.height {
            for x in 0..space.width {
                buf[y][x] = outline.color.into();
            }
        }

        // (left)
        for y in 0..space.height {
            for x in 0..outline.left {
                buf[y][x] = outline.color.into();
            }
        }

        // (right)
        for y in 0..space.height {
            for x in (space.width - outline.right)..space.width {
                buf[y][x] = outline.color.into();
            }
        }

        for x in 0..dr.width {
            for y in 0..dr.height {
                match fill {
                    Color::White => dr.set_px(&mut buf, x, y, PixelColor::White),
                    Color::Black => dr.set_px(&mut buf, x, y, PixelColor::Black),
                    Color::Gray => {
                        let should = if y % 2 == 0 { (x % 2 == 0) } else { x % 2 == 1 };
                        let color = if should {
                            PixelColor::White
                        } else {
                            PixelColor::Black
                        };
                        dr.set_px(&mut buf, x, y, color)
                    }
                }
            }
        }

        Self {
            space,
            canvas: dr,
            offset: Offset { x, y },
            fill,
            padding,
            outline,
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
        if self.buf.len() == 332 {
            let k = 0;
        }

        for y in 0..self.space.height {
            for x in 0..self.space.width {
                image.set_pixel(
                    x + self.offset.x,
                    y + self.offset.y,
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
            data: vec![PixelColor::White as u8; size],
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: PixelColor) {
        let byte_index = (y * EPD_WIDTH + x) / 8;
        let bit_index = x % 8;
        match color {
            PixelColor::White => self.data[byte_index] |= 1 << (7 - bit_index),
            PixelColor::Black => self.data[byte_index] &= !(1 << (7 - bit_index)),
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> PixelColor {
        let byte_index = (y * EPD_WIDTH + x) / 8;
        let bit_index = x % 8;
        let clr = (self.data[byte_index] >> (7 - bit_index)) & 0x1;
        if clr == 1 {
            PixelColor::White
        } else {
            PixelColor::Black
        }
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
                    if matches!(self.get_pixel(x, y), PixelColor::White) {
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
                    PixelColor::White
                } else {
                    PixelColor::Black
                };
                self.set_pixel((x + offset_x) as usize, (y + offset_y) as usize, color);
            }
        }
    }
}
