use std::{convert::Infallible, net::SocketAddr, sync::Arc, time::Duration};

use async_stream::stream;
use axum::{
    extract::State,
    http::HeaderMap,
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
    Json, Router,
};
use oathstar_core::{Engine, SaveData};
use oathstar_datastar::{feed_patch, opening_patches};
use oathstar_protocol::{CommandRequest, CommandResponse, GameEvent, GameSnapshot};
use oathstar_storage::{FileSaveStore, SaveStore};
use serde::{Deserialize, Serialize};
use tokio::{
    net::TcpListener,
    sync::{broadcast, Mutex},
    time,
};

/// The slot a save/load request without an explicit `slot` uses (ticket #28).
/// The v1 client always saves and loads here; named slots are exercised by
/// callers that pass one.
const DEFAULT_SAVE_SLOT: &str = "quicksave";

#[derive(Clone)]
struct AppState {
    engine: Arc<Mutex<Engine>>,
    events: broadcast::Sender<GameEvent>,
    /// The opening-scene events from `Engine::begin()`, captured once at startup
    /// and replayed at the head of every new `/events` subscription so a fresh
    /// client renders the start room without sending `look` first (REQ-001 /
    /// Decision 031: `try_new` emits nothing; `begin` is the on-start emitter).
    opening: Arc<Vec<GameEvent>>,
    /// Where save slots live (ticket #28): the hardened `FileSaveStore` rooted
    /// at `OATHSTAR_SAVE_DIR` (default `saves`). The store is the only
    /// persistence path — slot validation and symlink defense come with it.
    saves: FileSaveStore,
}

/// The JSON body of POST `/save` and `/load` (ticket #28). The body itself is
/// required (the client sends `{}`); the `slot` FIELD is optional and an
/// omitted one means [`DEFAULT_SAVE_SLOT`]. The slot name is validated by the
/// storage layer before any filesystem contact.
#[derive(Debug, Deserialize)]
struct SaveLoadRequest {
    #[serde(default)]
    slot: Option<String>,
}

/// The in-band result of POST `/save` and `/load` (ticket #28): `ok` plus an
/// error message on refusal — the `/command` `accepted: false` convention, so
/// a failed save/load is a readable refusal, not a bare status code.
#[derive(Debug, Serialize)]
struct SaveLoadResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl SaveLoadResponse {
    const fn success() -> Self {
        Self {
            ok: true,
            error: None,
        }
    }

