use crate::calendar::Time::{AllDay, Timed};
use std::ops::Add;

#[derive(Debug)]
pub enum Time {
    AllDay(chrono::NaiveDate),
    Timed(chrono::DateTime<chrono::Utc>, chrono::TimeDelta),
}

#[derive(Debug)]
pub struct Event {
    pub time: Time,
    pub title: String,
}

pub struct CalendarHandler {}

impl CalendarHandler {
    pub fn new() -> Self {
        CalendarHandler {}
    }
    pub fn fetch(&self) -> Vec<Event> {
        vec![
            Event {
                title: "Test All Day!".to_string(),
                time: AllDay(chrono::Local::now().date_naive()),
            },
            Event {
                title: "OOOOOOOOOOOOOOO".to_string(),
                time: AllDay(
                    chrono::Local::now()
                        .add(chrono::TimeDelta::days(1))
                        .date_naive(),
                ),
            },
            Event {
                title: "VVVVVVVVVVVVVVVVVVVVVV".to_string(),
                time: AllDay(
                    chrono::Local::now()
                        .add(chrono::TimeDelta::days(1))
                        .date_naive(),
                ),
            },
            Event {
                title: "Test Timed!".to_string(),
                time: Timed(
                    chrono::Local::now()
                        .add(chrono::TimeDelta::days(2))
                        .with_timezone(&chrono::Utc),
                    chrono::TimeDelta::minutes(63),
                ),
            },
            Event {
                title: "locking in..".to_string(),
                time: Timed(
                    chrono::Local::now().with_timezone(&chrono::Utc),
                    chrono::TimeDelta::minutes(63),
                ),
            },
            Event {
                title: "crashing out..".to_string(),
                time: Timed(
                    chrono::Local::now()
                        .with_timezone(&chrono::Utc)
                        .add(chrono::TimeDelta::minutes(30)),
                    chrono::TimeDelta::minutes(63),
                ),
            },
        ]
    }
}
