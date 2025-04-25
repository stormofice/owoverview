use crate::provider::google::{CalendarProvider, Event, Time};
use crate::provider::image::ImageProvider;
use crate::provider::quote::QuoteProvider;
use crate::provider::weather::{NiceDaily, WeatherProvider, wmo_weather_code_to_str};
use crate::render::epd::{Area, EPD_HEIGHT, EPD_WIDTH, EpdImage, Outline, Padding};
use crate::render::fonts::{Font, FontCollection};
use crate::render::graphics::{Color, Rect};
use crate::settings::Config;
use chrono::{NaiveDate, TimeDelta};
use fontdue::layout::{HorizontalAlign, LayoutSettings, TextStyle, VerticalAlign};
use image::imageops;
use log::debug;
use reqwest::blocking::multipart;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::ops::Add;

pub struct Dash {
    previous_frame: Option<EpdImage>,
    calendar_provider: CalendarProvider,
    quote_provider: QuoteProvider,
    image_provider: ImageProvider,
    weather_provider: WeatherProvider,
    font_collection: FontCollection,
    config: Config,
}

impl Dash {
    pub fn new(config: Config) -> Self {
        Self {
            config: config.clone(),
            previous_frame: None,
            calendar_provider: CalendarProvider::new(config.clone()),
            quote_provider: QuoteProvider::new(config.quote.clone()),
            image_provider: ImageProvider::new(config.clone()),
            weather_provider: WeatherProvider::new(config.clone()),
            font_collection: FontCollection::new(),
        }
    }