    fn refusal(error: impl std::fmt::Display) -> Self {
        Self {
            ok: false,
            error: Some(error.to_string()),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let world = oathstar_content::load_beginner_world()?;
    let mut engine = Engine::try_new(world)?;
    // Emit the opening scene once, up front, and keep it to seed each new
    // subscriber. begin() does not move the player, so /state stays consistent.
    let opening = Arc::new(engine.begin());
    let (events, _) = broadcast::channel(256);

    let save_dir = std::env::var("OATHSTAR_SAVE_DIR").unwrap_or_else(|_| "saves".to_string());
    let app_state = AppState {
        engine: Arc::new(Mutex::new(engine)),
        events,
        opening,
        saves: FileSaveStore::new(save_dir),
    };

    spawn_tick_loop(app_state.clone());

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/state", get(state_snapshot))
        .route("/command", post(command))
        .route("/save", post(save))
        .route("/load", post(load))
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

/// POST `/save` (ticket #28): capture the running session and write it to the
/// requested slot. The payload is cloned under the engine lock; the disk write
/// happens after the lock drops, so pulses never stall on IO. A refused slot
/// or a failed write is an in-band refusal and the session is untouched
/// (saving never mutates it either way).
async fn save(
    State(app): State<AppState>,
    Json(request): Json<SaveLoadRequest>,
) -> Json<SaveLoadResponse> {
    let slot = request.slot.as_deref().unwrap_or(DEFAULT_SAVE_SLOT);
    let engine = app.engine.lock().await;
    let data = engine.save_data();
    drop(engine);

    Json(match app.saves.write_json(slot, &data) {
        Ok(()) => SaveLoadResponse::success(),
        // anyhow alternate keeps the context chain ("failed to write …: …").
        Err(error) => SaveLoadResponse::refusal(format!("{error:#}")),
    })
}

/// POST `/load` (ticket #28): read the requested slot, rebuild the engine
/// through [`Engine::from_save`] (the untrusted-input boundary), and only on
/// success swap it in under the engine lock — the tick loop and concurrent
/// commands serialize through the same lock, so no partial session is ever
/// observable. Any failure (slot, file, parse, version, validation,
/// incoherence) is an in-band refusal that leaves the running session
/// unchanged.
async fn load(
    State(app): State<AppState>,
    Json(request): Json<SaveLoadRequest>,
) -> Json<SaveLoadResponse> {
    let slot = request.slot.as_deref().unwrap_or(DEFAULT_SAVE_SLOT);
    let data: SaveData = match app.saves.read_json(slot) {
        Ok(data) => data,
        Err(error) => {
            // The commonest refusal — no save yet — gets a player-readable
            // line instead of a raw OS error with a filesystem path.
            let refusal = if error.chain().any(|cause| {
                cause
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|io_error| io_error.kind() == std::io::ErrorKind::NotFound)
            }) {
                format!("no save exists in slot '{slot}' yet")
            } else {
                format!("{error:#}")
            };
            return Json(SaveLoadResponse::refusal(refusal));
        }
    };
    let loaded = match Engine::from_save(data) {
        Ok(engine) => engine,
        Err(error) => return Json(SaveLoadResponse::refusal(error)),
    };

    let mut engine = app.engine.lock().await;
    *engine = loaded;
    drop(engine);

    Json(SaveLoadResponse::success())
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
        // A suspended/stalled process must not fast-forward the world: the
        // default Burst behavior fires every missed 1s tick back-to-back on
        // resume, which would resolve whole real-time combats in one instant
        // (ticket #24). Skip drops the missed ticks and resumes the cadence.
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let mut engine = app.engine.lock().await;
            // A tick now returns the Tick event plus any combat-pulse events
            // (ticket #24); broadcast in order, after the lock drops, like the
            // /command handler.
            let events = engine.tick();
            drop(engine);
            for event in events {
                let _ = app.events.send(event);
            }
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
    use oathstar_protocol::{
        CombatOutcome, EventChannel, GameEventKind, OathStatus, OutputComponent,
    };

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
            saves: FileSaveStore::new(scratch_save_dir("tick-loop")),
        };
        let mut rx = events.subscribe();
        spawn_tick_loop(state);
        let event = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("a tick is broadcast within 2s")
            .expect("broadcast channel stays open");
        assert!(matches!(event.kind, GameEventKind::Tick { .. }));
    }

