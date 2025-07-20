use std::env;
use std::sync::Arc;

use anyhow::Result;
use plannr::data::Event;
use plannr::db::get_events;
use sqlx::SqlitePool;
use xilem::core::fork;
use xilem::view::{Axis, MainAxisAlignment, flex, label, task_raw};
use xilem::{EventLoop, FontWeight, WidgetView, WindowOptions, Xilem};

struct State {
    pool: Arc<SqlitePool>,
    events: Vec<Event>,
}

impl State {
    fn new(pool: Arc<SqlitePool>) -> Result<Self> {
        Ok(Self {
            pool,
            events: vec![],
        })
    }
}

fn event_view(event: &Event) -> impl WidgetView<State> + use<> {
    flex((
        label(event.label.clone()),
        label(event.calendar_id.to_string()).weight(FontWeight::BOLD),
        label(event.interval.to_string()),
    ))
    .direction(Axis::Horizontal)
}

fn app_logic(data: &mut State) -> impl WidgetView<State> + use<> {
    let pool = data.pool.clone();
    fork(
        flex(data.events.iter().map(event_view).collect::<Vec<_>>())
            .gap(4.)
            .main_axis_alignment(MainAxisAlignment::Center),
        task_raw(
            move |proxy| {
                let pool = pool.clone();
                async move {
                    let mut conn = pool.acquire().await.unwrap();
                    let events = get_events(&mut *conn).await.unwrap();
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
    let rt = Arc::new(tokio::runtime::Runtime::new()?);
    let rt_xilem = rt.clone();
    rt.block_on(async move {
        let pool = Arc::new(SqlitePool::connect_lazy(&env::var("DATABASE_URL")?)?);
        let app = Xilem::new_simple_with_tokio(
            State::new(pool.clone())?,
            app_logic,
            WindowOptions::new("Calendar App"),
            rt_xilem,
        );

        app.run_in(EventLoop::with_user_event())?;
        Ok(())
    })
}
