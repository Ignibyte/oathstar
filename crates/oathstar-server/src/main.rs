use std::{convert::Infallible, net::SocketAddr, sync::Arc, time::Duration};

use async_stream::stream;
use axum::{
    extract::{FromRef, State},
    http::HeaderMap,
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
    Json, Router,
};
use oathstar_core::{Engine, SaveData};
use oathstar_datastar::{feed_patch, opening_patches};
use oathstar_protocol::{
    AuthRole, CommandRequest, CommandResponse, GameEvent, GameSnapshot, Principal,
};
use oathstar_storage::{FileSaveStore, SaveStore};
use serde::{Deserialize, Serialize};
use tokio::{
    net::TcpListener,
    sync::{broadcast, Mutex},
    time,
};

use oathstar_auth::{AuthError, AuthPrincipal, SessionStore};

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
    /// The auth session registry (ticket #41/#42): resolves a bearer token (or a
    /// studio session id) to its `Principal`. Empty in production until a real
    /// session source exists; seeded by `OATHSTAR_DEV_OWNER` in local dev. Shared
    /// with the studio via the `oathstar-auth` crate.
    auth_sessions: SessionStore,
}

impl FromRef<AppState> for SessionStore {
    fn from_ref(state: &AppState) -> Self {
        state.auth_sessions.clone()
    }
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
        auth_sessions: SessionStore::from_env(),
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
        .route("/admin/session", get(admin_session))
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

/// Protected probe (ticket #41): the auth boundary's first consumer. The
/// `AuthPrincipal` extractor refuses unauthenticated callers with 401; this
/// handler then requires an editor-tier session ([`AuthRole::Owner`] satisfies
/// it via full authority) and echoes the authenticated principal. The admin
/// shell surface itself is a later ticket.
async fn admin_session(principal: AuthPrincipal) -> Result<Json<Principal>, AuthError> {
    oathstar_auth::require_role(&principal.0, AuthRole::Editor)?;
    Ok(Json(principal.0))
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
            auth_sessions: SessionStore::default(),
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
            auth_sessions: SessionStore::default(),
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

    // ── ticket #41: auth boundary integration ───────────────────────────────

    fn auth_principal(roles: Vec<AuthRole>) -> AuthPrincipal {
        AuthPrincipal(Principal {
            id: "tester".to_owned(),
            name: "Tester".to_owned(),
            roles,
        })
    }

    // REQ-002/006: a non-editor principal is forbidden (403) at the probe.
    #[tokio::test]
    async fn admin_session_forbids_a_player() {
        use axum::response::IntoResponse;

        let rejection = admin_session(auth_principal(vec![AuthRole::Player]))
            .await
            .expect_err("a player must be forbidden");
        assert_eq!(
            rejection.into_response().status(),
            axum::http::StatusCode::FORBIDDEN
        );
    }

    // REQ-003/006: an editor — and an owner, via full authority — gets the
    // authenticated principal echoed back.
    #[tokio::test]
    async fn admin_session_echoes_an_authorized_principal() {
        for roles in [vec![AuthRole::Editor], vec![AuthRole::Owner]] {
            let echoed = admin_session(auth_principal(roles.clone()))
                .await
                .expect("an authorized principal is echoed");
            assert_eq!(echoed.0.id, "tester");
            assert_eq!(echoed.0.roles, roles);
        }
    }

    // REQ-001: the extractor refuses an unauthenticated request with 401 before
    // the handler body runs; a seeded dev-owner token yields the principal.
    #[tokio::test]
    async fn auth_principal_extractor_gates_on_the_session() {
        use axum::extract::FromRequestParts;
        use axum::response::IntoResponse;

        let mut state = test_app_state();
        state.auth_sessions = SessionStore::from_owner_token(Some("devtok".to_owned()));

        let mut no_auth = axum::http::Request::builder()
            .body(axum::body::Body::empty())
            .expect("request builds")
            .into_parts()
            .0;
        let rejection = AuthPrincipal::from_request_parts(&mut no_auth, &state)
            .await
            .expect_err("a request with no session must be rejected");
        assert_eq!(
            rejection.into_response().status(),
            axum::http::StatusCode::UNAUTHORIZED
        );

        let mut with_auth = axum::http::Request::builder()
            .header(axum::http::header::AUTHORIZATION, "Bearer devtok")
            .body(axum::body::Body::empty())
            .expect("request builds")
            .into_parts()
            .0;
        let principal = AuthPrincipal::from_request_parts(&mut with_auth, &state)
            .await
            .expect("a seeded token authenticates");
        assert!(principal.0.grants(AuthRole::Owner));
    }

    // REQ-005: player endpoints need no auth — the default app state carries an
    // empty session store and `/state` still answers.
    #[tokio::test]
    async fn player_endpoints_need_no_auth() {
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

    /// The shared journey of the beginner slice (SV-B at ticket #29): look →
    /// talk mara (the offer) → swear → the authored route to the roost →
    /// confront (the REAL pulse-loop fight) → victory. Returns the fight's
    /// Combat-channel kinds plus the live subscription (still positioned
    /// right after `CombatEnded` — the ticket #30 `LevelUp` follows on it) so
    /// each caller pins its own act.
    async fn play_to_boss_victory(
        app: &AppState,
    ) -> (Vec<GameEventKind>, broadcast::Receiver<GameEvent>) {
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

        // Ticket #29: confront STARTS the real boss fight — the pulse loop
        // drives it to victory (12 hp vs strike 4 = three rounds; two
        // 4-damage returns land).
        let mut rx = app.events.subscribe();
        let confront = command(State(app.clone()), req("confront")).await;
        assert!(confront.0.accepted, "confront engages the Bell-Eater");
        assert!(
            confront.0.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::CombatStarted { enemy_id, .. } if enemy_id == "bell_eater"
            )),
            "confront starts the boss encounter"
        );
        assert!(
            !confront
                .0
                .events
                .iter()
                .any(|e| matches!(&e.kind, GameEventKind::OathFulfilled { .. })),
            "the confrontation alone fulfills nothing"
        );
        spawn_tick_loop(app.clone());
        let kinds = drain_combat_until_ended(&mut rx).await;
        (kinds, rx)
    }

