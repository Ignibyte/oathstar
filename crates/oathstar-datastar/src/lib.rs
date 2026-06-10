//! Datastar presentation boundary for Oathstar (ticket #15).
//!
//! Maps `oathstar-protocol` game events into Datastar-compatible SSE HTML
//! fragments for the first-party player client (Decision 034). This crate is the
//! *only* place Datastar's wire format lives; `oathstar-core` must never depend on
//! it. It depends solely on `oathstar-protocol` for the event types.
//!
//! The player-client event feed (`#log`) is rendered here as escaped, single-line
//! `<article>` fragments delivered via the Datastar `datastar-patch-elements` SSE
//! event (`mode append`). All server-provided text is HTML-escaped (REQ-005) so it
//! cannot inject markup.

use oathstar_protocol::{EventChannel, GameEvent, GameEventKind, OutputComponent};

/// The SSE `event:` name Datastar consumes to patch DOM elements (Datastar v1.0.x).
pub const PATCH_ELEMENTS_EVENT: &str = "datastar-patch-elements";

/// CSS selector for the player-client event-feed container the feed appends into.
pub const FEED_SELECTOR: &str = "#log";

/// A Datastar SSE patch: the `event:` name and the `\n`-joined `data:` body.
///
/// Axum's `Sse` splits `Event::data` on `\n` into one `data:` line each, which
/// reproduces Datastar's multi-line `selector` / `mode` / `elements` framing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatastarPatch {
    /// The SSE event name (always [`PATCH_ELEMENTS_EVENT`]).
    pub event: &'static str,
    /// The `\n`-joined `data:` body (`selector …\nmode …\nelements …`).
    pub data: String,
}

/// Escape text for safe interpolation into HTML (REQ-005).
///
/// `&` is escaped first so the entities introduced by the later replacements are
/// not double-escaped.
pub fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Collapse CR/LF to spaces so a feed fragment stays on one line.
///
/// Fragments must be single-line, else the SSE `data: elements …` framing would
/// be split into an unkeyed continuation line that Datastar cannot parse.
fn single_line(input: &str) -> String {
    input.replace(['\n', '\r'], " ")
}

/// Render one game event as an escaped, single-line HTML feed fragment, or `None`
/// for events that do not belong in the feed (currently [`GameEventKind::Tick`]).
///
/// The fragment mirrors the player-client feed-entry markup
/// (`<article class="log-entry …">`) so the existing styles apply.
pub fn render_feed_fragment(event: &GameEvent) -> Option<String> {
    let (variant, label, text) = describe(event)?;
    let id = event.event_id;
    let channel = channel_str(&event.channel);
    let ty = kind_type(&event.kind);
    let body = single_line(&escape_html(&text));
    // SECURITY (REQ-005): only fixed `&'static str` (variant/label/channel/ty) and
    // the numeric `id` are interpolated into attributes; `body` is the only
    // user-derived value and is escaped + single-lined. Any future String field
    // placed in an attribute MUST be escaped for its quoted context first.
    Some(format!(
        r#"<article class="log-entry {variant}" id="event-{id}" data-event-id="{id}" data-channel="{channel}" data-component="{ty}"><span class="log-meta">{label}</span><p>{body}</p></article>"#
    ))
}

/// Build the Datastar `datastar-patch-elements` patch that appends one event's
/// fragment into the feed container, or `None` for non-feed events.
pub fn feed_patch(event: &GameEvent) -> Option<DatastarPatch> {
    let fragment = render_feed_fragment(event)?;
    Some(DatastarPatch {
        event: PATCH_ELEMENTS_EVENT,
        data: format!("selector {FEED_SELECTOR}\nmode append\nelements {fragment}"),
    })
}

/// Whether a new feed subscription should be seeded with the opening scene.
///
/// Returns `false` when the client's `Last-Event-ID` shows it already holds the
/// opening scene (reconnect dedup); `true` on a fresh or behind connection, or
/// when the header is absent/unparseable.
pub fn should_seed_opening(last_event_id: Option<&str>, last_opening_id: u64) -> bool {
    last_event_id
        .and_then(|id| id.trim().parse::<u64>().ok())
        .is_none_or(|seen| seen < last_opening_id)
}

