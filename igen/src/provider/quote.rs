use crate::settings::QuoteConfig;
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use std::collections::VecDeque;
use std::fs;

#[derive(Deserialize, Clone)]
pub struct Quote {
    pub content: String,
    pub author: String,
    pub tags: Vec<String>,
}

pub struct QuoteProvider {
    quote_config: QuoteConfig,
    quotes: VecDeque<Quote>,
    cache: Option<(DateTime<Utc>, Quote)>,
}

impl QuoteProvider {
    fn load_quotes(&mut self) {
        let quotes = serde_json::from_str::<Vec<Quote>>(
            fs::read_to_string(&self.quote_config.quotes_path)
                .expect("Could not read quotes path")
                .as_str(),
        )
        .expect("Could not deserialize quotes");
        self.quotes = VecDeque::from(quotes);
    }

    pub fn get_quote(&mut self) -> Quote {
        if let Some((last_refresh, last_quote)) = self.cache.as_ref() {
            if Utc::now().signed_duration_since(last_refresh) > Duration::minutes(15) {
            } else {
                return last_quote.clone();
            }
        }

        match self.quotes.pop_front() {
            None => {
                self.load_quotes();
                if self.quotes.is_empty() {
                    panic!("no quotes");
                }
                self.get_quote()
            }
            Some(q) => {
                self.cache = Some((Utc::now(), q.clone()));
                q
            }
        }
    }

    pub fn new(quote_config: QuoteConfig) -> QuoteProvider {
        QuoteProvider {
            quote_config,
            quotes: VecDeque::new(),
            cache: None,
        }
    }
}
