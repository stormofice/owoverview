use crate::calendar::Time::{AllDay, Timed};
use crate::settings::GoogleConfig;

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
    Timed(chrono::DateTime<chrono::Utc>, chrono::TimeDelta),
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

pub struct CalendarHandler {
    google_config: GoogleConfig,
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
        println!("{:?}", &value);
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
            .expect("Could not parse event start time")
            .to_utc();
            let et = chrono::DateTime::parse_from_rfc3339(
                value
                    .end
                    .date_time
                    .expect(
                        "Timed \
            Event must have end time",
                    )
                    .as_str(),
            )
            .expect("Could not parse event end time")
            .to_utc();
            Timed(st, et.signed_duration_since(st))
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
        BasicClient::new(ClientId::new($self.google_config.client_id.clone()))
            .set_client_secret(ClientSecret::new($self.google_config.client_secret.clone()))
            .set_auth_uri(
                AuthUrl::new($self.google_config.auth_uri.clone())
                    .expect("Could not construct auth uri"),
            )
            .set_redirect_uri(
                RedirectUrl::new($self.google_config.redirect_uri.clone())
                    .expect("Could not construct redirect uri"),
            )
            .set_token_uri(
                TokenUrl::new($self.google_config.token_uri.clone())
                    .expect("Could not construct token uri"),
            )
    };
}

impl CalendarHandler {
    pub fn new(google_config: GoogleConfig) -> Self {
        let cl = CalendarHandler {
            google_config,
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

        let (authorize_url, csrf_state) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/calendar".to_string(),
            ))
            .set_pkce_challenge(pkce_code_challenge)
            .url();

        println!("Open this URL in your browser:\n{authorize_url}\n");

        let (code, state) = {
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

            let message = "Go back to your terminal :)";
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
                message.len(),
                message
            );
            stream.write_all(response.as_bytes()).unwrap();

            (code, state)
        };

        println!("Google returned the following code:\n{}\n", code.secret());
        println!(
            "Google returned the following state:\n{} (expected `{}`)\n",
            state.secret(),
            csrf_state.secret()
        );

        // Exchange the code with a token.
        let token_response = client
            .exchange_code(code)
            .set_pkce_verifier(pkce_code_verifier)
            .request(&self.http_client)
            .expect("Could not get token response");

        println!("Google returned the following token:\n{token_response:?}\n");

        token_response
    }

    fn load_or_refresh_token(&self) -> String {
        if Path::new(&self.google_config.token_path).exists() {
            let token_str = fs::read_to_string(&self.google_config.token_path).expect("");
            let stored_token: StoredToken = serde_json::from_str(&token_str).expect("");

            let now = chrono::Utc::now();
            let is_expired = match stored_token.expires_at {
                Some(expiry) => now >= expiry,
                None => true,
            };

            if is_expired {
                if let Some(refresh_token_str) = stored_token.refresh_token {
                    println!("Access token expired. Refreshing...");

                    let refresh_token = RefreshToken::new(refresh_token_str);
                    let oauth_client = create_oauth_client!(self);
                    let token_response = oauth_client
                        .exchange_refresh_token(&refresh_token)
                        .request(&self.http_client)
                        .expect("Could not exchange refresh token");
                    self.store_token(&token_response);
                    token_response.access_token().secret().clone()
                } else {
                    println!("No refresh token in file. Authenticating...");
                    let tok = self.authenticate();
                    self.store_token(&tok);
                    tok.access_token().secret().clone()
                }
            } else {
                println!("Using existing token");
                stored_token.access_token
            }
        } else {
            println!("no token file, authenticating");
            let tok = self.authenticate();
            self.store_token(&tok);
            tok.access_token().secret().clone()
        }
    }

    fn store_token(&self, token_response: &BasicTokenResponse) {
        let stored_token = StoredToken {
            access_token: token_response.access_token().secret().to_string(),
            refresh_token: token_response
                .refresh_token()
                .map(|rt| rt.secret().to_string()),
            expires_at: token_response
                .expires_in()
                .map(|d| chrono::Utc::now().add(d)),
        };
        fs::write(
            Path::new(&self.google_config.token_path),
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
            .filter(|cal| self.google_config.calendar_list.contains(&cal.summary))
            .for_each(|cal| {
                let id = &cal.id;
                for event in self.fetch_events_for_calendar(id) {
                    println!("{:?}", &event);
                    combined_events.push(event);
                }
            });

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
        self.retrieve_calendar_events()
    }
}
