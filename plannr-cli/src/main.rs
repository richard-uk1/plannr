use std::env;

use anyhow::{Context, Result, bail};
use camino::Utf8Path;
use clap::Parser;
use cli_table::{WithTitle, print_stdout};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EmptyExtraTokenFields,
    EndpointNotSet, EndpointSet, PkceCodeChallenge, RedirectUrl, RevocationUrl, Scope,
    StandardTokenResponse, TokenResponse, TokenUrl,
    basic::{BasicClient, BasicTokenType},
};
use plannr::{data::EventInterval, db, google_creds::GoogleCreds};
use reqwest::{Url, redirect::Policy};
use sqlx::{SqliteConnection, SqlitePool, query};
use time::{
    Date, Month, UtcDateTime, format_description::BorrowedFormatItem, macros::format_description,
};
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncWriteExt, BufStream},
    net::TcpListener,
};
use tracing_subscriber::EnvFilter;

#[derive(Debug, clap::Parser)]
struct Args {
    #[clap(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, clap::Parser)]
enum Cmd {
    /// Clear database (warning destroys all data)
    ClearDb,
    /// Create some entries in the tables for testing
    InitFixtures,
    /// List all calendars
    ListCalendars,
    /// Create a new calendar
    CreateCalendar { name: String },
    /// List all events
    ListEvents {
        /// Fetch events for a specific calendar (by ID)
        #[clap(long)]
        calendar_id: Option<i64>,
        /// Fetch events for a specific calendar
        #[clap(short, long)]
        calendar: Option<String>,
    },
    /// Create a new event
    CreateEvent {
        calendar_id: i64,
        label: String,
        start_time: String,
        end_time: String,
    },
    /// Get google events through CalDAV
    DisplayGoogle,
}

const DATE_DESC: &[BorrowedFormatItem<'_>] = format_description!("[year]-[month]-[day]");
const DATETIME_DESC: &[BorrowedFormatItem<'_>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]");

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    // we load env vars before setting up logging, so just use main return
    dotenv::dotenv()?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    if let Err(e) = match args.cmd {
        Cmd::ClearDb => clear_database().await,
        Cmd::InitFixtures => init_fixtures(true).await,
        Cmd::ListCalendars => list_calendars().await,
        Cmd::CreateCalendar { name } => create_calendar(name).await,
        Cmd::ListEvents {
            calendar_id,
            calendar,
        } => list_events(calendar_id, calendar.as_deref()).await,
        Cmd::CreateEvent {
            calendar_id,
            label,
            start_time,
            end_time,
        } => create_event(calendar_id, label, start_time, end_time).await,
        Cmd::DisplayGoogle => display_google_events().await,
    } {
        tracing::error!("{e:?}");
        std::process::exit(1);
    }
    Ok(())
}

async fn clear_database() -> Result<()> {
    let pool = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;
    let mut conn = pool.acquire().await?;
    query!("DELETE FROM events").execute(&mut *conn).await?;
    query!("DELETE FROM calendars").execute(&mut *conn).await?;
    Ok(())
}

async fn init_fixtures(reset_database: bool) -> Result<()> {
    if reset_database {
        // could share pool but who cares its fast anyway
        clear_database().await?;
    }
    let pool = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;
    let mut conn = pool.acquire().await?;
    let fst_calendar = db::new_calendar("first test calendar", &mut *conn).await?;
    let snd_calendar = db::new_calendar("second test calendar", &mut *conn).await?;
    let interval = EventInterval::new_date(
        // Unwrap: we control input so can't fail
        Date::parse("2025-07-04", DATE_DESC).unwrap(),
        Date::parse("2025-07-06", DATE_DESC).unwrap(),
    )?;
    db::new_event(fst_calendar.id, "multiday event 1", interval, &mut *conn).await?;
    let interval = EventInterval::new_datetime(
        Date::from_calendar_date(2025, Month::July, 3)
            .unwrap()
            .with_hms(10, 0, 0)
            .unwrap()
            .as_utc(),
        Date::from_calendar_date(2025, Month::July, 3)
            .unwrap()
            .with_hms(10, 30, 0)
            .unwrap()
            .as_utc(),
    )?;
    db::new_event(fst_calendar.id, "event 1", interval, &mut *conn).await?;
    db::new_event(snd_calendar.id, "event 1", interval, &mut *conn).await?;
    let interval = EventInterval::new_datetime(
        UtcDateTime::parse("2025-07-03 10:45", DATETIME_DESC).unwrap(),
        UtcDateTime::parse("2025-07-03 11:00", DATETIME_DESC).unwrap(),
    )?;
    db::new_event(fst_calendar.id, "event 2", interval, &mut *conn).await?;
    Ok(())
}

async fn create_calendar(name: String) -> Result<()> {
    let pool = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;
    let mut conn = pool.acquire().await?;
    let calendar = db::new_calendar(&name, &mut *conn).await?;
    print_stdout(vec![calendar].with_title())?;
    Ok(())
}

async fn list_calendars() -> Result<()> {
    let pool = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;
    let mut conn = pool.acquire().await?;
    let calendars = db::get_calendars(&mut *conn).await?;
    print_stdout(calendars.with_title())?;
    Ok(())
}

async fn list_events(calendar_id: Option<i64>, calendar: Option<&str>) -> Result<()> {
    let pool = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;
    let mut conn = pool.acquire().await?;
    let calendar_id = match (calendar_id, calendar) {
        (None, None) => None,
        (None, Some(calendar)) => {
            let calendar = db::find_calendar(calendar, &mut *conn).await?;
            Some(calendar.id)
        }
        (Some(calendar_id), None) => {
            // check calendar exists
            match db::get_calendar(calendar_id, &mut *conn).await? {
                Some(calendar) => Some(calendar.id),
                None => bail!("No calendar with ID `{calendar_id}`"),
            }
        }
        (Some(_), Some(_)) => {
            bail!("only one of `calendar_id` and `calendar` can be set ")
        }
    };
    let events = db::get_events(calendar_id, &mut *conn).await?;
    print_stdout(events.with_title())?;
    Ok(())
}

async fn create_event(
    calendar_id: i64,
    label: String,
    start_time: String,
    end_time: String,
) -> Result<()> {
    let date_desc = format_description!("[year]-[month]-[day]");
    let datetime_desc = format_description!("[year]-[month]-[day] [hour]:[minute]");
    let interval = if let Ok(start) = Date::parse(&start_time, date_desc) {
        // end must be date
        let end = Date::parse(&end_time, date_desc)?;
        EventInterval::new_date(start, end)
    } else {
        // try datetime
        let start = UtcDateTime::parse(&start_time, datetime_desc)?;
        let end = UtcDateTime::parse(&end_time, datetime_desc)?;
        EventInterval::new_datetime(start, end)
    }?;
    let pool = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;
    let mut conn = pool.acquire().await?;
    let calendar = db::new_event(calendar_id, &label, interval, &mut *conn).await?;
    print_stdout(vec![calendar].with_title())?;
    Ok(())
}

async fn display_google_events() -> Result<()> {
    let pool = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;
    let mut conn = pool.acquire().await?;
    let http_client = reqwest::ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(Policy::none())
        .build()
        .expect("Client should build");

    let google_oauth_tok = google_oauth_token(&http_client, &mut *conn).await?;

    let calendar_id = env::var("GOOGLE_USERNAME")?;
    let address = format!("https://apidata.googleusercontent.com/caldav/v2/{calendar_id}/events");

    let req = http_client
        .get(Url::parse(&address).unwrap())
        .bearer_auth(google_oauth_tok.access_token().secret());
    let res_head = req.send().await?;
    let calendar = res_head.text().await?;
    fs::write("calendar.txt", &calendar).await?;
    Ok(())
}

async fn google_oauth_token(
    http_client: &reqwest::Client,
    exec: &mut SqliteConnection,
) -> Result<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>> {
    if let Some(token) = db::google_token(&mut *exec).await? {
        return Ok(token);
    }

    let client = google_oauth_client(Utf8Path::new("google_creds.json"))?;

    // Generate a PKCE challenge.
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Generate the full authorization URL.
    let (auth_url, csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        // Set the desired scopes.
        // Requesting access to the "calendar" features and the user's profile.
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/calendar.readonly".to_string(),
        ))
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/userinfo.email".to_string(),
        ))
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/userinfo.profile".to_string(),
        ))
        .set_pkce_challenge(pkce_challenge)
        .url();

    // This is the URL you should redirect the user to, in order to trigger the authorization
    // process.
    println!("Browse to: {}", auth_url);

    // Once the user has been redirected to the redirect URL, you'll have access to the
    // authorization code. For security reasons, your code should verify that the `state`
    // parameter returned by the server matches `csrf_token`.

    let (code, state) = {
        // A very naive implementation of the redirect server.
        let listener = TcpListener::bind("127.0.0.1:8080").await?;

        // The server will terminate itself after collecting the first code.
        let (stream, _addr) = listener.accept().await?;
        let mut stream = BufStream::new(stream);

        let mut request_line = String::new();
        stream.read_line(&mut request_line).await?;

        let redirect_url = request_line.split_whitespace().nth(1).unwrap();
        let url = Url::parse(&("http://localhost:8080".to_string() + redirect_url))?;

        let code = url
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, code)| AuthorizationCode::new(code.into_owned()))
            .context("no 'auth code' in redirect request")?;

        let state = url
            .query_pairs()
            .find(|(key, _)| key == "state")
            .map(|(_, state)| CsrfToken::new(state.into_owned()))
            .context("no 'state' in redirect request")?;

        let message = "Go back to your terminal :)";
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
            message.len(),
            message
        );
        stream.write_all(response.as_bytes()).await?;

        // server is dropped/shut down here
        (code, state)
    };

    // TODO is this already checked in `set_pkce_verifier`?
    if state.secret() != csrf_state.secret() {
        bail!("state token did not match one we created");
    }

    let token = client
        .exchange_code(code)
        // Set the PKCE code verifier.
        .set_pkce_verifier(pkce_verifier)
        .request_async(http_client)
        .await?;

    db::store_google_token(&env::var("GOOGLE_USERNAME")?, token.clone(), exec).await?;
    Ok(token)
}

fn google_oauth_client(
    creds_path: &Utf8Path,
) -> anyhow::Result<
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointSet, EndpointSet>,
> {
    let creds = GoogleCreds::from_file(creds_path)?;
    Ok(BasicClient::new(ClientId::new(creds.client_id))
        .set_client_secret(ClientSecret::new(creds.client_secret))
        .set_auth_uri(AuthUrl::new(creds.auth_uri)?)
        .set_token_uri(TokenUrl::new(creds.token_uri)?)
        // Set the URL the user will be redirected to after the authorization process.
        .set_redirect_uri(RedirectUrl::new("http://localhost:8080".to_string())?)
        .set_revocation_url(
            RevocationUrl::new("https://oauth2.googleapis.com/revoke".to_string()).unwrap(),
        ))
}