    fn create_calendar_day_grouped(&mut self, cal: &mut Area) {
        // This should be possible without the clone, no?
        let date_font = self.font_collection.load_font(Font::Wellfleet);
        let title_font = self.font_collection.load_font(Font::Dina);

        let mut events_per_day: HashMap<NaiveDate, Vec<Event>> = HashMap::new();
        let mut dates: BTreeSet<NaiveDate> = BTreeSet::new();
        self.calendar_provider
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

        for date in dates {
            // Take until we can still fit a date + event
            if (cur_y + DATE_HEIGHT + DATE_EVENT_PADDING + EVENT_HEIGHT)
                >= cal.get_available_vspace()
            {
                break;
            }

            let mut date_area = Area::new(
                0,
                cur_y,
                cal.get_available_hspace(),
                DATE_HEIGHT,
                Color::White,
                Padding::full(0),
                Outline::default(),
            );
            date_area.put_text(
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

            for event in events_per_day.get(&date).unwrap().iter() {
                if (cur_y + EVENT_HEIGHT) >= cal.get_available_vspace() {
                    break;
                }

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

                event_area.put_text(
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

    fn create_quote(&mut self, quote_area: &mut Area) {
        let quote = self.quote_provider.get_quote();

        quote_area.auto_layout_text_size(
            &self.font_collection.load_font(Font::Wellfleet),
            LayoutSettings {
                x: quote_area.get_hstart() as f32,
                y: quote_area.get_vstart() as f32,
                max_height: Some(quote_area.get_available_vspace() as f32),
                max_width: Some(quote_area.get_available_hspace() as f32),
                ..LayoutSettings::default()
            },
            &[TextStyle::new(&quote.content, 1f32, 0)],
            100,
            30f32,
        );
    }

    fn create_image(&mut self, image_area: &mut Area) {
        let image = self.image_provider.get_image();
        let resized = image.resize(
            image_area.get_available_hspace() as u32,
            image_area.get_available_vspace() as u32,
            imageops::FilterType::Nearest,
        );
        let x_off = (image_area.get_available_hspace() - resized.width() as usize) / 2;
        let y_off = (image_area.get_available_vspace() - resized.height() as usize) / 2;

        image_area.load_image(x_off, y_off, &resized);
    }

    fn create_weather(&mut self, weather_area: &mut Area) {
        let weather = self.weather_provider.check_sky();

        let day_font = self.font_collection.load_font(Font::Wellfleet);
        let weather_font = self.font_collection.load_font(Font::Dina);
        let mut y_off = 0;
        const DAY_NAME_STEP_SIZE: usize = 28;
        weather_area.put_text(
            &day_font,
            LayoutSettings {
                x: weather_area.get_hstart() as f32,
                y: y_off as f32,
                max_width: Some(weather_area.get_available_hspace() as f32),
                max_height: Some(weather_area.get_available_vspace() as f32),
                horizontal_align: HorizontalAlign::Center,
                vertical_align: VerticalAlign::Top,
                ..LayoutSettings::default()
            },
            &[TextStyle::new("Now", 23.0, 0)],
            100,
        );
        y_off += DAY_NAME_STEP_SIZE;

        let mut now_area = Area::new(
            0,
            y_off,
            weather_area.get_available_hspace(),
            50,
            Color::White,
            Padding::full(0),
            Outline {
                left: 0,
                right: 0,
                color: Color::Black,
                top: 1,
                bottom: 1,
            },
        );
        now_area.auto_layout_text_size(
            &weather_font,
            LayoutSettings {
                x: now_area.get_hstart() as f32,
                y: 0.0,
                max_width: Some(now_area.get_available_hspace() as f32),
                max_height: Some((now_area.get_available_vspace()) as f32),
                horizontal_align: HorizontalAlign::Center,
                ..LayoutSettings::default()
            },
            &[TextStyle::new(
                wmo_weather_code_to_str(weather.current.weather_code),
                24.0,
                0,
            )],
            40,
            24.0,
        );
        now_area.put_text(
            &weather_font,
            LayoutSettings {
                x: now_area.get_hstart() as f32,
                y: 28.0,
                max_width: Some(now_area.get_available_hspace() as f32),
                max_height: Some((now_area.get_available_vspace()) as f32),
                horizontal_align: HorizontalAlign::Center,
                ..LayoutSettings::default()
            },
            &[TextStyle::new(
                format!(
                    "{}°C {}%",
                    weather.current.temperature, weather.current.humidity
                )
                .as_str(),
                20.0,
                0,
            )],
            40,
        );

        y_off += now_area.space.height;
        weather_area.add_sub_area(now_area);

        let tomorrow = weather
            .days
            .get(&chrono::Local::now().date_naive().add(TimeDelta::days(1)))
            .expect("There is no tomorrow");
        let day_after_tmrw = weather
            .days
            .get(&chrono::Local::now().date_naive().add(TimeDelta::days(2)))
            .expect("There is no day after tomorrow");

        let mut show_weather_for_day = |day: &NiceDaily, name: &str| {
            weather_area.put_text(
                &day_font,
                LayoutSettings {
                    x: weather_area.get_hstart() as f32,
                    y: y_off as f32,
                    max_width: Some(weather_area.get_available_hspace() as f32),
                    max_height: Some(weather_area.get_available_vspace() as f32),
                    horizontal_align: HorizontalAlign::Center,
                    ..LayoutSettings::default()
                },
                &[TextStyle::new(name, 23.0, 0)],
                100,
            );

            y_off += DAY_NAME_STEP_SIZE;

            let mut day_area = Area::new(
                0,
                y_off,
                weather_area.get_available_hspace(),
                50,
                Color::White,
                Padding::full(0),
                Outline {
                    left: 0,
                    right: 0,
                    color: Color::Black,
                    top: 1,
                    bottom: 1,
                },
            );

            let max_wmo_size = if wmo_weather_code_to_str(day.weather_code).len() >= 19 {
                20.0
            } else {
                24.0
            };

            day_area.auto_layout_text_size(
                &weather_font,
                LayoutSettings {
                    x: day_area.get_hstart() as f32,
                    y: 0.0,
                    max_width: Some(day_area.get_available_hspace() as f32),
                    max_height: Some(18f32),
                    horizontal_align: HorizontalAlign::Center,
                    ..LayoutSettings::default()
                },
                &[TextStyle::new(
                    wmo_weather_code_to_str(day.weather_code),
                    0.0,
                    0,
                )],
                40,
                max_wmo_size,
            );
            day_area.put_text(
                &weather_font,
                LayoutSettings {
                    x: day_area.get_hstart() as f32,
                    y: 28.0,
                    max_width: Some(day_area.get_available_hspace() as f32),
                    max_height: Some((day_area.get_available_vspace()) as f32),
                    horizontal_align: HorizontalAlign::Center,
                    ..LayoutSettings::default()
                },
                &[TextStyle::new(
                    format!(
                        "{:05.2}°C-{:04.2}°C {:02}h",
                        day.temp_min,
                        day.temp_max,
                        (day.sunshine / 3600f64).round() as usize
                    )
                    .as_str(),
                    20.0,
                    0,
                )],
                40,
            );
            y_off += day_area.space.height;
            weather_area.add_sub_area(day_area);
        };

        show_weather_for_day(tomorrow, "Tomorrow");
        show_weather_for_day(day_after_tmrw, "Tomorrow++");
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
            Padding::full(2),
            Outline {
                right: 0,
                bottom: 0,
                left: 0, // borders left column
                top: 1,
                color: Color::Black,
            },
        );

        self.create_quote(&mut quote_area);

        let mut misc_column = Area::new(
            right_column.get_available_hspace() - 100,
            0,
            100,
            // right_column.get_available_vspace() - quote_area.space.height,
            40,
            Color::White,
            Padding::full(0),
            Outline {
                color: Color::Black,
                bottom: 1,
                top: 0,
                left: 1,
                right: 0,
            },
        );

        let mut image_area = Area::new(
            0,
            0,
            // right_column.get_available_hspace() - misc_column.space.width,
            right_column.get_available_hspace(),
            right_column.get_available_vspace() - quote_area.space.height,
            Color::White,
            Padding::full(0),
            Outline::none(),
        );
        self.create_image(&mut image_area);
        right_column.add_sub_area(image_area);

        let now = chrono::Local::now();
        let now_str = now.format("%H:%M").to_string();
        misc_column.put_text(
            &font,
            LayoutSettings {
                max_width: Some(misc_column.get_available_hspace() as f32),
                max_height: Some(misc_column.get_available_vspace() as f32),
                horizontal_align: HorizontalAlign::Center,
                ..LayoutSettings::default()
            },
            &[TextStyle::new(now_str.as_str(), 32f32, 0)],
            110,
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

        let mut weather_area = Area::new(
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
        self.create_weather(&mut weather_area);

        left_column.add_sub_area(calendar_area);
        left_column.add_sub_area(weather_area);

        total.add_sub_area(left_column);
        total.add_sub_area(right_column);

        total.draw(&mut image);

        image
    }

    fn get_change_bbox(&self, current: &EpdImage) -> Option<Rect> {
        if let Some(previous) = &self.previous_frame {
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
            debug!("change bbox: {:?}", bbox);
        }

        current.to_img_file("output.png");
        current.to_file("output.bin");

        self.previous_frame = Some(current)
    }

    pub fn play_video(&mut self) {
        const FRAMES_PATH: &str = "./bad_apple/";
        let mut frame_paths = fs::read_dir(FRAMES_PATH)
            .expect("Could not read dir")
            .filter(|e| {
                e.as_ref().unwrap().file_type().unwrap().is_file()
                    && e.as_ref()
                        .unwrap()
                        .file_name()
                        .to_str()
                        .unwrap()
                        .ends_with(".bmp")
            })
            .map(|et| et.unwrap().file_name().to_str().unwrap().to_string())
            .collect::<Vec<String>>();
        frame_paths.sort();

        const FRAME_WIDTH: usize = 480 / 2;
        const FRAME_HEIGHT: usize = 360 / 2;

        let client = reqwest::blocking::Client::new();
        for path in frame_paths.iter().skip(100) {
            let path = FRAMES_PATH.to_string() + path.as_str();
            println!("playing {:?}", &path);
            let frame = image::open(path)
                .expect("Could not load image")
                .resize_exact(
                    FRAME_WIDTH as u32,
                    FRAME_HEIGHT as u32,
                    imageops::FilterType::Nearest,
                );
            let mut img = EpdImage::new(EPD_WIDTH, EPD_HEIGHT);
            let mut whole = Area::new(
                0,
                0,
                EPD_WIDTH,
                EPD_HEIGHT,
                Color::White,
                Padding::full(0),
                Outline::none(),
            );
            whole.load_image(0, 0, &frame);
            whole.draw(&mut img);

            let abc = img.to_partial(0, 0, FRAME_WIDTH, FRAME_HEIGHT);
            // Send partial update
            let form = multipart::Form::new().part(
                "data",
                multipart::Part::bytes(abc)
                    .file_name("file")
                    .mime_str("application/octet-stream")
                    .expect("Could not create multiform data"),
            );

            let response = client
                .post(
                    format!(
                        "http://192.168.178.61/direct_image_partial?rect=0,0,{},{}",
                        FRAME_WIDTH, FRAME_HEIGHT
                    )
                    .as_str(),
                )
                .multipart(form)
                .send()
                .expect("Could not send request");
            println!("{:?} {:?}", response.status(), response.text());

            // thread::sleep(Duration::from_millis(50));
        }
    }
}
