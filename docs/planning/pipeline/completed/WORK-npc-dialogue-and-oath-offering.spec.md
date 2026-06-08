---
pipeline_id: 1ed7b10e-1b8f-4d12-b223-ee09063af5b8
title: WORK-npc-dialogue-and-oath-offering
ticket: 8a66fea8-56eb-4015-b445-2608b8c4ddbf
type: work
intake:
notes: WORK-npc-dialogue-and-oath-offering.notes.md
status: Phase 5 — Complete PASS
---

# WORK-npc-dialogue-and-oath-offering

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** NPC Dialogue + Oath Offering V1 — `talk mara` returns authored
  dialogue from module TOML and offers the Hollow Bell oath; `swear`/`vow` then
  binds the *offered* oath instead of a contextless module-global swear.
- **Scope:**
  - **In:** minimal per-NPC dialogue metadata authored in the beginner module
    TOML (deserialized by `oathstar-content` behind defaulted fields); Mara's
    beginner dialogue, including an authored line for the already-sworn /
    already-fulfilled states; an authored "oath offered" gate reached by talking
    to the issuer at interaction distance; issuer/source metadata on the oath
    model (REQ-006); an offer-gated `swear` that refuses / guides toward the
    oath-giver before the oath is offered; reuse of the existing `OathSworn` /
    `OathFulfilled` event shapes once sworn; focused Rust tests; a minimal,
    additive client surface so the browser path (talk → swear) stays obvious.
  - **Out:** branching dialogue trees / modal conversation UI; `ask <npc> about
    <topic>`; persuasion, reputation, or region-standing changes; LLM-generated
    dialogue; shops/trade; multiple simultaneous oath offers; any combat change;
    new oath lifecycle states beyond what a test can construct. **No new
    distance/geometry logic — reuse ticket #17.**
- **Systems:** engine · content (TOML) · oath · npc/dialogue · protocol · ui

## Acceptance Criteria (EARS)
Carried verbatim from ticket #19 (already EARS-form, one observable behavior
each). Verification methods made concrete for this slice.

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a talkable NPC has dialogue metadata, `talk <npc>` shall return authored dialogue from world/module data rather than a hardcoded generic response. | Rust test (`oathstar-core` engine) |
| REQ-002 | When the player talks to Mara from an interactable distance, the response shall introduce the Hollow Bell problem and expose that an oath can be sworn. | Rust test (`oathstar-core`) + server smoke |
| REQ-003 | When the player attempts to swear before discovering/being offered the oath, the engine shall refuse or guide them toward the oath-giver instead of allowing a contextless global oath. | Rust test (`oathstar-core`) |
| REQ-004 | When the oath has been offered, `swear`/`vow` shall bind the offered oath and emit the existing oath event shape. | Rust test (`oathstar-core`; asserts `OathSworn{oath_id,title}` unchanged) |
| REQ-005 | When the oath is already sworn or fulfilled, Mara's dialogue shall reflect that state at a minimal authored level. | Rust test (`oathstar-core`) |
| REQ-006 | The oath model shall record enough issuer/source metadata to support future oath-giver UI and region/faction effects. | Rust test (model + content load) + docs |
| REQ-007 | Existing beginner oath progression to the Bell-Eater shall remain playable. | `bin/gate.sh` (cargo test + node --test) + server smoke |

## Locked-In Decisions
Fixed by the ticket + prior architecture and not to be re-litigated mid-pipeline.
(Representation choices left open for Phase 2 are listed in the notes, not here.)

- **Reuse the ticket #17 resolver and ticket #18 `talk` handler — no duplicated
  distance math.** Dialogue and the oath-offer trigger fire when `talk <npc>`
  resolves to an **interactable** actor through `awareness::resolve_target`
  (interaction radius 1; Decision 036). `talk_at` (`crates/oathstar-core/src/lib.rs:666-702`)
  is the handler to extend; Mara sits at `(1,0,0)`, one cell from the start room
  `(0,0,0)`, so REQ-002's "from an interactable distance" is the existing
  `Proximity::Interactable` gate, not a new check.
