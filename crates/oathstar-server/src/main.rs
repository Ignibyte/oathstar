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
    /// The opening-scene events from `Engine::begin()`, captured once at startup
    /// and replayed at the head of every new `/events` subscription so a fresh
    /// client renders the start room without sending `look` first (REQ-001 /
    /// Decision 031: `try_new` emits nothing; `begin` is the on-start emitter).
    opening: Arc<Vec<GameEvent>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let world = oathstar_content::load_beginner_world()?;
    let mut engine = Engine::try_new(world)?;
    // Emit the opening scene once, up front, and keep it to seed each new
    // subscriber. begin() does not move the player, so /state stays consistent.
    let opening = Arc::new(engine.begin());
    let (events, _) = broadcast::channel(256);

    let app_state = AppState {
        engine: Arc::new(Mutex::new(engine)),
        events,
        opening,
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
    let opening = Arc::clone(&app.opening);
    let mut receiver = app.events.subscribe();

    let stream = stream! {
        // Seed the opening scene so a fresh subscriber renders the start room
        // without sending `look` first (REQ-001 / Decision 031).
        for event in opening.iter() {
            if let Some(data) = event_to_json(event) {
                yield Ok(Event::default()
                    .event("game_event")
                    .id(event.event_id.to_string())
                    .data(data));
            }
        }
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    let Some(data) = event_to_json(&event) else {
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
    let opening = Arc::clone(&app.opening);
    let mut receiver = app.events.subscribe();

    let stream = stream! {
        // Seed the opening scene as HTML fragments for a fresh subscriber
        // (REQ-001 / Decision 031).
        for event in opening.iter() {
            yield Ok(Event::default()
                .event("game_event_html")
                .id(event.event_id.to_string())
                .data(render_event_html(event)));
        }
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

/// Serialize a [`GameEvent`] to its JSON wire form for the `/events` SSE stream.
/// A named seam — used by both the opening-scene seed and the live broadcast
/// loop — so the snake/camel wire split of Decision 031 is unit-testable.
fn event_to_json(event: &GameEvent) -> Option<String> {
    serde_json::to_string(event).ok()
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
        GameEventKind::OathSworn { oath_id, title } => format!(
            r#"<article class="message message-oath" data-event-id="{}" data-oath-id="{}">Sworn: {}</article>"#,
            event.event_id,
            escape_html(oath_id),
            escape_html(title)
        ),
        GameEventKind::OathFulfilled { oath_id } => format!(
            r#"<article class="message message-oath" data-event-id="{}" data-oath-id="{}">Oath fulfilled</article>"#,
            event.event_id,
            escape_html(oath_id)
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
    use oathstar_protocol::{EventChannel, OathStatus, OutputComponent};

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
        let mut engine = Engine::try_new(world).expect("valid beginner world");
        let opening = Arc::new(engine.begin());
        let state = AppState {
            engine: Arc::new(Mutex::new(engine)),
            events: events.clone(),
            opening,
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
        let mut engine = Engine::try_new(world).expect("valid beginner world");
        let opening = Arc::new(engine.begin());
        AppState {
            engine: Arc::new(Mutex::new(engine)),
            events,
            opening,
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

    // ---- ticket #7: oath/boss render arms + begin + full-slice smoke ----

    // REQ-005 (render): the new oath events render as oath articles with their
    // ids/titles HTML-escaped.
    #[test]
    fn render_event_html_renders_oath_events_and_escapes() {
        let sworn = GameEvent {
            event_id: 5,
            tick: 1,
            channel: EventChannel::Oath,
            kind: GameEventKind::OathSworn {
                oath_id: "o<1>".to_string(),
                title: "A&B".to_string(),
            },
        };
        let html = render_event_html(&sworn);
        assert!(html.contains("message-oath"), "oath article class: {html}");
        assert!(html.contains(r#"data-event-id="5""#));
        assert!(html.contains("Sworn: A&amp;B"), "title escaped: {html}");
        assert!(html.contains("o&lt;1&gt;"), "oath id escaped: {html}");
        assert!(!html.contains("A&B"), "no raw ampersand survives");

        let fulfilled = GameEvent {
            event_id: 6,
            tick: 2,
            channel: EventChannel::Oath,
            kind: GameEventKind::OathFulfilled {
                oath_id: "o1".to_string(),
            },
        };
        let html = render_event_html(&fulfilled);
        assert!(html.contains("Oath fulfilled"), "fulfilled text: {html}");
        assert!(html.contains(r#"data-oath-id="o1""#));
    }

    // REQ-001: begin() on the real beginner world produces the start-room event.
    #[test]
    fn begin_emits_beginner_start_room() {
        let world = oathstar_content::load_beginner_world().expect("beginner world loads");
        let mut engine = Engine::try_new(world).expect("valid beginner world");
        let events = engine.begin();
        assert!(
            events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::RoomEntered { room_id, .. } if room_id == "hollowmere_square"
            )),
            "begin emits RoomEntered for hollowmere_square"
        );
        assert!(
            events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage {
                    component: OutputComponent::RoomHeader,
                    text,
                } if text.contains("Hollowmere Square")
            )),
            "begin emits the Hollowmere Square room header"
        );
    }

    // REQ-005 (smoke): the whole slice runs through the /command path on the real
    // beginner world: look (REQ-001) → swear (REQ-002) → route (REQ-003) →
    // confront (REQ-004), with the typed oath events and final state.
    #[tokio::test]
    async fn beginner_slice_runs_through_command_path() {
        let app = test_app_state();

        let req = |input: &str| {
            Json(CommandRequest {
                input: input.to_string(),
                actor_id: None,
            })
        };

        let look = command(State(app.clone()), req("look")).await;
        assert!(look.0.accepted, "look accepted");
        assert_eq!(look.0.snapshot.current_room_id, "hollowmere_square");

        let swear = command(State(app.clone()), req("swear")).await;
        assert!(swear.0.accepted, "swear accepted");
        assert!(
            swear
                .0
                .events
                .iter()
                .any(|e| matches!(&e.kind, GameEventKind::OathSworn { .. })),
            "swear emits OathSworn"
        );
        assert_eq!(
            swear.0.snapshot.oath.as_ref().expect("oath sworn").status,
            OathStatus::Sworn
        );

        for step in ["north", "north", "north", "up", "up"] {
            let moved = command(State(app.clone()), req(step)).await;
            assert!(moved.0.accepted, "move {step} accepted");
        }
        let here = state_snapshot(State(app.clone())).await;
        assert_eq!(
            here.0.current_room_id, "bell_eater_roost",
            "the authored route reaches the boss room"
        );

        let confront = command(State(app.clone()), req("confront")).await;
        assert!(confront.0.accepted, "confront accepted");
        assert!(
            confront
                .0
                .events
                .iter()
                .any(|e| matches!(&e.kind, GameEventKind::OathFulfilled { .. })),
            "confront emits OathFulfilled"
        );
        assert_eq!(
            confront
                .0
                .snapshot
                .oath
                .as_ref()
                .expect("oath present")
                .status,
            OathStatus::Fulfilled
        );
    }

    // ---- ticket #12: opening scene seeded onto new /events subscriptions ----

    // REQ-001: the server captures begin()'s opening scene at startup so it can
    // seed every new /events subscription (no `look` required).
    #[test]
    fn opening_scene_is_captured() {
        let app = test_app_state();
        assert!(
            app.opening.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::RoomEntered { room_id, .. } if room_id == "hollowmere_square"
            )),
            "opening scene enters hollowmere_square"
        );
        assert!(
            app.opening.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage {
                    component: OutputComponent::RoomHeader,
                    text,
                } if text.contains("Hollowmere Square")
            )),
            "opening scene carries the Hollowmere Square room header"
        );
    }

    // REQ-001 wire: the shared JSON serializer emits the Decision 031 wire shape
    // (camelCase envelope, snake_case `type` tag + payload).
    #[test]
    fn event_to_json_emits_wire_shape() {
        let event = GameEvent {
            event_id: 1,
            tick: 0,
            channel: EventChannel::Room,
            kind: GameEventKind::RoomEntered {
                room_id: "hollowmere_square".to_string(),
                title: "Hollowmere Square".to_string(),
            },
        };
        let json = event_to_json(&event).expect("event serializes");
        assert!(
            json.contains(r#""type":"room_entered""#),
            "snake_case type tag: {json}"
        );
        assert!(
            json.contains(r#""room_id":"hollowmere_square""#),
            "snake_case payload field: {json}"
        );
        assert!(
            json.contains(r#""eventId":1"#),
            "camelCase envelope: {json}"
        );
    }

    // REQ-001: every captured opening event serializes, and the bytes seeded onto
    // a new subscription name the start room (the same path the handler streams).
    #[test]
    fn opening_scene_seeds_serialize() {
        let app = test_app_state();
        let payloads: Vec<String> = app
            .opening
            .iter()
            .map(|e| event_to_json(e).expect("opening event serializes"))
            .collect();
        assert!(!payloads.is_empty(), "opening scene is non-empty");
        let joined = payloads.join("\n");
        assert!(
            joined.contains("hollowmere_square"),
            "seeded bytes name the start room: {joined}"
        );
        assert!(
            joined.contains("Hollowmere Square"),
            "seeded bytes carry the room header: {joined}"
        );
    }

    // REQ-001 (html variant): the html seed renders the opening room article.
    #[test]
    fn events_html_opening_renders_room() {
        let app = test_app_state();
        let html: String = app
            .opening
            .iter()
            .map(render_event_html)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            html.contains("message-room"),
            "html seed has a room article: {html}"
        );
        assert!(
            html.contains("Hollowmere Square"),
            "html seed names the start room: {html}"
        );
    }
}