    // REQ-002/007 + ticket #29 act 1: the slice fights the boss to victory
    // through the real pulse loop — authored reward, article-correct drop
    // line, and the oath provably STILL Sworn after the fight.
    #[tokio::test(start_paused = true)]
    async fn beginner_slice_fights_the_boss_to_victory() {
        let app = test_app_state();
        let (kinds, mut rx) = play_to_boss_victory(&app).await;
        assert!(
            matches!(
                kinds.last(),
                Some(GameEventKind::CombatEnded {
                    outcome: CombatOutcome::Victory,
                    text,
                }) if text == "You have defeated The Bell-Eater. Victory! You gain 25 XP and 25 coins."
            ),
            "the pulse loop fells the boss with the authored reward: {kinds:?}"
        );
        assert!(
            kinds.iter().any(|kind| matches!(
                kind,
                GameEventKind::LogMessage {
                    component: OutputComponent::CombatMessage,
                    text,
                } if text == "The Bell-Eater drops Bell Clapper."
            )),
            "the clapper falls with the authored article intact: {kinds:?}"
        );
        // The LevelUp broadcasts right after the victory summary, on the
        // Skill channel (outside the Combat-channel drain) — ticket #30.
        let level_up = loop {
            let event = tokio::time::timeout(Duration::from_mins(5), rx.recv())
                .await
                .expect("the level-up follows the victory")
                .expect("broadcast stays open");
            if matches!(event.channel, EventChannel::Skill) {
                break event;
            }
        };
        assert!(
            matches!(
                level_up.kind,
                GameEventKind::LevelUp {
                    level: 2,
                    max_hp: 25
                }
            ),
            "the 25-xp victory crosses the first threshold: {level_up:?}"
        );
        let after_fight = state_snapshot(State(app.clone())).await;
        assert_eq!(after_fight.0.player.xp, 25, "the boss reward landed");
        assert_eq!(after_fight.0.player.level, 2, "the victory lands level 2");
        assert_eq!(
            after_fight.0.player.max_hp, 25,
            "the level grows max HP (20 + 5)"
        );
        assert_eq!(
            after_fight.0.player.hp, 25,
            "the level-up heals to the new max (ticket #30 — was 12/20)"
        );
        assert_eq!(
            after_fight.0.oath.as_ref().expect("oath present").status,
            OathStatus::Sworn,
            "victory alone does not fulfill — recovery does"
        );
    }

