use crate::render::graphics::{Color, PixelColor, Rect};
use fontdue::Font;
use fontdue::layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle};
use image::{DynamicImage, GenericImageView, Luma, Pixel};
use std::io::Write;

pub const EPD_WIDTH: usize = 800;
pub const EPD_HEIGHT: usize = 480;

pub struct Padding {
    top: usize,
    bottom: usize,
    left: usize,
    right: usize,
}

impl Padding {
    pub fn full(pad: usize) -> Self {
        Padding {
            top: pad,
            bottom: pad,
            left: pad,
            right: pad,
        }
    }

    pub fn new(top: usize, bottom: usize, left: usize, right: usize) -> Self {
        Padding {
            top,
            bottom,
            left,
            right,
        }
    }
}

pub struct Offset {
    pub x: usize,
    y: usize,
}

pub struct Outline {
    pub top: usize,
    pub bottom: usize,
    pub left: usize,
    pub right: usize,
    pub color: Color,
}

impl Outline {
    pub fn none() -> Self {
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

pub struct Area {
    // In relative coordinates (total area)
    pub space: Rect,
    // In relative coordinates (drawable area (-padding, decorations, ...))
    pub canvas: Rect,
    // True x,y offset of space to image
    pub offset: Offset,

    fill: Color,
    padding: Padding,
    outline: Outline,

    buf: Vec<Vec<PixelColor>>,
    children: Vec<Area>,
}

impl Area {
    pub fn new(
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
                        let should = if y % 2 == 0 { x % 2 == 0 } else { x % 2 == 1 };
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

    fn is_layout_possible(
        &self,
        font: &Font,
        layout_settings: LayoutSettings,
        texts: &[TextStyle],
    ) -> bool {
        let mut is_possible = true;
        Self::layout_text(font, layout_settings, texts, |x, y, _| {
            if x >= (self.canvas.x + self.canvas.width) || y >= (self.canvas.y + self.canvas.height)
            {
                is_possible = false;
            }
        });
        is_possible
    }

    pub fn auto_layout_text_size(
        &mut self,
        font: &Font,
        layout_settings: LayoutSettings,
        texts: &[TextStyle],
        coverage_threshold: u8,
        max_text_size: f32,
    ) {
        let mut current_text_size = 1f32;

        let mut largest_text_styles: Vec<TextStyle> = vec![];
        loop {
            let mut text_styles: Vec<TextStyle> = vec![];
            for ts in texts {
                text_styles.push(TextStyle::new(ts.text, current_text_size, 0));
            }

            if self.is_layout_possible(font, layout_settings, text_styles.as_slice())
                && current_text_size <= max_text_size
            {
                current_text_size += 1f32;
                largest_text_styles = text_styles;
            } else {
                break;
            }
        }

        self.put_text(
            font,
            layout_settings,
            largest_text_styles.as_slice(),
            coverage_threshold,
        )
    }

    pub fn put_text(
        &mut self,
        font: &Font,
        layout_settings: LayoutSettings,
        texts: &[TextStyle],
        coverage_threshold: u8,
    ) {
        if !self.is_layout_possible(font, layout_settings, texts) {
            panic!("layouting impossible");
        }
        Self::layout_text(font, layout_settings, texts, |x, y, coverage| {
            let color = if coverage > coverage_threshold {
                PixelColor::Black
            } else {
                PixelColor::White
            };
            self.canvas.set_px(&mut self.buf, x, y, color)
        });
    }

    fn layout_text<F>(
        font: &Font,
        layout_settings: LayoutSettings,
        // lol
        texts: &[TextStyle],
        mut on_px: F,
    ) where
        F: FnMut(usize, usize, u8),
    {
        let mut layout: Layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&layout_settings);

        for ts in texts {
            layout.append(&[font], ts)
        }

        for glyph in layout.glyphs() {
            let (metrics, bitmap) = font.rasterize_config(glyph.key);
            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let idx = y * metrics.width + x;
                    let coverage = bitmap[idx];

                    on_px(
                        (x as f32 + glyph.x) as usize,
                        (y as f32 + glyph.y) as usize,
                        coverage,
                    )
                }
            }
        }
    }

    pub fn load_image(&mut self, x: usize, y: usize, image: &DynamicImage) {
        if (image.width() > self.get_available_hspace() as u32)
            || (image.height() > self.get_available_vspace() as u32)
        {
            panic!("Image is too large");
        }
        let offset_x = x;
        let offset_y = y;

        for y in 0..image.height() {
            for x in 0..image.width() {
                let pixel = image.get_pixel(x, y);
                let rgb = pixel.to_rgb();

                let avg = (rgb[0] as u32 + rgb[1] as u32 + rgb[2] as u32) / 3;
                let color = if avg > 127 {
                    PixelColor::White
                } else {
                    PixelColor::Black
                };
                self.canvas.set_px(
                    &mut self.buf,
                    x as usize + offset_x,
                    y as usize + offset_y,
                    color,
                );
            }
        }
    }

    pub fn add_sub_area(&mut self, mut area: Area) {
        area.offset.x += self.offset.x + self.canvas.x;
        area.offset.y += self.offset.y + self.canvas.y;
        area.children.iter_mut().for_each(|area| {
            area.offset.x += self.offset.x + self.canvas.x;
            area.offset.y += self.offset.y + self.canvas.y;
        });

        self.children.push(area)
    }

    pub fn draw(&self, image: &mut EpdImage) {
        self.render(image);

        self.children.iter().for_each(|c| c.draw(image));
    }

    fn render(&self, image: &mut EpdImage) {
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

    pub fn get_hstart(&self) -> usize {
        self.canvas.x
    }

    pub fn get_vstart(&self) -> usize {
        self.canvas.y
    }

    pub fn get_available_vspace(&self) -> usize {
        self.canvas.height
    }

    pub fn get_available_hspace(&self) -> usize {
        self.canvas.width
    }
}

pub struct EpdImage {
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

    pub fn to_file_partial(&self, filename: &str, x: usize, y: usize, w: usize, h: usize) {
        let mut partial = vec![0u8; w.div_ceil(8) * h];

        for y in y..(y + h) {
            for x in x..(x + w) {
                let byte_index = (y * w + x) / 8;
                let bit_index = x % 8;
                match self.get_pixel(x, y) {
                    PixelColor::White => partial[byte_index] |= 1 << (7 - bit_index),
                    PixelColor::Black => partial[byte_index] &= !(1 << (7 - bit_index)),
                }
            }
        }

        let mut file = std::fs::File::create(filename).unwrap();
        file.write_all(&partial)
            .expect("Could not write partial file");
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
}
