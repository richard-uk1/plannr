use std::env;
use std::sync::Arc;

use anyhow::Result;
use plannr::data::Event;
use plannr::db::get_events;
use sqlx::SqlitePool;
use xilem::core::fork;
use xilem::masonry::peniko::color::AlphaColor;
use xilem::style::{Padding, Style};
use xilem::view::{
    Axis, CrossAxisAlignment, FlexExt, MainAxisAlignment, flex, label, sized_box, task_raw,
};
use xilem::{EventLoop, FontWeight, LineBreaking, WidgetView, WindowOptions, Xilem};

struct State {
    pool: Arc<SqlitePool>,
    events: Vec<Event>,
    year: i32,
    iso_week: u8,
}

impl State {
    fn new(pool: Arc<SqlitePool>) -> Result<Self> {
        Ok(Self {
            pool,
            events: vec![],
            year: 2025,
            iso_week: 27,
        })
    }
}

fn event_view(event: &Event, alt_row: bool) -> impl WidgetView<State> + use<> {
    flex((
        sized_box(label(event.label.clone()).line_break_mode(LineBreaking::WordWrap)).width(100.),
        sized_box(label(event.calendar_id.to_string()).line_break_mode(LineBreaking::WordWrap))
            .width(100.),
        sized_box(label(event.interval.to_string()).line_break_mode(LineBreaking::WordWrap))
            .width(1000.),
    ))
    .direction(Axis::Horizontal)
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .background_color(if alt_row {
        AlphaColor::WHITE.with_alpha(0.2)
    } else {
        AlphaColor::TRANSPARENT
    })
    .padding(4.)
}

fn week_view(data: &mut State) -> impl WidgetView<State> + use<> {
    // ISO weeks start on monday
    flex((
        label("Monday").flex(1.),
        label("Tuesday").flex(1.),
        label("Wednesday").flex(1.),
        label("Thursday").flex(1.),
        label("Friday").flex(1.),
        label("Saturday").flex(1.),
        label("Sunday").flex(1.),
    ))
    .direction(Axis::Horizontal)
}

fn app_logic(data: &mut State) -> impl WidgetView<State> + use<> {
    week_view(data)
}

fn app_logic_bak(data: &mut State) -> impl WidgetView<State> + use<> {
    let pool = data.pool.clone();
    fork(
        flex((
            flex((
                sized_box(label("Event name").weight(FontWeight::BOLD)).width(100.),
                sized_box(label("Calendar ID").weight(FontWeight::BOLD)).width(100.),
                sized_box(label("Date/Time").weight(FontWeight::BOLD)).width(1000.),
            ))
            .direction(Axis::Horizontal)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .padding(Padding::from_vh(8., 4.)),
            data.events
                .iter()
                .enumerate()
                .map(|(idx, evt)| event_view(evt, idx % 2 == 1))
                .collect::<Vec<_>>(),
        ))
        .gap(4.)
        .main_axis_alignment(MainAxisAlignment::Center),
        task_raw(
            move |proxy| {
                let pool = pool.clone();
                async move {
                    let mut conn = pool.acquire().await.unwrap();
                    let events = get_events(None, &mut *conn).await.unwrap();
                    let _ = proxy.message(events);
                }
            },
            |state: &mut State, msg| {
                state.events = msg;
            },
        ),
    )
}

fn main() -> Result<()> {
    dotenv::dotenv()?;
    // Runtime must be initialized before call to `connect_lazy`
    let rt = Arc::new(tokio::runtime::Runtime::new()?);
    let rt_xilem = rt.clone();
    let pool = rt.block_on(async move {
        Ok::<_, anyhow::Error>(Arc::new(SqlitePool::connect_lazy(&env::var(
            "DATABASE_URL",
        )?)?))
    })?;
    let app = Xilem::new_simple_with_tokio(
        State::new(pool.clone())?,
        app_logic,
        WindowOptions::new("Calendar App"),
        rt_xilem,
    );

    app.run_in(EventLoop::with_user_event())?;
    Ok(())
}