    /// A per-test save root under the OS temp dir, namespaced by pid + tag so
    /// parallel test binaries and parallel tests never share a slot file.
    fn scratch_save_dir(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "oathstar-server-saves-{}-{tag}",
            std::process::id()
        ))
    }

    fn test_app_state() -> AppState {
        test_app_state_with_saves("shared-readonly")
    }

    /// Like [`test_app_state`], with the save store rooted at a caller-owned
    /// scratch tag — save/load tests pass their own tag so they never collide.
    fn test_app_state_with_saves(tag: &str) -> AppState {
        let world = oathstar_content::load_beginner_world().expect("beginner world loads");
        let (events, _rx) = broadcast::channel(16);
        let mut engine = Engine::try_new(world).expect("valid beginner world");
        let opening = Arc::new(engine.begin());
        AppState {
            engine: Arc::new(Mutex::new(engine)),
            events,
            opening,
            saves: FileSaveStore::new(scratch_save_dir(tag)),
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

        // N11 (ticket #27): the world-scoped bell alarm is delivered at the
        // roost with its exact authored text, while the hollowmere-region
        // notice is provably NOT delivered in the old_bell_tower region —
        // the in-play both-arms scoped-announcement demo.
        assert!(
            confront.0.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::Announcement { text, .. }
                    if text == "The bell of Hollowmere rings again. Its voice rolls out over every road and roof."
            )),
            "the world-scoped bell alarm is delivered at the roost"
        );
        assert!(
            !confront.0.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::Announcement { text, .. }
                    if text.contains("Hollowmere's streets")
            )),
            "the hollowmere-region notice is not delivered in old_bell_tower"
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

    // ---- ticket #24: the real-time pulse loop over the live server seam ----
    // `start_paused`: tokio's virtual clock auto-advances whenever every task is
    // idle, so the REAL 1s `spawn_tick_loop` drives fully deterministic pulses
    // with no real waiting — the integration-layer face of REQ-006.

    fn req(input: &str) -> Json<CommandRequest> {
        Json(CommandRequest {
            input: input.to_string(),
            actor_id: None,
        })
    }

    /// Walk the authored route from the start room to the combat-enabled
    /// Ashen Road (two cells north), where the beginner hostile waits.
    async fn walk_to_ashen_road(app: &AppState) {
        for step in ["north", "north"] {
            let moved = command(State(app.clone()), req(step)).await;
            assert!(moved.0.accepted, "move {step} accepted");
        }
    }

    /// Drain the subscription's Combat-channel events until `CombatEnded`,
    /// returning the kinds in arrival order. Virtual time advances while the
    /// recv awaits, so the tick loop keeps pulsing underneath.
    async fn drain_combat_until_ended(
        rx: &mut broadcast::Receiver<GameEvent>,
    ) -> Vec<GameEventKind> {
        let mut kinds = Vec::new();
        loop {
            let event = tokio::time::timeout(Duration::from_mins(5), rx.recv())
                .await
                .expect("combat resolves within virtual time")
                .expect("broadcast stays open");
            if !matches!(event.channel, EventChannel::Combat) {
                continue;
            }
            let done = matches!(event.kind, GameEventKind::CombatEnded { .. });
            kinds.push(event.kind);
            if done {
                return kinds;
            }
        }
    }

    // S1 (REQ-001/003): the real tick loop streams the whole deterministic
    // combat sequence to a subscriber — no command after the opening attack —
    // and /state reflects the resolved fight (the battle modal's data pair).
    #[tokio::test(start_paused = true)]
    async fn pulses_stream_combat_to_subscribers_until_victory() {
        let app = test_app_state();
        walk_to_ashen_road(&app).await;
        let mut rx = app.events.subscribe();
        let attack = command(State(app.clone()), req("attack")).await;
        assert!(
            attack.0.accepted,
            "the bare attack engages the road hostile"
        );
        spawn_tick_loop(app.clone());

        let kinds = drain_combat_until_ended(&mut rx).await;
        // Ashen Stray (9 hp / attack 3) vs strike 4: round 1 from the command
        // (5/9, 17/20), pulse round 2 (1/9, 14/20), pulse round 3 (0/9 — no
        // return after the kill), then the #26 reward loop: the fang drop line
        // and the XP-bearing victory summary.
        assert_eq!(kinds.len(), 10, "the exact combat sequence: {kinds:?}");
        assert!(
            matches!(&kinds[0], GameEventKind::CombatStarted { enemy_id, .. } if enemy_id == "ashen_stray")
        );
        assert!(matches!(kinds[3], GameEventKind::CombatPulse { round: 2 }));
        assert!(matches!(kinds[6], GameEventKind::CombatPulse { round: 3 }));
        assert!(
            matches!(
                &kinds[8],
                GameEventKind::LogMessage { component: OutputComponent::CombatMessage, text }
                    if text == "The Ashen Stray drops Cracked Fang."
            ),
            "the drop narrates before the summary: {kinds:?}"
        );
        assert!(matches!(
            &kinds[9],
            GameEventKind::CombatEnded {
                outcome: CombatOutcome::Victory,
                text,
            } if text == "You have defeated Ashen Stray. Victory! You gain 5 XP."
        ));

        let state = state_snapshot(State(app.clone())).await;
        assert!(state.0.combat.is_none(), "combat cleared after the victory");
        assert_eq!(state.0.player.hp, 14, "two enemy returns landed (20 → 14)");
        assert_eq!(state.0.player.xp, 5, "the authored reward landed");

        // X14 (ticket #26): the played reward loop closes — the dropped fang
        // is takeable through the existing flow and lands in the pack.
        let take = command(State(app), req("take fang")).await;
        assert!(take.0.accepted, "the dropped fang is takeable");
        assert!(
            take.0
                .snapshot
                .pack
                .iter()
                .any(|item| item.name == "Cracked Fang"),
            "the fang lands in the pack"
        );
    }

    // S2 (REQ-003/004): a flee submitted between pulses queues, and the next
    // pulse's skill window ends the encounter as fled — streamed live.
    #[tokio::test(start_paused = true)]
    async fn flee_between_pulses_ends_the_encounter_fled() {
        let app = test_app_state();
        walk_to_ashen_road(&app).await;
        let mut rx = app.events.subscribe();
        assert!(command(State(app.clone()), req("attack")).await.0.accepted);
        let flee = command(State(app.clone()), req("flee")).await;
        assert!(flee.0.accepted);
        assert_eq!(
            flee.0
                .snapshot
                .combat
                .as_ref()
                .and_then(|combat| combat.queued_action.as_deref()),
            Some("flee"),
            "the queued action rides the command snapshot"
        );
        spawn_tick_loop(app.clone());

        let kinds = drain_combat_until_ended(&mut rx).await;
        // started + round-1 pair + the flee confirmation + pulse marker +
        // round-2 pair + fled end.
        assert_eq!(kinds.len(), 8, "the exact fled sequence: {kinds:?}");
        assert!(matches!(kinds[4], GameEventKind::CombatPulse { round: 2 }));
        assert!(matches!(
            &kinds[7],
            GameEventKind::CombatEnded {
                outcome: CombatOutcome::Fled,
                ..
            }
        ));

        let state = state_snapshot(State(app)).await;
        assert!(state.0.combat.is_none(), "fled clears combat");
        assert_eq!(state.0.player.hp, 14, "post-exchange HP kept — no revive");
    }

    // S3 (REQ-007): a command burst interleaves safely with the running tick
    // loop — the engine mutex serializes them, everything completes, and the
    // final state is coherent.
    #[tokio::test(start_paused = true)]
    async fn commands_and_tick_loop_interleave_safely() {
        let app = test_app_state();
        walk_to_ashen_road(&app).await;
        spawn_tick_loop(app.clone());
        let mut rx = app.events.subscribe();

        let (attack, looked, flee) = tokio::join!(
            command(State(app.clone()), req("attack")),
            command(State(app.clone()), req("look")),
            command(State(app.clone()), req("flee")),
        );
        assert!(attack.0.accepted && looked.0.accepted && flee.0.accepted);

        let kinds = drain_combat_until_ended(&mut rx).await;
        assert!(
            matches!(
                kinds.last(),
                Some(GameEventKind::CombatEnded {
                    outcome: CombatOutcome::Fled,
                    ..
                })
            ),
            "the queued flee resolves the interleaved fight: {kinds:?}"
        );
        let state = state_snapshot(State(app)).await;
        assert!(state.0.combat.is_none(), "no encounter survives");
        assert!(
            state.0.player.hp > 0 && state.0.player.hp <= state.0.player.max_hp,
            "player state stays coherent under interleaving"
        );
    }

    // ---- ticket #28: save & load over the live server seam ----

    fn slot_request(slot: Option<&str>) -> Json<SaveLoadRequest> {
        Json(SaveLoadRequest {
            slot: slot.map(str::to_owned),
        })
    }

    async fn state_value(app: &AppState) -> serde_json::Value {
        serde_json::to_value(&state_snapshot(State(app.clone())).await.0)
            .expect("snapshots serialize")
    }

    // SV1 (REQ-005/006): the endpoint round-trip — save lands the default
    // slot file under the configured root (the DEFAULT_SAVE_SLOT/env-root
    // killer), the session walks away, and load swaps the at-save /state
    // back in byte-for-byte.
    #[tokio::test]
    async fn save_then_load_round_trips_state() {
        let app = test_app_state_with_saves("sv1-round-trip");
        std::fs::remove_dir_all(app.saves.root()).ok();
        walk_to_ashen_road(&app).await;

        let saved = save(State(app.clone()), slot_request(None)).await;
        assert!(saved.0.ok, "save accepted: {:?}", saved.0.error);
        assert!(
            app.saves.root().join("quicksave.json").exists(),
            "the default slot file lands under the configured root"
        );
        let at_save = state_value(&app).await;

        let back = command(State(app.clone()), req("south")).await;
        assert!(back.0.accepted);
        assert_ne!(
            state_value(&app).await,
            at_save,
            "the live session diverges after the save"
        );

        let loaded = load(State(app.clone()), slot_request(None)).await;
        assert!(loaded.0.ok, "load accepted: {:?}", loaded.0.error);
        assert_eq!(
            state_value(&app).await,
            at_save,
            "the restored /state equals the at-save /state byte-for-byte"
        );
        std::fs::remove_dir_all(app.saves.root()).ok();
    }

    // SV2 (REQ-004): an invalid slot is refused through the storage layer's
    // validation, end to end, without any filesystem contact.
    #[tokio::test]
    async fn invalid_slot_is_refused_without_touching_the_filesystem() {
        let app = test_app_state_with_saves("sv2-invalid-slot");
        std::fs::remove_dir_all(app.saves.root()).ok();
        let response = save(State(app.clone()), slot_request(Some("../evil"))).await;
        assert!(!response.0.ok, "a traversal slot is refused");
        let error = response.0.error.expect("the refusal names the cause");
        assert!(
            error.contains("save slot id '../evil' must not contain a path separator"),
            "the storage validation surfaces verbatim: {error}"
        );
        assert!(
            !app.saves.root().exists(),
            "no save directory is created for a rejected slot"
        );
    }

    // SV3a (REQ-003/005): loading with no save on disk refuses with the
    // friendly first-load line — no OS error, no filesystem path — and the
    // running session is untouched (the NotFound branch's friendly arm).
    #[tokio::test]
    async fn missing_save_load_is_refused_and_state_unchanged() {
        let app = test_app_state_with_saves("sv3-missing");
        std::fs::remove_dir_all(app.saves.root()).ok();
        let before = state_value(&app).await;
        let response = load(State(app.clone()), slot_request(None)).await;
        assert!(!response.0.ok, "a missing save refuses the load");
        assert_eq!(
            response.0.error.as_deref(),
            Some("no save exists in slot 'quicksave' yet"),
            "the friendly refusal, not a raw OS error"
        );
        assert_eq!(
            state_value(&app).await,
            before,
            "a refused load leaves the session untouched"
        );
    }

    // SV3b: the NotFound branch's OTHER arm — a present-but-corrupt slot
    // keeps the parse-context refusal (kills a branch-flip mutant on the
    // ErrorKind check) and still leaves the session untouched.
    #[tokio::test]
    async fn corrupt_save_load_is_refused_with_the_parse_context() {
        let app = test_app_state_with_saves("sv3-corrupt");
        std::fs::remove_dir_all(app.saves.root()).ok();
        std::fs::create_dir_all(app.saves.root()).expect("make the save root");
        std::fs::write(app.saves.root().join("quicksave.json"), b"{ not json")
            .expect("seed a corrupt slot");
        let before = state_value(&app).await;
        let response = load(State(app.clone()), slot_request(None)).await;
        assert!(!response.0.ok, "a corrupt save refuses the load");
        let error = response.0.error.expect("the refusal names the cause");
        assert!(
            error.contains("failed to parse"),
            "the non-NotFound arm keeps the context chain: {error}"
        );
        assert_eq!(
            state_value(&app).await,
            before,
            "a refused load leaves the session untouched"
        );
        std::fs::remove_dir_all(app.saves.root()).ok();
    }

    // SV3c: a version-mismatched file refuses through Engine::from_save and
    // the typed message is forwarded verbatim (the from_save match arm's
    // killer at the endpoint layer).
    #[tokio::test]
    async fn version_mismatched_save_load_is_refused_via_from_save() {
        let app = test_app_state_with_saves("sv3-version");
        std::fs::remove_dir_all(app.saves.root()).ok();
        let saved = save(State(app.clone()), slot_request(None)).await;
        assert!(saved.0.ok);
        let slot_path = app.saves.root().join("quicksave.json");
        let mut on_disk: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&slot_path).expect("the slot file reads"),
        )
        .expect("the slot file parses");
        on_disk["version"] = serde_json::json!(2);
        std::fs::write(&slot_path, on_disk.to_string()).expect("rewrite the slot");

        let response = load(State(app.clone()), slot_request(None)).await;
        assert!(!response.0.ok, "a future-version save refuses the load");
        assert_eq!(
            response.0.error.as_deref(),
            Some("save format version 2 is not supported (this build reads version 1)"),
            "the typed LoadError display is forwarded verbatim"
        );
        std::fs::remove_dir_all(app.saves.root()).ok();
    }

    // SV4 (REQ-007): the played acceptance loop — earn the xp and the fang,
    // save, lose both, load: restored, standing back on the road.
    #[tokio::test(start_paused = true)]
    async fn played_loop_xp_and_fang_survive_save_and_load() {
        let app = test_app_state_with_saves("sv4-played-loop");
        std::fs::remove_dir_all(app.saves.root()).ok();
        walk_to_ashen_road(&app).await;
        let mut rx = app.events.subscribe();
        assert!(command(State(app.clone()), req("attack")).await.0.accepted);
        spawn_tick_loop(app.clone());
        let _ = drain_combat_until_ended(&mut rx).await;
        assert!(
            command(State(app.clone()), req("take fang"))
                .await
                .0
                .accepted
        );

        let saved = save(State(app.clone()), slot_request(None)).await;
        assert!(saved.0.ok, "{:?}", saved.0.error);

        assert!(
            command(State(app.clone()), req("drop fang"))
                .await
                .0
                .accepted
        );
        assert!(command(State(app.clone()), req("south")).await.0.accepted);

        let loaded = load(State(app.clone()), slot_request(None)).await;
        assert!(loaded.0.ok, "{:?}", loaded.0.error);
        let state = state_snapshot(State(app.clone())).await;
        assert_eq!(state.0.player.xp, 5, "the earned xp survives the loop");
        assert!(
            state.0.pack.iter().any(|item| item.name == "Cracked Fang"),
            "the taken fang survives the loop"
        );
        assert_eq!(
            state.0.current_room_id, "ashen_road",
            "the player stands back on the road"
        );
        std::fs::remove_dir_all(app.saves.root()).ok();
    }

    // SV5 (REQ-005): the swap under fire — save mid-combat, let the REAL
    // tick loop finish the live fight, then load the mid-fight save back in
    // UNDER the running loop: /state is the at-save encounter byte-for-byte
    // and the restored fight pulses to the same deterministic victory.
    #[tokio::test(start_paused = true)]
    async fn mid_combat_save_and_load_resume_the_saved_fight() {
        let app = test_app_state_with_saves("sv5-swap-under-fire");
        std::fs::remove_dir_all(app.saves.root()).ok();
        walk_to_ashen_road(&app).await;
        assert!(command(State(app.clone()), req("attack")).await.0.accepted);

        let saved = save(State(app.clone()), slot_request(None)).await;
        assert!(saved.0.ok, "{:?}", saved.0.error);
        let at_save = state_value(&app).await;

        let mut rx = app.events.subscribe();
        spawn_tick_loop(app.clone());
        let _ = drain_combat_until_ended(&mut rx).await;
        let post = state_snapshot(State(app.clone())).await;
        assert!(post.0.combat.is_none(), "the live fight ran to its end");

        let mut rx = app.events.subscribe();
        let loaded = load(State(app.clone()), slot_request(None)).await;
        assert!(loaded.0.ok, "{:?}", loaded.0.error);
        assert_eq!(
            state_value(&app).await,
            at_save,
            "the loaded /state is the at-save mid-encounter session"
        );

        let kinds = drain_combat_until_ended(&mut rx).await;
        assert!(
            matches!(
                kinds.last(),
                Some(GameEventKind::CombatEnded {
                    outcome: CombatOutcome::Victory,
                    ..
                })
            ),
            "the running loop pulses the restored fight to victory: {kinds:?}"
        );
        let end = state_snapshot(State(app.clone())).await;
        assert_eq!(end.0.player.xp, 5, "the restored fight pays the reward");
        assert_eq!(
            end.0.player.hp, 14,
            "the replayed pulses land the saved fight's exact damage"
        );
        std::fs::remove_dir_all(app.saves.root()).ok();
    }
}
