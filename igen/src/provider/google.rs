use crate::provider::google::Time::{AllDay, Timed};
use crate::settings::Config;
use std::cmp::Ordering;

use log::{debug, warn};
use oauth2::basic::{BasicClient, BasicTokenResponse};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
    RefreshToken, Scope, TokenResponse, TokenUrl, reqwest,
};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::ops::Add;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug)]
pub enum Time {
    AllDay(chrono::NaiveDate),
    Timed(chrono::DateTime<chrono::Local>, chrono::TimeDelta),
}

impl PartialEq for Time {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (AllDay(f), AllDay(s)) => f == s,
            (Timed(f, df), Timed(s, ds)) => f == s && df == ds,
            (_, _) => false,
        }
    }
}

impl Eq for Time {}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Time {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (AllDay(f), AllDay(s)) => f.cmp(s),
            (Timed(f, _), Timed(s, _)) => f.cmp(s),
            // "AllDays" are always before timed events
            (AllDay(date1), Timed(start2, _)) => {
                let date2 = start2.date_naive();
                match date1.cmp(&date2) {
                    Ordering::Equal => Ordering::Less,
                    ordering => ordering,
                }
            }
            (Timed(start1, _), AllDay(date2)) => {
                let date1 = start1.date_naive();
                match date1.cmp(date2) {
                    Ordering::Equal => Ordering::Greater,
                    ordering => ordering,
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct Event {
    pub time: Time,
    pub title: String,
}

#[derive(Serialize, Deserialize)]
struct StoredToken {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct CalendarProvider {
    config: Config,
    http_client: reqwest::blocking::Client,
    calendar_list: Option<CalendarListResponse>,
}

#[derive(Debug, Deserialize)]
struct EventsResponse {
    items: Vec<GoogleEvent>,
}

#[derive(Debug, Deserialize)]
struct GoogleEvent {
    id: String,
    summary: String,
    start: GoogleEventDateTime,
    end: GoogleEventDateTime,
}

impl From<GoogleEvent> for Event {
    fn from(value: GoogleEvent) -> Self {
        // if there is a date, it is an all-day event
        let time = if let Some(start_date) = value.start.date {
            AllDay(chrono::NaiveDate::from_str(&start_date).expect("Could not parse date"))
        } else {
            let st = chrono::DateTime::parse_from_rfc3339(
                value
                    .start
                    .date_time
                    .expect("Timed Event must have start time")
                    .as_str(),
            )
            .expect("Could not parse event start time");
            let et = chrono::DateTime::parse_from_rfc3339(
                value
                    .end
                    .date_time
                    .expect("Timed Event must have end time")
                    .as_str(),
            )
            .expect("Could not parse event end time");
            Timed(chrono::DateTime::from(st), et.signed_duration_since(st))
        };
        Event {
            title: value.summary,
            time,
        }
    }
}

#[derive(Debug, Deserialize)]
struct GoogleEventDateTime {
    #[serde(default, rename(deserialize = "dateTime"))]
    date_time: Option<String>,
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CalendarListResponse {
    items: Vec<CalendarListEntry>,
}

#[derive(Debug, Deserialize)]
struct CalendarListEntry {
    id: String,
    summary: String,
}

macro_rules! create_oauth_client {
    ($self:expr) => {
        BasicClient::new(ClientId::new($self.config.google.client_id.clone()))
            .set_client_secret(ClientSecret::new($self.config.google.client_secret.clone()))
            .set_auth_uri(
                AuthUrl::new($self.config.google.auth_uri.clone())
                    .expect("Could not construct auth uri"),
            )
            .set_redirect_uri(
                RedirectUrl::new($self.config.google.redirect_uri.clone())
                    .expect("Could not construct redirect uri"),
            )
            .set_token_uri(
                TokenUrl::new($self.config.google.token_uri.clone())
                    .expect("Could not construct token uri"),
            )
    };
}

impl CalendarProvider {
    pub fn new(config: Config) -> Self {
        let cl = CalendarProvider {
            config,
            http_client: reqwest::blocking::ClientBuilder::new()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("HTTP client could not be constructed"),
            calendar_list: None,
        };
        cl.load_or_refresh_token();
        cl
    }

    fn authenticate(&self) -> BasicTokenResponse {
        let client = create_oauth_client!(self);

        let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();

        let (authorize_url, _) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/calendar".to_string(),
            ))
            .set_pkce_challenge(pkce_code_challenge)
            .url();

        println!("Open this URL in your browser:\n{authorize_url}\n");

        let (code, _) = {
            // A very naive implementation of the redirect server.
            let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

            // The server will terminate itself after collecting the first code.
            let Some(mut stream) = listener.incoming().flatten().next() else {
                panic!("listener terminated without accepting a connection");
            };

            let mut reader = BufReader::new(&stream);

            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();

            let redirect_url = request_line.split_whitespace().nth(1).unwrap();
            let url = Url::parse(&("http://localhost".to_string() + redirect_url)).unwrap();

            let code = url
                .query_pairs()
                .find(|(key, _)| key == "code")
                .map(|(_, code)| AuthorizationCode::new(code.into_owned()))
                .unwrap();

            let state = url
                .query_pairs()
                .find(|(key, _)| key == "state")
                .map(|(_, state)| CsrfToken::new(state.into_owned()))
                .unwrap();

            let message = "Authentication successful. You can safely go back to your terminal :^)";
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
                message.len(),
                message
            );
            stream.write_all(response.as_bytes()).unwrap();

            (code, state)
        };

        // Exchange the code with a token.
        client
            .exchange_code(code)
            .set_pkce_verifier(pkce_code_verifier)
            .request(&self.http_client)
            .expect("Could not get token response")
    }

    fn load_or_refresh_token(&self) -> String {
        if Path::new(&self.config.google.token_path).exists() {
            let token_str = fs::read_to_string(&self.config.google.token_path).expect("");
            let stored_token: StoredToken = serde_json::from_str(&token_str).expect("");

            let now = chrono::Utc::now();
            let is_expired = match stored_token.expires_at {
                Some(expiry) => now >= expiry,
                None => true,
            };

            if is_expired {
                if let Some(refresh_token_str) = &stored_token.refresh_token {
                    debug!("Access token expired. Refreshing...");

                    let refresh_token = RefreshToken::new(refresh_token_str.clone());
                    let oauth_client = create_oauth_client!(self);
                    let token_response = oauth_client
                        .exchange_refresh_token(&refresh_token)
                        .request(&self.http_client)
                        .expect("Could not exchange refresh token");
                    self.store_token(&token_response, Some(&stored_token));
                    token_response.access_token().secret().clone()
                } else {
                    warn!("No refresh token in file. Authenticating...");
                    let tok = self.authenticate();
                    self.store_token(&tok, Some(&stored_token));
                    tok.access_token().secret().clone()
                }
            } else {
                debug!("Using existing token");
                stored_token.access_token
            }
        } else {
            debug!("Found no token file, authenticating");
            let tok = self.authenticate();
            self.store_token(&tok, None);
            tok.access_token().secret().clone()
        }
    }

    fn store_token(&self, token_response: &BasicTokenResponse, previous: Option<&StoredToken>) {
        let refresh_token: Option<&String> = if let Some(refresh) = token_response.refresh_token() {
            Some(refresh.secret())
        } else if let Some(prev) = previous {
            if let Some(refresh) = &prev.refresh_token {
                Some(refresh)
            } else {
                None
            }
        } else {
            None
        };

        let stored_token = StoredToken {
            access_token: token_response.access_token().secret().to_string(),
            refresh_token: refresh_token.cloned(),
            expires_at: token_response
                .expires_in()
                .map(|d| chrono::Utc::now().add(d)),
        };
        fs::write(
            Path::new(&self.config.google.token_path),
            serde_json::to_string_pretty(&stored_token).expect("Could not prettify token"),
        )
        .expect("Could not store token");
    }

    fn retrieve_calendar_events(&mut self) -> Vec<Event> {
        if self.calendar_list.is_none() {
            self.fetch_calenders()
        }

        let clr = self.calendar_list.as_ref().expect("Calendar list unset");

        let mut combined_events: Vec<Event> = vec![];

        clr.items
            .iter()
            .filter(|cal| self.config.google.calendar_list.contains(&cal.summary))
            .for_each(|cal| {
                let id = &cal.id;
                for event in self.fetch_events_for_calendar(id) {
                    combined_events.push(event);
                }
            });

        combined_events.sort_by(|f, s| f.time.cmp(&s.time));

        combined_events
    }

    fn fetch_events_for_calendar(&self, cal_id: &str) -> Vec<Event> {
        let events_url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events",
            cal_id
        );

        let gevents = self
            .http_client
            .get(events_url)
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.load_or_refresh_token()),
            )
            .query(&[
                (
                    "timeMin",
                    chrono::Local::now().to_utc().to_rfc3339().as_str(),
                ),
                ("singleEvents", "true"),
                ("orderBy", "startTime"),
                ("maxResults", "10"),
            ])
            .send()
            .expect("Could not send list calendar request");

        let gevents = gevents
            .json::<EventsResponse>()
            .unwrap_or_else(|_| panic!("Could not deserialize events response, cal: {}", cal_id));

        let mut events: Vec<Event> = vec![];
        for gevent in gevents.items {
            events.push(Event::from(gevent));
        }
        events
    }

    fn fetch_calenders(&mut self) {
        const LIST_CALENDARS: &str = "https://www.googleapis.com/calendar/v3/users/me/calendarList";

        let calenders = self
            .http_client
            .get(LIST_CALENDARS)
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.load_or_refresh_token()),
            )
            .send()
            .expect("Could not send list calendar request");

        let clr = calenders
            .json::<CalendarListResponse>()
            .expect("Could not deserialize calendars to json");
        self.calendar_list = Some(clr)
    }

    pub fn fetch(&mut self) -> Vec<Event> {
        if self.config.general.debug {
            vec![Event {
                time: AllDay(chrono::Local::now().date_naive()),
                title: "hehe".to_string(),
            }]
        } else {
            self.retrieve_calendar_events()
        }
    }
}
