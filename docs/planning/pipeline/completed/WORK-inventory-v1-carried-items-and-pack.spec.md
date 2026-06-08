---
pipeline_id: 3e0656f0-ffb8-47fb-a4e6-e154907296c2
title: WORK-inventory-v1-carried-items-and-pack
ticket: ec4a28af-73db-4614-b4f8-04870e420b3e
type: work
intake:
notes: WORK-inventory-v1-carried-items-and-pack.notes.md
status: Phase 5 — Complete PASS
---

# WORK-inventory-v1-carried-items-and-pack

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Inventory V1 — durable carried items (`pack`), an `inventory`/`pack`/`i`
  list, carried-item `look`, a `drop` that places the item in the current cell
  (visible through #17 awareness), and an enriched pack snapshot. Extends #18's
  minimal pack; **not** the full ROT equipment model.
- **Scope:**
  - **In:** keep durable carried item ids in `GameState.pack` (REQ-001); an
    `inventory`/`pack`/`i` list command with an honest empty state (REQ-003);
    `look <item>` that resolves from the pack as well as nearby world contents
    (REQ-004); a `drop <item>` that removes the item from the pack and places it in
    the current room/cell so `awareness::perceive` surfaces it (REQ-005); an
    additive, JSON-friendly enrichment of `PackItemSnapshot` with `kind`/type
    placeholder + basic flags (REQ-002); total, non-corrupting inventory ops that
    reject unknown/missing/hidden/duplicate item refs (REQ-006); the Pack tab
    rendering server snapshot data only — no invented placeholders (REQ-007);
    focused Rust tests + a JS test for the enriched Pack render.
  - **Out:** equipment slots, weight, stacking/quantities, shops, crafting, item
    use, persistence, rarity, and elemental stats. **No full ROT equipment model.**
    **No new distance/geometry — reuse ticket #17.**
- **Systems:** parser · engine · inventory · protocol · ui

## Acceptance Criteria (EARS)
Carried verbatim from ticket #20 (already EARS-form, one observable behavior each).

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the player carries items, the engine shall store carried item ids in player/game state rather than only emitting events. | Rust test (`oathstar-core`) |
| REQ-002 | When a snapshot is produced, it shall expose a JSON-friendly pack/inventory list with item id, name, kind/type placeholder, and basic flags. | Rust + `oathstar-protocol` test |
| REQ-003 | When the player runs `inventory`/`pack`/`i`, the engine shall list carried items or an honest empty state. | Rust test (`command.rs` + `lib.rs`) |
| REQ-004 | When the player looks at a carried item, the engine shall resolve it from inventory as well as nearby world contents. | Rust test (`lib.rs`) |
| REQ-005 | When the player drops an item, the engine shall remove it from carried state and place it in the current room/cell, making it visible through spatial awareness. | Rust test (`lib.rs`) |
| REQ-006 | Inventory operations shall reject unknown, missing, hidden, or duplicate invalid item references without corrupting state. | Rust test (`lib.rs`) |
| REQ-007 | The UI Pack tab shall render server snapshot data without inventing placeholder items. | JS test (`toPack`) + browser smoke |

## Locked-In Decisions
Fixed by the ticket + prior architecture; not to be re-litigated mid-pipeline.
(Representation choices left open for Phase 2 are listed in the notes.)

- **Extend #18's `GameState.pack: Vec<String>`** (durable, pickup-ordered carried
  ids) — do not add a parallel store. `take_at` (`lib.rs:842`) already fills it;
  `pack_snapshot` (`lib.rs:1154`) maps it to the view (REQ-001).
- **Reuse the #17 awareness resolver — no new distance math.** `drop` is the
  inverse of `take_at`: it removes the item from the pack and pushes it back into
  the **current room's `items`**, after which `awareness::perceive` surfaces it at
  the player's cell (exact) with no special-casing (Decision 036; REQ-005).
  Carried-item `look` checks the `pack` first, then falls through to
  `awareness::resolve_target` (REQ-004) — a carried item has no cell, so it
  resolves from inventory.
- **Additive, JSON-friendly protocol (Decisions 028/031/034).** `PackItemSnapshot`
  (`protocol:51`, today `{id,name}`) gains a `kind`/type placeholder + basic flags
  as additive camelCase fields (`#[serde(default, skip_serializing_if = …)]` where
  empty), so old payloads still deserialize and an empty enrichment stays
  byte-identical. No change to any existing event shape.
- **Item `kind`/flags are authored data, never invented (Decisions 004/013).** The
  placeholder kind/type + flags derive from `world.items` content (the shared
  entity/item model), not from engine or client guesswork. The Pack tab renders
  the server snapshot only (REQ-002/007).
- **New parser verbs are pure forgiving-symbolic additions (Decision 002).** Add
  `Command::Drop { target }` (required target) and a bare `Command::Inventory`
  (verbs `inventory`/`pack`/`i`) alongside `Take`; verbs case-folded, target text
  preserved as `Take`/`Look` do. Unrecognized stays `Unknown`. (`intent.js` already
  advertises `inventory`; this makes it real.)
- **Inventory ops are total and non-corrupting (REQ-006).** `drop` of an item not
  in the pack, an unknown id, or a hidden/duplicate ref is refused with a clear
  event and **no** state change — mirroring `take_at`'s guard discipline. State is
  never left half-mutated.
- **100% MSI + clippy discipline.** New branches, verbs, and snapshot fields must
  be reachable + mutation-killable; keep `handle_command`/`validate` under the
  clippy `too_many_lines` ceiling by extracting per-command helpers (prevention
  rule `PR-claude-validator-length-001`, ticket #19). Deterministic engine, no RNG.
- **Browser-playable, server-authoritative, minimal client (Decisions 015/032/034).**
  The Pack tab + the already-advertised `inventory` command become real; client
  edits stay additive and XSS-safe (`textContent`). `npm run build` is run at
  validate because the UI is touched.

## Linked Artifacts
- Design docs: `docs/inventory-and-items.md` (Inventory Direction — carry/drop/
  inspect/list; ROT is the long-horizon reference, #20 is the minimal foundation);
  `docs/decisions.md` (002 parser, 004 entity/item model, 013 inventory ROT-inspired
  /slot-based [direction; slots/weight/etc. scoped out], 028/031/034 wire, 036
  awareness); `docs/spatial-awareness.md` (drop/look reuse the resolver);
  `docs/protocol-and-output.md` (snapshot/event shapes).
- Intake doc: none.
- Ticket doc: `docs/planning/tickets/open/TICKET-20-inventory-v1-carried-items-and-pack-snapshot.md`
- Forge ticket: `ec4a28af-73db-4614-b4f8-04870e420b3e` (#20)
- AAR: `85fa4f75-f922-4500-9e48-f138f3036d93`

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS (design + test plan in notes) |
| 3 — Implement | PASS (compiles; fmt+clippy green; content tests pass; parse_bare_verb extracted) |
| 3.5 — Inspect | PASS (4 critics; clippy green; 1 doc tidy; 0 code defects) |
| 4 — Validate | PASS (+13 tests; gate --fast 14/14; node 32; npm build clean; cov 99.5%; +1 Codex review regression) |
| 5 — Complete | PASS (Decision 038; AAR+ADR+failure captured; ticket #20 done; archived) |
