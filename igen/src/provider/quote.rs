use crate::settings::QuoteConfig;
use serde::Deserialize;
use std::collections::VecDeque;
use std::fs;

#[derive(Deserialize)]
pub struct Quote {
    pub content: String,
    pub author: String,
    pub tags: Vec<String>,
}

pub struct QuoteProvider {
    quote_config: QuoteConfig,
    quotes: VecDeque<Quote>,
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
        match self.quotes.pop_front() {
            None => {
                self.load_quotes();
                if self.quotes.is_empty() {
                    panic!("no quotes");
                }
                self.get_quote()
            }
            Some(q) => q,
        }
    }

    pub fn new(quote_config: QuoteConfig) -> QuoteProvider {
        QuoteProvider {
            quote_config,
            quotes: VecDeque::new(),
        }
    }
}