- **Content is TOML-authored; the engine stays generic (Decisions 022, 004).**
  Mara's dialogue and the oath's issuer/source metadata are authored in the
  beginner module TOML and deserialized by `oathstar-content` behind
  `#[serde(default)]` fields so existing modules still load. No Mara-specific
  `if id == "mara"` branch in the engine beyond the smallest honest v1 bridge —
  dialogue is data keyed off entity/oath metadata plus oath state (Decision 004:
  a role's behavior comes from its declared metadata, not hardcoding).
- **Preserve the oath event wire shapes (ticket + Decisions 028/031).**
  `OathSworn{oath_id,title}` and `OathFulfilled{oath_id}` keep their current
  fields and snake_case `type`-tag wire form. Any new metadata (issuer/source,
  offered-state, dialogue) is **additive** — new optional fields, or a new typed
  event/snapshot field with `#[serde(default, skip_serializing_if = …)]` — never a
  change to the two existing event shapes.
- **The oath model gains authored issuer/source metadata (REQ-006).**
  `OathDefinition` (today `{id,title,description}`, `lib.rs:136-140`) grows
  optional fields naming the oath-giver and enough region/faction context for
  future oath-giver UI and effects. Fields are optional + defaulted so existing
  oaths and the `[[oaths]]` TOML stay valid; the exact field set is a Phase-2
  choice.
- **"Offered" is an authored, discoverable gate — not a relationship meter
  (Decisions 005, 006).** Pre-swear the oath is *available*; being *offered* is
  reached by talking to the issuer. `swear` requires the designated oath to have
  been offered (REQ-003); the contextless module-global swear is replaced by an
  offer-gated swear. Movement/geography stay ungated — the **offer** gates
  `swear`, mirroring how the sworn oath gates `confront`. (Whether "offered" is a
  `GameState` field, NPC-memory flag, or oath-state value is a Phase-2 choice.)
- **100% MSI discipline — no uncoverable mutants (`MUT_MSI_MIN=100`; Decision 031
  precedent).** Every new branch, enum variant, and field-bearing event/struct
  must be reachable and killable by a test. Prefer `expect`-invariants guaranteed
  by `validate()` (the `current_room()` precedent) over unreachable defensive
  arms, and do **not** introduce an unconstructed variant (the reason `OathBroken`
  was deliberately omitted). A new construction-boundary validation (e.g. a
  dangling issuer reference, Decision 030) is added only if a test can construct
  the failing world.
- **Browser-playable, server-authoritative, minimal client (Decisions
  015/032/034).** The manual path stays obvious — **`talk mara` → `swear`**.
  Client edits are minimal and additive (at most an `intent.js` contextual-hint /
  snapshot tweak so the UI guides "talk to the oath-giver first"), rendered
  XSS-safe via `textContent`. No dialogue modal/branching UI.
- **No parser expansion (Decision 002).** `talk`/`speak`, `swear`/`vow`, and
  `confront`/`challenge` already parse to typed commands (`command.rs`). #19
  reuses `Command::Talk` and `Command::Swear`; it adds no new verbs.

## Linked Artifacts
- Design docs: `docs/decisions.md` (001 oaths-are-quests++, 004 role contracts /
  code-behind, 005 oath lifecycle, 006 NPC memory & "discoverable through NPCs",
  022 TOML-first, 028/031 typed events + wire split, 030 construction-boundary
  validation, 036 awareness); `docs/mechanics-and-systems.md` (NPCs And Dialogue —
  command-based dialogue for the first slice); `docs/entity-model.md` (role
  metadata); `docs/spatial-awareness.md` (talk reuses the resolver);
  `docs/protocol-and-output.md` (event/snapshot shapes).
- Intake doc: none.
- Ticket doc: `docs/planning/tickets/open/TICKET-19-npc-dialogue-and-oath-offering-v1.md`
- Forge ticket: `8a66fea8-56eb-4015-b445-2608b8c4ddbf` (#19)
- AAR: `bcfdb840-0d77-409e-a6e4-6ea1a821bb17`

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS (design + test plan in notes) |
| 3 — Implement | PASS (code compiles; fmt+check clean; content load tests green) |
| 3.5 — Inspect | PASS (4 critics; 1 real fix: validate too_many_lines → validate_oaths; clippy green) |
| 4 — Validate | PASS (+15 tests; gate --fast 14/14 green; cov 99.6% / 100% fn) |
| 5 — Complete | PASS (Decision 037; AAR+ADR+failure+rule captured; ticket #19 done; archived) |
