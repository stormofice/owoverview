use crate::calendar::{CalendarHandler, Event, Time};
use crate::epd::{Area, EPD_HEIGHT, EPD_WIDTH, EpdImage, Outline, Padding};
use crate::graphics::{Color, Rect};
use chrono::Timelike;
use fontdue::layout::{HorizontalAlign, LayoutSettings, TextStyle, VerticalAlign, WrapStyle};
use fontdue::{Font, FontSettings};

pub struct Dash {
    previous: Option<EpdImage>,
    calendar_handler: CalendarHandler,
}

impl Dash {
    pub fn new() -> Self {
        Self {
            previous: None,
            calendar_handler: CalendarHandler::new(),
        }
    }

    fn create_calendar(&self, cal: &mut Area) {
        let events = self.calendar_handler.fetch();

        let mut y_offset: usize = cal.get_vstart();

        const N_ENTRIES: usize = 6;
        const ENTRY_PADDING: usize = 2;

        let available_y_space = (cal.get_available_vspace() - ((N_ENTRIES - 1) * ENTRY_PADDING));
        assert_eq!(
            (available_y_space % N_ENTRIES),
            0,
            "Can't cleanly divide calendar area"
        );
        let entry_y_size: usize = available_y_space / N_ENTRIES;

        // TODO: change font
        let font_data = include_bytes!("../assets/Wellfleet/Wellfleet-Regular.ttf") as &[u8];
        let font =
            Font::from_bytes(font_data, FontSettings::default()).expect("Could not load font");

        let mut add_event = |e: &Event| {
            if y_offset >= cal.get_available_vspace() {
                panic!("calendar event oob")
            }

            let mut entry_area = Area::new(
                cal.get_hstart(),
                y_offset,
                cal.get_available_hspace(),
                entry_y_size,
                Color::White,
                Padding::full(0),
                Outline::default(),
            );

            println!("adding event: {:?}", e);

            let fmt = match e.time {
                Time::AllDay(date) => {
                    format!("{} - {}", date.format("%d.%m"), e.title)
                }
                Time::Timed(start, delta) => {
                    format!(
                        "{} ({}h) {}",
                        start.format("%d.%m:%H%M"),
                        delta.num_hours(),
                        e.title
                    )
                }
            };

            entry_area.auto_layout_text(
                &font,
                LayoutSettings {
                    max_height: Some(entry_area.get_available_vspace() as f32),
                    y: entry_area.get_vstart() as f32,
                    x: entry_area.get_hstart() as f32,
                    max_width: Some(entry_area.get_available_hspace() as f32),
                    wrap_style: WrapStyle::Letter,
                    horizontal_align: HorizontalAlign::Center,
                    vertical_align: VerticalAlign::Middle,
                    ..LayoutSettings::default()
                },
                &[TextStyle::new(fmt.as_str(), 16.0, 0)],
                160,
            );

            cal.add_sub_area(entry_area);

            y_offset += entry_y_size + ENTRY_PADDING;
        };

        // Sort by all day and timed
        for event in events.iter().filter(|e| matches!(e.time, Time::AllDay(_))) {
            add_event(event);
        }

        for event in events
            .iter()
            .filter(|e| matches!(e.time, Time::Timed(_, _)))
        {
            add_event(event);
        }
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

        quote_area.try_put_text(
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
        misc_column.try_put_text(
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

        let mut calendar_area = Area::new(
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
        self.create_calendar(&mut calendar_area);

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

        current.to_img_file("output.png");
        current.to_file("output.bin");

        self.previous = Some(current)
    }
}