/// The opening-scene patches to seed onto a new feed subscription.
///
/// Empty when the client already has the opening scene (see
/// [`should_seed_opening`]); otherwise one `(event_id, patch)` per feed-visible
/// opening event (ticks are skipped).
pub fn opening_patches(
    opening: &[GameEvent],
    last_event_id: Option<&str>,
) -> Vec<(u64, DatastarPatch)> {
    let patches: Vec<_> = opening
        .iter()
        .filter_map(|event| feed_patch(event).map(|patch| (event.event_id, patch)))
        .collect();
    let last_opening_id = patches.last().map_or(0, |(id, _)| *id);
    if !should_seed_opening(last_event_id, last_opening_id) {
        return Vec::new();
    }
    patches
}

/// The variant class, feed label, and feed text for an event, or `None` for
/// events that are not shown in the feed.
fn describe(event: &GameEvent) -> Option<(&'static str, &'static str, String)> {
    match &event.kind {
        // Ticks and combat-pulse markers never reach the feed: the pulse's
        // exchange messages narrate the cycle (ticket #24), so a 2s marker line
        // would only spam the log (Decision 023's rationale).
        GameEventKind::Tick { .. } | GameEventKind::CombatPulse { .. } => None,
        GameEventKind::LogMessage { component, text } => {
            let (variant, label) = component_variant_label(component);
            Some((variant, label, text.clone()))
        }
        GameEventKind::RoomEntered { title, .. } => {
            let (variant, label) = channel_variant_label(&event.channel);
            let text = if title.is_empty() {
                "You enter a new room.".to_owned()
            } else {
                format!("You enter {title}.")
            };
            Some((variant, label, text))
        }
        GameEventKind::OathSworn { title, .. } => {
            let (variant, label) = channel_variant_label(&event.channel);
            let text = if title.is_empty() {
                "You swear an oath.".to_owned()
            } else {
                format!("Oath sworn: {title}.")
            };
            Some((variant, label, text))
        }
        GameEventKind::OathFulfilled { .. } => {
            let (variant, label) = channel_variant_label(&event.channel);
            Some((variant, label, "Your oath is fulfilled.".to_owned()))
        }
        GameEventKind::CombatStarted { text, .. } | GameEventKind::CombatEnded { text, .. } => {
            // Both combat lifecycle markers carry their own feed text (ticket #22),
            // rendered on the Combat channel like the combat-message play-by-play.
            let (variant, label) = channel_variant_label(&event.channel);
            Some((variant, label, text.clone()))
        }
    }
}

/// The `snake_case` `type` tag for an event kind (the `data-component` attribute).
const fn kind_type(kind: &GameEventKind) -> &'static str {
    match kind {
        GameEventKind::LogMessage { .. } => "log_message",
        GameEventKind::Tick { .. } => "tick",
        GameEventKind::RoomEntered { .. } => "room_entered",
        GameEventKind::OathSworn { .. } => "oath_sworn",
        GameEventKind::OathFulfilled { .. } => "oath_fulfilled",
        GameEventKind::CombatStarted { .. } => "combat_started",
        GameEventKind::CombatEnded { .. } => "combat_ended",
        GameEventKind::CombatPulse { .. } => "combat_pulse",
    }
}

/// The `snake_case` channel name (the `data-channel` attribute).
const fn channel_str(channel: &EventChannel) -> &'static str {
    match channel {
        EventChannel::Narrative => "narrative",
        EventChannel::Room => "room",
        EventChannel::Combat => "combat",
        EventChannel::Loot => "loot",
        EventChannel::Skill => "skill",
        EventChannel::Oath => "oath",
        EventChannel::Region => "region",
        EventChannel::Inventory => "inventory",
        EventChannel::Equipment => "equipment",
        EventChannel::System => "system",
        EventChannel::Dm => "dm",
        EventChannel::Debug => "debug",
    }
}

/// The feed variant class + label for a channel (matches the player-client catalog).
const fn channel_variant_label(channel: &EventChannel) -> (&'static str, &'static str) {
    match channel {
        EventChannel::Room => ("room", "Room"),
        EventChannel::Oath => ("success", "Oath"),
        EventChannel::Combat => ("danger", "Combat"),
        EventChannel::Narrative => ("dialogue", "Narrative"),
        EventChannel::Region => ("system", "Region"),
        EventChannel::Loot => ("success", "Loot"),
        EventChannel::Skill => ("system", "Skill"),
        EventChannel::Inventory => ("system", "Pack"),
        EventChannel::Equipment => ("system", "Gear"),
        EventChannel::System => ("system", "System"),
        EventChannel::Dm => ("dialogue", "Storyteller"),
        EventChannel::Debug => ("system", "Debug"),
    }
}

