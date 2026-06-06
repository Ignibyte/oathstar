use std::{convert::Infallible, net::SocketAddr, sync::Arc, time::Duration};

use async_stream::stream;
use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
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
    let engine = Engine::try_new(world)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use oathstar_protocol::{EventChannel, OutputComponent};

    #[test]
    fn escape_html_neutralizes_markup() {
        let out = escape_html(r#"<script>alert("x&y")</script>'"#);
        assert!(!out.contains('<'), "no raw < survives");
        assert!(!out.contains('>'), "no raw > survives");
        assert_eq!(
            out,
            "&lt;script&gt;alert(&quot;x&amp;y&quot;)&lt;/script&gt;&#39;"
        );
    }

    #[test]
    fn escape_html_escapes_ampersand_first() {
        // & must be escaped before the entities it introduces, else the output
        // double-escapes; pin every replacement so a dropped one fails here.
        assert_eq!(escape_html("a&b"), "a&amp;b");
        assert_eq!(escape_html("<>"), "&lt;&gt;");
        assert_eq!(escape_html("\"'"), "&quot;&#39;");
        assert_eq!(escape_html("plain text"), "plain text");
    }

    #[tokio::test]
    async fn root_serves_the_status_line() {
        assert_eq!(
            root().await,
            "Oathstar server is running. Try GET /state, POST /command, or GET /events."
        );
    }

    #[test]
    fn render_event_html_escapes_log_messages() {
        let event = GameEvent {
            event_id: 7,
            tick: 3,
            channel: EventChannel::Narrative,
            kind: GameEventKind::LogMessage {
                component: OutputComponent::NarrativeMessage,
                text: "<b>hi</b>".to_string(),
            },
        };
        let html = render_event_html(&event);
        assert!(html.contains(r#"data-event-id="7""#));
        assert!(html.contains("&lt;b&gt;hi&lt;/b&gt;"));
        assert!(!html.contains("<b>hi"));
    }

    #[test]
    fn render_event_html_renders_tick_and_room_entered() {
        let tick = GameEvent {
            event_id: 1,
            tick: 1,
            channel: EventChannel::Debug,
            kind: GameEventKind::Tick { value: 9 },
        };
        let tick_html = render_event_html(&tick);
        assert!(tick_html.contains(r#"class="tick""#));
        assert!(tick_html.contains(r#"data-tick="9""#));

        let room = GameEvent {
            event_id: 2,
            tick: 1,
            channel: EventChannel::Room,
            kind: GameEventKind::RoomEntered {
                room_id: "r1".to_string(),
                title: "Hall <x>".to_string(),
            },
        };
        let room_html = render_event_html(&room);
        assert!(room_html.contains(r#"data-room-id="r1""#));
        assert!(room_html.contains("Entered Hall &lt;x&gt;"));
    }

    #[tokio::test]
    async fn spawn_tick_loop_broadcasts_ticks() {
        let world = oathstar_content::load_beginner_world().expect("beginner world loads");
        let (events, _initial_rx) = broadcast::channel(16);
        let state = AppState {
            engine: Arc::new(Mutex::new(
                Engine::try_new(world).expect("valid beginner world"),
            )),
            events: events.clone(),
        };
        let mut rx = events.subscribe();
        spawn_tick_loop(state);
        let event = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("a tick is broadcast within 2s")
            .expect("broadcast channel stays open");
        assert!(matches!(event.kind, GameEventKind::Tick { .. }));
    }

    fn test_app_state() -> AppState {
        let world = oathstar_content::load_beginner_world().expect("beginner world loads");
        let (events, _rx) = broadcast::channel(16);
        AppState {
            engine: Arc::new(Mutex::new(
                Engine::try_new(world).expect("valid beginner world"),
            )),
            events,
        }
    }

    #[tokio::test]
    async fn health_reports_ok() {
        let body = health().await;
        assert_eq!(body.0["ok"], true);
        assert_eq!(body.0["service"], "oathstar-server");
    }

    #[tokio::test]
    async fn state_snapshot_returns_engine_state() {
        let snapshot = state_snapshot(State(test_app_state())).await;
        assert_eq!(snapshot.0.current_room_id, "hollowmere_square");
    }

    #[tokio::test]
    async fn command_processes_and_broadcasts() {
        let app = test_app_state();
        let mut rx = app.events.subscribe();
        let response = command(
            State(app),
            Json(CommandRequest {
                input: "look".to_string(),
                actor_id: None,
            }),
        )
        .await;
        assert!(response.0.accepted, "a known command is accepted");
        assert!(!response.0.events.is_empty(), "the command emits events");
        rx.try_recv()
            .expect("the command's events are broadcast to subscribers");
    }
}
