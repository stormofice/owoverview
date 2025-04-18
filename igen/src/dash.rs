use crate::calendar::{CalendarHandler, Event, Time};
use crate::epd::{Area, EPD_HEIGHT, EPD_WIDTH, EpdImage, Outline, Padding};
use crate::fonts::{Font, FontCollection};
use crate::graphics::{Color, Rect};
use crate::settings::Config;
use chrono::NaiveDate;
use fontdue::layout::{HorizontalAlign, LayoutSettings, TextStyle, VerticalAlign};
use std::collections::{BTreeSet, HashMap};

pub struct Dash {
    previous: Option<EpdImage>,
    calendar_handler: CalendarHandler,
    font_collection: FontCollection,
    config: Config,
}

// TODO: I think there should be a better way for this
#[allow(unused_macros)]
macro_rules! fast_create_text {
    ($font:expr, $area:ident, $layout_settings:expr, $styles:expr, $cover:expr) => {
        $area.auto_layout_text(
            $font,
            LayoutSettings {
                x: $area.get_hstart() as f32,
                y: $area.get_vstart() as f32,
                max_width: Some($area.get_available_hspace() as f32),
                max_height: Some($area.get_available_vspace() as f32),
                ..$layout_settings
            },
            $styles,
            $cover,
        );
    };
}

impl Dash {
    pub fn new(config: Config) -> Self {
        Self {
            config: config.clone(),
            previous: None,
            calendar_handler: CalendarHandler::new(config.google.clone()),
            font_collection: FontCollection::new(),
        }
    }

    fn create_calendar_day_grouped(&mut self, cal: &mut Area) {
        // This should be possible without the clone, no?
        let date_font = self.font_collection.load_font(Font::Wellfleet);
        let title_font = self.font_collection.load_font(Font::Dina);

        let mut events_per_day: HashMap<NaiveDate, Vec<Event>> = HashMap::new();
        let mut dates: BTreeSet<NaiveDate> = BTreeSet::new();
        self.calendar_handler
            .fetch()
            .into_iter()
            .for_each(|e| match e.time {
                Time::AllDay(nd) => {
                    dates.insert(nd);
                    if let std::collections::hash_map::Entry::Vacant(entry) =
                        events_per_day.entry(nd)
                    {
                        entry.insert(vec![e]);
                    } else {
                        events_per_day.get_mut(&nd).unwrap().push(e);
                    }
                }
                Time::Timed(dt, _) => {
                    dates.insert(dt.date_naive());
                    if let std::collections::hash_map::Entry::Vacant(entry) =
                        events_per_day.entry(dt.date_naive())
                    {
                        entry.insert(vec![e]);
                    } else {
                        events_per_day.get_mut(&dt.date_naive()).unwrap().push(e);
                    }
                }
            });

        const DAYS_SHOWN: usize = 4;
        const EVENTS_PER_DAY: usize = 3;
        const DATE_EVENT_PADDING: usize = 2;
        const DATE_HEIGHT: usize = 32;
        const EVENT_HEIGHT: usize = 24;
        const EVENT_PADDING: usize = 2;
        const TITLE_MAX_LENGTH: usize = 16;
        let fit_title = |title: &str| {
            if title.len() > TITLE_MAX_LENGTH {
                format!("{}>", &title[0..TITLE_MAX_LENGTH])
            } else {
                title.to_string()
            }
        };

        let mut cur_y = cal.get_vstart();

        for date in dates.iter().take(DAYS_SHOWN) {
            let mut date_area = Area::new(
                0,
                cur_y,
                cal.get_available_hspace(),
                DATE_HEIGHT,
                Color::White,
                Padding::full(0),
                Outline::default(),
            );
            date_area.try_put_text(
                &date_font,
                LayoutSettings {
                    x: date_area.get_hstart() as f32,
                    y: date_area.get_vstart() as f32,
                    max_width: Some(date_area.get_available_hspace() as f32),
                    max_height: Some(date_area.get_available_vspace() as f32),
                    horizontal_align: HorizontalAlign::Center,
                    vertical_align: VerticalAlign::Middle,
                    ..LayoutSettings::default()
                },
                &[TextStyle::new(
                    date.format("%a, %-d. %b").to_string().as_str(),
                    23.0,
                    0,
                )],
                50,
            );
            cal.add_sub_area(date_area);

            cur_y += DATE_HEIGHT + DATE_EVENT_PADDING;

            for event in events_per_day
                .get(date)
                .unwrap()
                .iter()
                .take(EVENTS_PER_DAY)
            {
                let mut event_area = Area::new(
                    0,
                    cur_y,
                    cal.get_available_hspace(),
                    EVENT_HEIGHT,
                    Color::White,
                    Padding::full(0),
                    Outline::none(),
                );

                let text = match event.time {
                    Time::AllDay(_) => &event.title,
                    Time::Timed(dt, _) => &format!("{} {}", dt.format("%H:%M"), event.title),
                };

                event_area.try_put_text(
                    &title_font,
                    LayoutSettings {
                        x: event_area.get_hstart() as f32,
                        y: event_area.get_vstart() as f32,
                        max_width: Some(event_area.get_available_hspace() as f32),
                        max_height: Some(event_area.get_available_vspace() as f32),
                        horizontal_align: HorizontalAlign::Left,
                        vertical_align: VerticalAlign::Middle,
                        ..LayoutSettings::default()
                    },
                    &[TextStyle::new(fit_title(text).as_str(), 22.0, 0)],
                    20,
                );
                cal.add_sub_area(event_area);

                cur_y += EVENT_HEIGHT + EVENT_PADDING;
            }
        }
    }

    fn create_dashboard(&mut self) -> EpdImage {
        let mut image = EpdImage::new(EPD_WIDTH, EPD_HEIGHT);

        let font = self.font_collection.load_font(Font::Wellfleet);

        let mut total = Area::new(
            0,
            0,
            EPD_WIDTH,
            EPD_HEIGHT,
            Color::White,
            Padding::full(0),
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

        quote_area.auto_layout_text_size(
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
        let now_str = now.format("%H:%M").to_string();
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
                top: 0,
                bottom: 1,
                left: 0,
                right: 1,
                color: Color::Black,
            },
        );
        self.create_calendar_day_grouped(&mut calendar_area);

        let weather_area = Area::new(
            0,
            left_column.get_available_vspace() / 2,
            left_column.get_available_hspace(),
            left_column.get_available_vspace() / 2,
            Color::White,
            Padding::full(2),
            Outline {
                top: 1,
                bottom: 0,
                left: 0,
                right: 1,
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