    // Ticket #29 act 2: recovering the clapper fulfills the oath and rings
    // the bell (ticket #27) — the world-scoped alarm delivered at the roost
    // with its exact authored text, the hollowmere-region notice provably
    // NOT delivered in old_bell_tower (the in-play both-arms demo, now on
    // the take) — then Mara speaks her fulfilled line back in the square.
    #[tokio::test(start_paused = true)]
    async fn beginner_recovery_fulfills_and_rings_the_bell() {
        let app = test_app_state();
        let _ = play_to_boss_victory(&app).await;
        let take = command(State(app.clone()), req("take clapper")).await;
        assert!(take.0.accepted, "the dropped clapper is takeable");
        assert!(
            take.0
                .events
                .iter()
                .any(|e| matches!(&e.kind, GameEventKind::OathFulfilled { .. })),
            "recovering the clapper emits OathFulfilled"
        );
        assert_eq!(
            take.0.snapshot.oath.as_ref().expect("oath present").status,
            OathStatus::Fulfilled
        );
        assert!(
            take.0.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::Announcement { text, .. }
                    if text == "The bell of Hollowmere rings again. Its voice rolls out over every road and roof."
            )),
            "the world-scoped bell alarm is delivered at the roost"
        );
        assert!(
            !take.0.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::Announcement { text, .. }
                    if text.contains("Hollowmere's streets")
            )),
            "the hollowmere-region notice is not delivered in old_bell_tower"
        );

        // The loop closes at the oath-giver: walk back and hear Mara's
        // fulfilled line (ticket #19's dialogue selection, post-recovery).
        for step in ["down", "down", "south", "south", "south"] {
            let moved = command(State(app.clone()), req(step)).await;
            assert!(moved.0.accepted, "move {step} accepted");
        }
        let mara = command(State(app.clone()), req("talk mara")).await;
        assert!(mara.0.accepted, "mara is reachable from the square");
        assert!(
            mara.0.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage { text, .. } if text.contains("kept your word")
            )),
            "Mara speaks her fulfilled line: {:?}",
            mara.0.events
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
            } if text == "You have defeated Ashen Stray. Victory! You gain 5 XP and 4 coins."
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
        on_disk["version"] = serde_json::json!(99);
        std::fs::write(&slot_path, on_disk.to_string()).expect("rewrite the slot");

        let response = load(State(app.clone()), slot_request(None)).await;
        assert!(!response.0.ok, "a future-version save refuses the load");
        assert_eq!(
            response.0.error.as_deref(),
            Some("save format version 99 is not supported (this build reads version 2)"),
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

    // SV-L (ticket #30, REQ-008): the played progression over the live seam —
    // the stray's 5 xp banks below the first threshold (level 1), then the
    // boss's 25 lands 30 total, crossing BOTH thresholds in one victory: the
    // burst carries LevelUp{2,25} then LevelUp{3,30}, and /state settles at
    // level 3, 30/30.
    #[tokio::test(start_paused = true)]
    async fn played_progression_reaches_level_three() {
        let app = test_app_state();
        assert!(
            command(State(app.clone()), req("talk mara"))
                .await
                .0
                .accepted
        );
        assert!(command(State(app.clone()), req("swear")).await.0.accepted);

        walk_to_ashen_road(&app).await;
        let mut rx = app.events.subscribe();
        assert!(command(State(app.clone()), req("attack")).await.0.accepted);
        spawn_tick_loop(app.clone());
        let _ = drain_combat_until_ended(&mut rx).await;
        let mid = state_snapshot(State(app.clone())).await;
        assert_eq!(mid.0.player.xp, 5, "the stray pays its 5 xp");
        assert_eq!(
            mid.0.player.level, 1,
            "5 xp stays below the first threshold — no premature level"
        );

        for step in ["north", "up", "up"] {
            let moved = command(State(app.clone()), req(step)).await;
            assert!(moved.0.accepted, "move {step} accepted");
        }
        assert!(
            command(State(app.clone()), req("confront"))
                .await
                .0
                .accepted
        );
        let _ = drain_combat_until_ended(&mut rx).await;
        let mut levels = Vec::new();
        while levels.len() < 2 {
            let event = tokio::time::timeout(Duration::from_mins(5), rx.recv())
                .await
                .expect("the level burst follows the boss victory")
                .expect("broadcast stays open");
            if let GameEventKind::LevelUp { level, max_hp } = event.kind {
                levels.push((level, max_hp));
            }
        }
        assert_eq!(
            levels,
            vec![(2, 25), (3, 30)],
            "one event per level, ascending, in the boss victory burst"
        );
        let state = state_snapshot(State(app.clone())).await;
        assert_eq!(
            state.0.player.xp, 30,
            "5 + 25 lands exactly on the threshold"
        );
        assert_eq!(state.0.player.level, 3);
        assert_eq!(state.0.player.max_hp, 30, "two levels grow 20 -> 30");
        assert_eq!(state.0.player.hp, 30, "healed to the final max");
    }

    // F-T26 (ticket #31, REQ-006): the played economy fight — the pool
    // spends on the road stray, settles a change-tack at the boss, keeps the
    // remainder through victory, and rests back to full, all over the seam.
    #[tokio::test(start_paused = true)]
    async fn beginner_slice_plays_the_focus_economy() {
        let app = test_app_state();
        assert!(
            command(State(app.clone()), req("talk mara"))
                .await
                .0
                .accepted
        );
        assert!(command(State(app.clone()), req("swear")).await.0.accepted);

        walk_to_ashen_road(&app).await;
        let mut rx = app.events.subscribe();
        let attack = command(State(app.clone()), req("attack stray")).await;
        assert!(attack.0.accepted, "the road stray engages");
        assert_eq!(attack.0.snapshot.player.focus, 5, "attacking is free");
        let strike = command(State(app.clone()), req("power strike")).await;
        assert_eq!(
            strike.0.snapshot.player.focus, 3,
            "the strike costs 2 over the seam"
        );
        spawn_tick_loop(app.clone());
        let kinds = drain_combat_until_ended(&mut rx).await;
        assert!(
            matches!(
                kinds.last(),
                Some(GameEventKind::CombatEnded {
                    outcome: CombatOutcome::Victory,
                    ..
                })
            ),
            "the strike fells the stray: {kinds:?}"
        );

        for step in ["north", "up", "up"] {
            let moved = command(State(app.clone()), req(step)).await;
            assert!(moved.0.accepted, "move {step} accepted");
        }
        let confront = command(State(app.clone()), req("confront")).await;
        assert!(confront.0.accepted, "confront engages the Bell-Eater");
        assert_eq!(
            confront.0.snapshot.player.focus, 3,
            "the pool stays spent between fights"
        );
        let guard = command(State(app.clone()), req("guard")).await;
        assert_eq!(guard.0.snapshot.player.focus, 2, "the guard costs 1");
        let switched = command(State(app.clone()), req("power strike")).await;
        assert!(
            switched.0.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage {
                    component: OutputComponent::CombatMessage,
                    text
                } if text == "You change tack. You wind up a power strike."
            )),
            "the change-tack settles over the seam: {:?}",
            switched.0.events
        );
        assert_eq!(
            switched.0.snapshot.player.focus, 1,
            "refund the guard, charge the strike"
        );
        // A fresh subscription for the boss pulses: the test channel holds 16
        // events, so a receiver parked across the walk + queue commands lags
        // out. Pulses only fire while the drain awaits (paused time), so
        // nothing is missed by subscribing here.
        drop(rx);
        let mut rx = app.events.subscribe();
        let kinds = drain_combat_until_ended(&mut rx).await;
        assert!(
            matches!(
                kinds.last(),
                Some(GameEventKind::CombatEnded {
                    outcome: CombatOutcome::Victory,
                    ..
                })
            ),
            "the strike fells the boss: {kinds:?}"
        );
        let state = state_snapshot(State(app.clone())).await;
        assert_eq!(state.0.player.focus, 1, "victory keeps the spent pool");

        let rest = command(State(app.clone()), req("rest")).await;
        assert!(rest.0.accepted, "rest recovers between fights");
        let state = state_snapshot(State(app.clone())).await;
        assert_eq!(state.0.player.focus, 5, "the pool refills to its maximum");
        assert_eq!(state.0.player.max_focus, 5);
    }

    // F-T27 (ticket #31, REQ-006/007): a crafted one-point pool loaded over
    // the real /load boundary visibly limits the played boss fight — the
    // strike refuses with the typed line, the guard fits, the pulse loop
    // still wins, and the spent point stays spent.
    #[tokio::test(start_paused = true)]
    async fn crafted_low_focus_limits_the_served_boss_fight() {
        let app = test_app_state_with_saves("focus-crafted-refusal");
        std::fs::remove_dir_all(app.saves.root()).ok();
        assert!(
            command(State(app.clone()), req("talk mara"))
                .await
                .0
                .accepted
        );
        assert!(command(State(app.clone()), req("swear")).await.0.accepted);
        for step in ["north", "north", "north", "up", "up"] {
            let moved = command(State(app.clone()), req(step)).await;
            assert!(moved.0.accepted, "move {step} accepted");
        }

        let mut data = { app.engine.lock().await.save_data() };
        data.state.player.focus = 1;
        app.saves
            .write_json("crafted", &data)
            .expect("the crafted slot writes");
        let loaded = load(State(app.clone()), slot_request(Some("crafted"))).await;
        assert!(loaded.0.ok, "the crafted slot loads: {:?}", loaded.0.error);

        let confront = command(State(app.clone()), req("confront")).await;
        assert!(confront.0.accepted, "confront engages the Bell-Eater");
        let strike = command(State(app.clone()), req("power strike")).await;
        assert!(!strike.0.accepted, "one point cannot buy a strike");
        assert!(
            strike.0.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage {
                    component: OutputComponent::SystemMessage,
                    text
                } if text == "You lack the focus for a power strike."
            )),
            "the typed refusal crosses the seam: {:?}",
            strike.0.events
        );
        assert_eq!(strike.0.snapshot.player.focus, 1, "nothing spent");
        assert_eq!(
            strike
                .0
                .snapshot
                .combat
                .as_ref()
                .expect("the fight is on")
                .queued_action,
            None,
            "nothing queued"
        );
        let guard = command(State(app.clone()), req("guard")).await;
        assert!(guard.0.accepted, "the pool still affords a guard");
        assert_eq!(guard.0.snapshot.player.focus, 0);
        // Subscribe right before the drain (the 16-slot test channel lags a
        // parked receiver); the pulses only fire while the drain awaits.
        let mut rx = app.events.subscribe();
        spawn_tick_loop(app.clone());
        let kinds = drain_combat_until_ended(&mut rx).await;
        assert!(
            matches!(
                kinds.last(),
                Some(GameEventKind::CombatEnded {
                    outcome: CombatOutcome::Victory,
                    ..
                })
            ),
            "the baseline still fells the boss: {kinds:?}"
        );
        let state = state_snapshot(State(app.clone())).await;
        assert_eq!(state.0.player.focus, 0, "the spent point stays spent");
        std::fs::remove_dir_all(app.saves.root()).ok();
    }

    // M-T11 (ticket #33, REQ-005): the marker lifecycle over the served seam —
    // the wax stub flags the square at start; discovering the road reveals the
    // stray's ember flag; felling it clears the hostile flag and the dropped
    // fang raises the loot flag; taking the fang clears that; the undiscovered
    // tower stays dark throughout.
    #[tokio::test]
    async fn map_marker_flags_track_the_served_fight() {
        let app = test_app_state();
        let room = |snapshot: &GameSnapshot, id: &str| {
            snapshot
                .map
                .rooms
                .iter()
                .find(|room| room.id == id)
                .expect("the room is on the map")
                .clone()
        };

        let start = state_snapshot(State(app.clone())).await;
        let square = room(&start.0, "hollowmere_square");
        assert!(square.has_items, "the wax stub flags the square at start");
        assert!(!square.has_hostiles, "no hostile in town");
        let road = room(&start.0, "ashen_road");
        assert!(!road.discovered, "the road begins fogged");
        assert!(!road.has_hostiles, "fog conceals the stray");

        for step in ["north", "north"] {
            assert!(command(State(app.clone()), req(step)).await.0.accepted);
        }
        let arrived = state_snapshot(State(app.clone())).await;
        let road = room(&arrived.0, "ashen_road");
        assert!(road.has_hostiles, "the discovered road flags the stray");
        assert!(!road.has_items, "no ground items before the fight");

        // The 9hp stray falls to three manual 4-damage strikes.
        assert!(
            command(State(app.clone()), req("attack stray"))
                .await
                .0
                .accepted
        );
        assert!(command(State(app.clone()), req("attack")).await.0.accepted);
        let won = command(State(app.clone()), req("attack")).await;
        assert!(
            won.0.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::CombatEnded {
                    outcome: CombatOutcome::Victory,
                    ..
                }
            )),
            "three strikes fell the stray: {:?}",
            won.0.events
        );
        let after = room(&won.0.snapshot, "ashen_road");
        assert!(!after.has_hostiles, "victory clears the ember flag");
        assert!(after.has_items, "the dropped fang raises the loot flag");

        let took = command(State(app.clone()), req("take fang")).await;
        assert!(took.0.accepted, "the fang is takeable");
        let cleared = room(&took.0.snapshot, "ashen_road");
        assert!(!cleared.has_items, "taking the fang clears the loot flag");

        let tower = room(&took.0.snapshot, "tower_foot");
        assert!(
            !tower.discovered && !tower.has_hostiles && !tower.has_items,
            "the unvisited tower stays dark throughout"
        );
    }

    // C-T12 (ticket #34, REQ-005/006): the played shop loop over the seam —
    // fight the stray (+4 coins, fang drops), take the spoils, walk back to
    // Mara's counter, sell the fang (+3) and the wax stub (+1), and buy the
    // candle at exactly 8 — the authored economy lands the loop on zero.
    #[tokio::test]
    async fn played_shop_loop_funds_the_candle() {
        let app = test_app_state();
        let coins = |snapshot: &GameSnapshot| snapshot.player.coins;

        // The square's wax stub is the pocket-change pickup.
        assert!(
            command(State(app.clone()), req("take wax stub"))
                .await
                .0
                .accepted
        );

        // North to the road; three manual strikes fell the 9hp stray.
        for step in ["north", "north"] {
            assert!(command(State(app.clone()), req(step)).await.0.accepted);
        }
        assert!(
            command(State(app.clone()), req("attack stray"))
                .await
                .0
                .accepted
        );
        assert!(command(State(app.clone()), req("attack")).await.0.accepted);
        let won = command(State(app.clone()), req("attack")).await;
        assert!(
            won.0.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::CombatEnded {
                    outcome: CombatOutcome::Victory,
                    ..
                }
            )),
            "the stray falls: {:?}",
            won.0.events
        );
        assert_eq!(coins(&won.0.snapshot), 4, "the victory faucet pays 4");
        assert!(
            command(State(app.clone()), req("take fang"))
                .await
                .0
                .accepted
        );

        // Back to the square, east into the candle shop.
        for step in ["south", "south", "east"] {
            assert!(command(State(app.clone()), req(step)).await.0.accepted);
        }

        let listed = command(State(app.clone()), req("shop")).await;
        assert!(listed.0.accepted, "Mara's counter lists");
        let sold_fang = command(State(app.clone()), req("sell fang")).await;
        assert!(sold_fang.0.accepted);
        assert_eq!(coins(&sold_fang.0.snapshot), 7, "4 + 3 for the fang");
        let sold_stub = command(State(app.clone()), req("sell wax stub")).await;
        assert!(sold_stub.0.accepted);
        assert_eq!(coins(&sold_stub.0.snapshot), 8, "+1 for the stub");

        let bought = command(State(app.clone()), req("buy candle")).await;
        assert!(bought.0.accepted, "{:?}", bought.0.events);
        assert_eq!(
            coins(&bought.0.snapshot),
            0,
            "the loop lands on exactly zero"
        );
        assert!(
            bought
                .0
                .snapshot
                .pack
                .iter()
                .any(|item| item.id == "candle"),
            "the candle is carried"
        );

        // Mara's stock now holds the player's spoils — the two-way economy.
        let relisted = command(State(app.clone()), req("shop")).await;
        let listing = relisted
            .0
            .events
            .iter()
            .find_map(|e| match &e.kind {
                GameEventKind::LogMessage { text, .. } => Some(text.clone()),
                _ => None,
            })
            .expect("a listing line");
        assert!(
            listing.contains("Cracked Fang") && listing.contains("Wax Stub"),
            "the sold goods joined the stock: {listing}"
        );
        assert!(
            !listing.contains("Black Candle"),
            "the bought candle left the stock: {listing}"
        );
    }

    /// Drive one accepted command over the seam — the played-loop tests'
    /// shared step (ticket #35; keeps each loop under the line ceiling).
    async fn played(app: &AppState, input: &str) -> CommandResponse {
        let response = command(State(app.clone()), req(input)).await.0;
        assert!(
            response.accepted,
            "'{input}' should be accepted: {:?}",
            response.events
        );
        response
    }

    /// The `(slot, id)` pairs of the snapshot's equipped gear.
    fn equipped_pairs(response: &CommandResponse) -> Vec<(String, String)> {
        response
            .snapshot
            .player
            .equipment
            .iter()
            .map(|entry| (entry.slot.clone(), entry.id.clone()))
            .collect()
    }

    // EQ1 (ticket #35, REQ-001/003/005/006): the played gear loop over the
    // seam — earn, buy the blade, wield it, fell the boss on strike-6 lines,
    // buy and wear the coat, and save/load keeps both slots filled.
    #[tokio::test]
    async fn played_gear_loop_arms_the_oathbearer() {
        let app = test_app_state_with_saves("eq1-gear-loop");
        std::fs::remove_dir_all(app.saves.root()).ok();

        // Swear the oath at the counter, pocket the stub on the way out.
        for step in ["take wax stub", "east", "talk mara", "swear", "west"] {
            played(&app, step).await;
        }

        // Fund the blade: stray coins (4) + fang (3) + stub (1) = 8 − 6 = 2.
        for step in ["north", "north", "attack stray", "attack", "attack"] {
            played(&app, step).await;
        }
        for step in [
            "take fang",
            "south",
            "south",
            "east",
            "sell fang",
            "sell wax stub",
        ] {
            played(&app, step).await;
        }
        let bought = played(&app, "buy blade").await;
        assert_eq!(bought.snapshot.player.coins, 2, "8 − 6 leaves change");

        // Wield it: the snapshot lists the weapon slot.
        let wielded = played(&app, "equip blade").await;
        assert_eq!(
            equipped_pairs(&wielded),
            vec![("weapon".to_string(), "rust_edge_blade".to_string())]
        );

        // The boss falls in two strike-6 blows instead of three.
        for step in ["west", "north", "north", "north", "up", "up"] {
            played(&app, step).await;
        }
        let opened = played(&app, "confront").await;
        assert!(
            opened.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage { text, .. }
                    if text.contains("You strike The Bell-Eater for 6 (6/12).")
            )),
            "the blade raises the opening strike: {:?}",
            opened.events
        );
        let felled = played(&app, "attack").await;
        assert!(
            felled.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::CombatEnded {
                    outcome: CombatOutcome::Victory,
                    ..
                }
            )),
            "two armed strikes fell 12 hp: {:?}",
            felled.events
        );
        assert_eq!(felled.snapshot.player.coins, 27, "2 + the boss's 25");

        // Back to Mara for the coat; wear it over the blade.
        for step in [
            "down", "down", "south", "south", "south", "east", "buy coat",
        ] {
            played(&app, step).await;
        }
        let worn = played(&app, "wear coat").await;
        assert_eq!(
            equipped_pairs(&worn),
            vec![
                ("weapon".to_string(), "rust_edge_blade".to_string()),
                ("armor".to_string(), "waxed_coat".to_string()),
            ],
            "both slots filled, weapon first"
        );

        // The gear survives the save/load round-trip byte-for-byte.
        let saved = save(State(app.clone()), slot_request(Some("geared"))).await;
        assert!(saved.0.ok, "save accepted: {:?}", saved.0.error);
        let at_save = state_value(&app).await;
        played(&app, "unequip weapon").await;
        let loaded = load(State(app.clone()), slot_request(Some("geared"))).await;
        assert!(loaded.0.ok, "load accepted: {:?}", loaded.0.error);
        assert_eq!(
            state_value(&app).await,
            at_save,
            "the restored /state still wears the blade and coat"
        );
        std::fs::remove_dir_all(app.saves.root()).ok();
    }
}