/// The feed variant class + label for a log-message component (overrides channel).
const fn component_variant_label(component: &OutputComponent) -> (&'static str, &'static str) {
    match component {
        OutputComponent::RoomHeader => ("room", "Room"),
        OutputComponent::NarrativeMessage => ("dialogue", "Narrative"),
        OutputComponent::SystemMessage => ("system", "System"),
        OutputComponent::CombatMessage => ("danger", "Combat"),
        OutputComponent::OathCard => ("success", "Oath"),
        OutputComponent::RegionStandingCard => ("system", "Region"),
        OutputComponent::EntityChip => ("system", "Nearby"),
        OutputComponent::ItemCard => ("success", "Item"),
        OutputComponent::MapPatch => ("system", "Map"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        escape_html, feed_patch, kind_type, opening_patches, render_feed_fragment,
        should_seed_opening, PATCH_ELEMENTS_EVENT,
    };
    use oathstar_protocol::{
        CombatOutcome, EventChannel, GameEvent, GameEventKind, OutputComponent,
    };

    fn log(text: &str) -> GameEvent {
        GameEvent {
            event_id: 7,
            tick: 3,
            channel: EventChannel::Narrative,
            kind: GameEventKind::LogMessage {
                component: OutputComponent::NarrativeMessage,
                text: text.to_owned(),
            },
        }
    }

    #[test]
    fn escape_html_neutralizes_markup() {
        let out = escape_html(r#"<script>alert("x&y")</script>'"#);
        assert_eq!(
            out,
            "&lt;script&gt;alert(&quot;x&amp;y&quot;)&lt;/script&gt;&#39;"
        );
        assert!(!out.contains('<'), "no raw < survives");
        assert!(!out.contains('>'), "no raw > survives");
    }

    #[test]
    fn escape_html_orders_ampersand_first() {
        // & must be escaped before the entities it introduces, else the output
        // double-escapes; pin every replacement so a dropped one fails here.
        assert_eq!(escape_html("a&b"), "a&amp;b");
        assert_eq!(escape_html("<>"), "&lt;&gt;");
        assert_eq!(escape_html("\"'"), "&quot;&#39;");
        assert_eq!(escape_html("plain text"), "plain text");
    }

    #[test]
    fn ticks_have_no_feed_fragment() {
        let tick = GameEvent {
            event_id: 1,
            tick: 1,
            channel: EventChannel::Debug,
            kind: GameEventKind::Tick { value: 9 },
        };
        assert!(render_feed_fragment(&tick).is_none());
        assert!(feed_patch(&tick).is_none());
    }

    // D1 (ticket #24): combat-pulse markers never reach the feed (the exchange
    // lines narrate the cycle); the type tag is pinned directly because the
    // render path short-circuits before kind_type.
    #[test]
    fn combat_pulses_have_no_feed_fragment() {
        let pulse = GameEvent {
            event_id: 3,
            tick: 4,
            channel: EventChannel::Combat,
            kind: GameEventKind::CombatPulse { round: 2 },
        };
        assert!(render_feed_fragment(&pulse).is_none());
        assert!(feed_patch(&pulse).is_none());
        assert_eq!(
            kind_type(&GameEventKind::CombatPulse { round: 2 }),
            "combat_pulse"
        );
    }

    // D2 (ticket #24): a fled CombatEnded renders like the other combat
    // lifecycle markers — danger variant on the combat channel, body = text.
    #[test]
    fn fled_combat_ended_renders_on_the_combat_channel() {
        let ended = GameEvent {
            event_id: 5,
            tick: 8,
            channel: EventChannel::Combat,
            kind: GameEventKind::CombatEnded {
                outcome: CombatOutcome::Fled,
                text: "You break away from Stray and escape.".to_string(),
            },
        };
        let html = render_feed_fragment(&ended).expect("a fled end renders");
        assert!(
            html.contains(r#"class="log-entry danger""#),
            "variant: {html}"
        );
        assert!(
            html.contains(r#"data-component="combat_ended""#),
            "type: {html}"
        );
        assert!(html.contains(r#"data-channel="combat""#), "channel: {html}");
        assert!(
            html.contains("You break away from Stray and escape."),
            "body: {html}"
        );
    }

    #[test]
    fn log_fragment_escapes_and_wraps() {
        let html = render_feed_fragment(&log("<b>hi</b>")).expect("log renders");
        assert!(html.contains(r#"id="event-7""#), "stable id: {html}");
        assert!(
            html.contains(r#"class="log-entry dialogue""#),
            "variant: {html}"
        );
        assert!(
            html.contains(r#"data-component="log_message""#),
            "type: {html}"
        );
        assert!(
            html.contains(r#"data-channel="narrative""#),
            "channel: {html}"
        );
        assert!(html.contains("&lt;b&gt;hi&lt;/b&gt;"), "escaped: {html}");
        assert!(!html.contains("<b>hi"), "no raw markup: {html}");
    }

    #[test]
    fn room_and_oath_render_escaped() {
        let room = GameEvent {
            event_id: 2,
            tick: 1,
            channel: EventChannel::Room,
            kind: GameEventKind::RoomEntered {
                room_id: "r1".to_owned(),
                title: "Hall <x>".to_owned(),
            },
        };
        let html = render_feed_fragment(&room).expect("room renders");
        assert!(
            html.contains(r#"class="log-entry room""#),
            "room variant: {html}"
        );
        assert!(
            html.contains("You enter Hall &lt;x&gt;."),
            "escaped title: {html}"
        );
        assert!(!html.contains("<x>"), "no raw markup: {html}");

        let sworn = GameEvent {
            event_id: 5,
            tick: 1,
            channel: EventChannel::Oath,
            kind: GameEventKind::OathSworn {
                oath_id: "o<1>".to_owned(),
                title: "A&B".to_owned(),
            },
        };
        let html = render_feed_fragment(&sworn).expect("oath renders");
        assert!(
            html.contains(r#"class="log-entry success""#),
            "oath variant: {html}"
        );
        assert!(
            html.contains("Oath sworn: A&amp;B."),
            "escaped oath title: {html}"
        );
        assert!(!html.contains("A&B"), "no raw ampersand: {html}");

        let done = GameEvent {
            event_id: 6,
            tick: 2,
            channel: EventChannel::Oath,
            kind: GameEventKind::OathFulfilled {
                oath_id: "o1".to_owned(),
            },
        };
        assert!(render_feed_fragment(&done)
            .expect("fulfilled renders")
            .contains("Your oath is fulfilled."));
    }

    #[test]
    fn empty_titles_fall_back() {
        let room = GameEvent {
            event_id: 3,
            tick: 1,
            channel: EventChannel::Room,
            kind: GameEventKind::RoomEntered {
                room_id: "r".to_owned(),
                title: String::new(),
            },
        };
        assert!(render_feed_fragment(&room)
            .expect("room renders")
            .contains("You enter a new room."));

        let sworn = GameEvent {
            event_id: 4,
            tick: 1,
            channel: EventChannel::Oath,
            kind: GameEventKind::OathSworn {
                oath_id: "o".to_owned(),
                title: String::new(),
            },
        };
        assert!(render_feed_fragment(&sworn)
            .expect("oath renders")
            .contains("You swear an oath."));
    }

    #[test]
    fn fragment_is_single_line() {
        let html = render_feed_fragment(&log("line1\nline2\rline3")).expect("renders");
        assert!(!html.contains('\n'), "no LF in fragment: {html}");
        assert!(!html.contains('\r'), "no CR in fragment: {html}");
        assert!(
            html.contains("line1 line2 line3"),
            "newlines became spaces: {html}"
        );
    }

    #[test]
    fn feed_patch_uses_datastar_append() {
        let patch = feed_patch(&log("hi")).expect("patch");
        assert_eq!(patch.event, "datastar-patch-elements");
        assert_eq!(patch.event, PATCH_ELEMENTS_EVENT);
        assert!(
            patch
                .data
                .starts_with("selector #log\nmode append\nelements <article"),
            "datastar append framing: {}",
            patch.data
        );
    }

    #[test]
    fn no_feed_kind_leaks_raw_markup() {
        // Guard the REQ-005 invariant for EVERY feed-visible kind: if any field
        // ever stops routing through escape_html, a raw tag survives and this fails
        // (covers both content `<p>…</p>` and any attribute context).
        const ATTACK: &str = "</p><script>alert(1)</script><img src=x onerror=alert(1)>";
        let kinds = [
            GameEventKind::LogMessage {
                component: OutputComponent::NarrativeMessage,
                text: ATTACK.to_owned(),
            },
            GameEventKind::RoomEntered {
                room_id: ATTACK.to_owned(),
                title: ATTACK.to_owned(),
            },
            GameEventKind::OathSworn {
                oath_id: ATTACK.to_owned(),
                title: ATTACK.to_owned(),
            },
            GameEventKind::OathFulfilled {
                oath_id: ATTACK.to_owned(),
            },
        ];
        for kind in kinds {
            let event = GameEvent {
                event_id: 9,
                tick: 0,
                channel: EventChannel::Narrative,
                kind,
            };
            let html = render_feed_fragment(&event).expect("feed kind renders");
            assert!(!html.contains("<script"), "raw <script survived: {html}");
            assert!(!html.contains("<img"), "raw <img survived: {html}");
            assert!(!html.contains("</p><script"), "breakout survived: {html}");
            // The feed body carries the only user text; no raw tag char may survive there.
            let body = html
                .split("<p>")
                .nth(1)
                .and_then(|rest| rest.split("</p>").next())
                .expect("fragment has a <p> body");
            assert!(!body.contains('<'), "raw < in body: {body}");
            assert!(!body.contains('>'), "raw > in body: {body}");
        }
    }

    #[test]
    fn should_seed_opening_reads_last_event_id() {
        assert!(should_seed_opening(None, 5), "fresh load seeds");
        assert!(should_seed_opening(Some("nope"), 5), "unparseable seeds");
        assert!(should_seed_opening(Some("3"), 5), "behind seeds");
        assert!(!should_seed_opening(Some("5"), 5), "has all -> skip");
        assert!(!should_seed_opening(Some("9"), 5), "ahead -> skip");
        assert!(should_seed_opening(Some(" 4 "), 5), "trims then parses");
    }

    #[test]
    fn opening_patches_seed_then_dedup() {
        let opening = vec![
            GameEvent {
                event_id: 1,
                tick: 0,
                channel: EventChannel::Room,
                kind: GameEventKind::RoomEntered {
                    room_id: "hollowmere_square".to_owned(),
                    title: "Hollowmere Square".to_owned(),
                },
            },
            GameEvent {
                event_id: 2,
                tick: 0,
                channel: EventChannel::Debug,
                kind: GameEventKind::Tick { value: 1 },
            },
        ];
        let fresh = opening_patches(&opening, None);
        assert_eq!(fresh.len(), 1, "tick is filtered out of the seed");
        assert_eq!(fresh[0].0, 1, "carries the source event id");
        assert!(fresh[0].1.data.contains("Hollowmere Square"));
        assert!(
            opening_patches(&opening, Some("2")).is_empty(),
            "a client that has the opening is not re-seeded"
        );
        assert!(
            opening_patches(&opening, Some("1")).is_empty(),
            "dedup compares against the last feed-visible event, not skipped ticks"
        );
        assert!(opening_patches(&[], None).is_empty(), "nothing to seed");
    }

    // C21 (ticket #22): the typed combat lifecycle events render on the Combat
    // channel (danger variant), carry their own escaped text, and tag their type.
    #[test]
    fn combat_events_render_on_the_combat_channel() {
        let started = GameEvent {
            event_id: 11,
            tick: 4,
            channel: EventChannel::Combat,
            kind: GameEventKind::CombatStarted {
                enemy_id: "stray".to_owned(),
                enemy_name: "Stray".to_owned(),
                text: "Stray turns on you <ready>".to_owned(),
            },
        };
        let html = render_feed_fragment(&started).expect("combat_started renders");
        assert!(
            html.contains(r#"class="log-entry danger""#),
            "danger variant: {html}"
        );
        assert!(
            html.contains(r#"data-channel="combat""#),
            "combat channel: {html}"
        );
        assert!(
            html.contains(r#"data-component="combat_started""#),
            "combat_started type tag: {html}"
        );
        assert!(
            html.contains("Stray turns on you &lt;ready&gt;"),
            "escaped text body: {html}"
        );
        assert!(!html.contains("<ready>"), "no raw markup: {html}");

        let ended = GameEvent {
            event_id: 12,
            tick: 5,
            channel: EventChannel::Combat,
            kind: GameEventKind::CombatEnded {
                outcome: oathstar_protocol::CombatOutcome::Victory,
                text: "You have defeated Stray. Victory!".to_owned(),
            },
        };
        let html = render_feed_fragment(&ended).expect("combat_ended renders");
        assert!(
            html.contains(r#"data-component="combat_ended""#),
            "combat_ended type tag: {html}"
        );
        assert!(
            html.contains(r#"class="log-entry danger""#),
            "danger variant: {html}"
        );
        assert!(
            html.contains("You have defeated Stray. Victory!"),
            "summary text in the feed: {html}"
        );
    }
}
