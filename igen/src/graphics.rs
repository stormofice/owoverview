#[derive(Copy, Clone, Debug)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl Rect {
    pub fn set_px(&self, buf: &mut [Vec<PixelColor>], x: usize, y: usize, color: PixelColor) {
        buf[self.y + y][self.x + x] = color;
    }

    pub fn get_px(&self, buf: &[Vec<PixelColor>], x: usize, y: usize) -> PixelColor {
        buf[self.y + y][self.x + x]
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

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PixelColor {
    White = 0xFF,
    Black = 0x00,
}

#[derive(Copy, Clone, Debug)]
pub enum Color {
    White,
    Black,
    Gray,
}
