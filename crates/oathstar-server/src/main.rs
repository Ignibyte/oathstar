use std::{convert::Infallible, net::SocketAddr, sync::Arc, time::Duration};

use async_stream::stream;
use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
        Html,
    },
    routing::{get, post},
    Json, Router,
};
use oathstar_core::Engine;
use oathstar_protocol::{CommandRequest, CommandResponse, GameEvent, GameEventKind, GameSnapshot};
use tokio::{
    net::TcpListener,
    sync::{broadcast, Mutex},
    time,
};

#[derive(Clone)]
struct AppState {
    engine: Arc<Mutex<Engine>>,
    events: broadcast::Sender<GameEvent>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let world = oathstar_content::load_beginner_world()?;
    let engine = Engine::new(world);
    let (events, _) = broadcast::channel(256);

    let app_state = AppState {
        engine: Arc::new(Mutex::new(engine)),
        events,
    };

    spawn_tick_loop(app_state.clone());

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/state", get(state_snapshot))
        .route("/command", post(command))
        .route("/events", get(events_json))
        .route("/events/json", get(events_json))
        .route("/events/html", get(events_html))
        .with_state(app_state);

    let addr: SocketAddr = std::env::var("OATHSTAR_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:7878".to_string())
        .parse()?;
    let listener = TcpListener::bind(addr).await?;

    println!("oathstar-server listening on http://{addr}");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn root() -> &'static str {
    "Oathstar server is running. Try GET /state, POST /command, or GET /events."
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "ok": true,
        "service": "oathstar-server"
    }))
}

async fn state_snapshot(State(app): State<AppState>) -> Json<GameSnapshot> {
    let engine = app.engine.lock().await;
    Json(engine.snapshot())
}

async fn command(
    State(app): State<AppState>,
    Json(request): Json<CommandRequest>,
) -> Json<CommandResponse> {
    let mut engine = app.engine.lock().await;
    let response = engine.handle_command(request);
    let events = response.events.clone();
    drop(engine);

    for event in events {
        let _ = app.events.send(event);
    }

    Json(response)
}

async fn events_json(
    State(app): State<AppState>,
) -> Sse<impl futures_core::Stream<Item = Result<Event, Infallible>>> {
    let mut receiver = app.events.subscribe();

    let stream = stream! {
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    let Ok(data) = serde_json::to_string(&event) else {
                        continue;
                    };
                    yield Ok(Event::default()
                        .event("game_event")
                        .id(event.event_id.to_string())
                        .data(data));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

async fn events_html(
    State(app): State<AppState>,
) -> Sse<impl futures_core::Stream<Item = Result<Event, Infallible>>> {
    let mut receiver = app.events.subscribe();

    let stream = stream! {
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    let html = render_event_html(&event);
                    yield Ok(Event::default()
                        .event("game_event_html")
                        .id(event.event_id.to_string())
                        .data(html));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

fn spawn_tick_loop(app: AppState) {
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            let mut engine = app.engine.lock().await;
            let event = engine.tick();
            drop(engine);
            let _ = app.events.send(event);
        }
    });
}

fn render_event_html(event: &GameEvent) -> String {
    match &event.kind {
        GameEventKind::LogMessage { component, text } => format!(
            r#"<article class="message message-{channel}" data-event-id="{id}" data-component="{component:?}">{text}</article>"#,
            channel = format!("{:?}", event.channel).to_lowercase(),
            id = event.event_id,
            component = component,
            text = escape_html(text),
        ),
        GameEventKind::Tick { value } => format!(
            r#"<span class="tick" data-event-id="{}" data-tick="{}"></span>"#,
            event.event_id, value
        ),
        GameEventKind::RoomEntered { room_id, title } => format!(
            r#"<article class="message message-room" data-event-id="{}" data-room-id="{}">Entered {}</article>"#,
            event.event_id,
            escape_html(room_id),
            escape_html(title)
        ),
    }
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[allow(dead_code)]
async fn _html_preview(State(app): State<AppState>) -> Html<String> {
    let engine = app.engine.lock().await;
    let snapshot = engine.snapshot();
    Html(format!(
        "<h1>{}</h1><p>{}</p>",
        escape_html(&snapshot.room.title),
        escape_html(&snapshot.room.description)
    ))
}
