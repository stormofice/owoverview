use crate::epd::{Area, EPD_HEIGHT, EPD_WIDTH, EpdImage, Outline, Padding};
use crate::graphics::{Color, Rect};
use chrono::Timelike;
use fontdue::layout::{HorizontalAlign, LayoutSettings, TextStyle};
use fontdue::{Font, FontSettings};

pub struct Dash {
    previous: Option<EpdImage>,
}

impl Dash {
    pub fn new() -> Self {
        Self { previous: None }
    }

    fn create_dashboard(&self) -> EpdImage {
        let mut image = EpdImage::new(EPD_WIDTH, EPD_HEIGHT);

        let font_data = include_bytes!("../assets/Wellfleet/Wellfleet-Regular.ttf") as &[u8];
        let font =
            Font::from_bytes(font_data, FontSettings::default()).expect("Could not load font");

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
            Color::White,
            Padding::full(0),
            Outline::none(),
        );

        let mut quote_area = Area::new(
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

        quote_area.layout_text(
            &font,
            LayoutSettings {
                max_height: Some(quote_area.get_available_vspace() as f32),
                max_width: Some(quote_area.get_available_hspace() as f32),
                ..LayoutSettings::default()
            },
            &[TextStyle::new(
                "Do not worry if you have built your castles in the air. They are where \
                    they should be. Now put the foundations under them.",
                20f32,
                0,
            )],
            100,
        );

        let mut misc_column = Area::new(
            right_column.get_available_hspace() - 100,
            0,
            100,
            right_column.get_available_vspace() - quote_area.space.height,
            Color::White,
            Padding::full(0),
            Outline::default(),
        );

        let now = chrono::Local::now();
        let now_str = format!("{}:{}", now.hour(), now.minute());
        misc_column.layout_text(
            &font,
            LayoutSettings {
                max_width: Some(misc_column.get_available_hspace() as f32),
                max_height: Some(misc_column.get_available_vspace() as f32),
                horizontal_align: HorizontalAlign::Center,
                ..LayoutSettings::default()
            },
            &[TextStyle::new(now_str.as_str(), 32f32, 0)],
            120,
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

        total.draw(&mut image);

        image
    }

    fn get_change_bbox(&self, current: &EpdImage) -> Option<Rect> {
        if let Some(previous) = &self.previous {
            let mut xmin = EPD_WIDTH;
            let mut ymin = EPD_HEIGHT;
            let mut xmax = 0;
            let mut ymax = 0;

            let mut change = false;
            for y in 0..EPD_HEIGHT {
                for x in 0..EPD_WIDTH {
                    if current.get_pixel(x, y) != previous.get_pixel(x, y) {
                        xmin = xmin.min(x);
                        xmax = xmax.max(x);
                        ymin = ymin.min(y);
                        ymax = ymin.max(y);
                        change = true;
                    }
                }
            }

            if !change {
                return None;
            }

            Some(Rect {
                x: xmin,
                y: ymin,
                width: (xmax - xmin),
                height: (ymax - ymin),
            })
        } else {
            None
        }
    }

    pub fn draw(&mut self) {
        let current = self.create_dashboard();

        if let Some(bbox) = self.get_change_bbox(&current) {
            println!("change bbox: {:?}", bbox);
        }

        self.previous = Some(current)
    }
}
