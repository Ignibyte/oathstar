use std::{convert::Infallible, net::SocketAddr, sync::Arc, time::Duration};

use async_stream::stream;
use axum::{
    extract::State,
    http::HeaderMap,
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
    Json, Router,
};
use oathstar_core::Engine;
use oathstar_datastar::{feed_patch, opening_patches};
use oathstar_protocol::{CommandRequest, CommandResponse, GameEvent, GameSnapshot};
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
        .route("/events/datastar", get(events_datastar))
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

/// The first-party Datastar feed stream: renders game events to Datastar
/// `datastar-patch-elements` SSE patches that append HTML fragments into the
/// player-client feed (`#log`). The Datastar-specific presentation lives in the
/// `oathstar-datastar` crate (REQ-002); `oathstar-core` never sees it.
///
/// The opening scene is seeded at the head of a fresh subscription, but skipped
/// when the client's `Last-Event-ID` shows it already has it — so a Datastar
/// reconnect does not duplicate the opening (REQ-003). Live broadcast events are
/// delivered once and never replayed.
async fn events_datastar(
    State(app): State<AppState>,
    headers: HeaderMap,
) -> Sse<impl futures_core::Stream<Item = Result<Event, Infallible>>> {
    let mut receiver = app.events.subscribe();

    let last_event_id = headers
        .get("last-event-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let seed = opening_patches(&app.opening, last_event_id.as_deref());

    let stream = stream! {
        for (id, patch) in seed {
            yield Ok(Event::default()
                .event(patch.event)
                .id(id.to_string())
                .data(patch.data));
        }
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    let Some(patch) = feed_patch(&event) else {
                        continue;
                    };
                    yield Ok(Event::default()
                        .event(patch.event)
                        .id(event.event_id.to_string())
                        .data(patch.data));
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

#[cfg(test)]
mod tests {
    use super::*;
    use oathstar_protocol::{EventChannel, GameEventKind, OathStatus, OutputComponent};

    #[tokio::test]
    async fn root_serves_the_status_line() {
        assert_eq!(
            root().await,
            "Oathstar server is running. Try GET /state, POST /command, or GET /events."
        );
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

    // ticket #17: from the start room, Mara in the adjacent candle_shop (same
    // subregion, one cell east) is perceivable and interactable — the proximity
    // foundation lights up the beginner slice's Nearby panel (REQ-004/005/007).
    #[tokio::test]
    async fn state_snapshot_exposes_nearby_contents() {
        let snapshot = state_snapshot(State(test_app_state())).await;
        let mara = snapshot
            .0
            .room
            .contents
            .iter()
            .find(|thing| thing.id == "mara")
            .expect("mara perceivable from the start room");
        assert_eq!(mara.kind, "actor");
        assert_eq!(mara.distance, 1);
        assert!(mara.interactable);
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

    // REQ-002/007 (smoke): the whole slice runs through the /command path on the
    // real beginner world: look → talk mara (offer, ticket #19) → swear → route →
    // confront, with the typed oath events and final state.
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

        // Ticket #19: the oath is offered by talking to Mara (one cell east, within
        // interaction range) before it can be sworn.
        let talk = command(State(app.clone()), req("talk mara")).await;
        assert!(talk.0.accepted, "talk mara accepted");
        assert!(
            talk.0.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage { text, .. }
                    if text.contains("Bell-Eater") && text.contains("swear")
            )),
            "talking to Mara introduces the Bell-Eater problem and exposes the oath"
        );

        let swear = command(State(app.clone()), req("swear")).await;
        assert!(swear.0.accepted, "swear accepted after the oath is offered");
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

    // REQ-003 (smoke): on the real beginner world, swearing before talking to Mara
    // is refused and guides the player to the oath-giver (ticket #19 offer gate).
    #[tokio::test]
    async fn beginner_swear_before_talking_to_mara_is_refused() {
        let app = test_app_state();
        let swear = command(
            State(app.clone()),
            Json(CommandRequest {
                input: "swear".to_string(),
                actor_id: None,
            }),
        )
        .await;
        assert!(!swear.0.accepted, "swearing before the offer is refused");
        assert!(
            swear.0.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage { text, .. }
                    if text.contains("offered") && text.contains("Mara")
            )),
            "refusal guides the player to Mara"
        );
        assert!(swear.0.snapshot.oath.is_none(), "no oath recorded");
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

    // REQ-003 (datastar variant): the feed seed renders the opening room as a
    // datastar-patch-elements append, via the oathstar-datastar crate.
    #[test]
    fn events_datastar_opening_renders_room() {
        let app = test_app_state();
        let seed = opening_patches(&app.opening, None);
        assert!(!seed.is_empty(), "opening seeds feed patches");
        assert!(
            seed.iter()
                .all(|(_, patch)| patch.event == "datastar-patch-elements"),
            "every seed patch is a datastar element patch"
        );
        let joined = seed
            .iter()
            .map(|(_, patch)| patch.data.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Hollowmere Square"),
            "seed names the start room: {joined}"
        );
        assert!(
            joined.contains("log-entry"),
            "seed uses feed-entry markup: {joined}"
        );
    }
}
