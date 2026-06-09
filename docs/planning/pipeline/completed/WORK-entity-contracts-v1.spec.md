---
pipeline_id: 87e381e7-93bb-4e9f-8253-7d1db07ab015
title: WORK-entity-contracts-v1
ticket: ef9c9854-e3ed-4f86-a9e6-2bd9439456b4
type: work
intake:
notes: WORK-entity-contracts-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-entity-contracts-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Entity Contracts V1 — a typed `Role` vocabulary + construction-boundary
  contract validation (typed errors naming entity/role/field), with the touched
  command handlers using typed capability helpers instead of ad-hoc role-string
  checks. Implements Decision 004's role-contract layer (currently "not yet
  implemented" per `mechanics-and-systems.md`).
- **Scope:**
  - **In:** a typed `Role` vocabulary covering the six current roles —
    `talkable`, `oath_giver`, `shopkeeper`, `combatant`, `boss`, `fixture` — parsed
    from the existing `Entity` role data; typed capability helpers on `Entity`;
    construction-boundary validation that a declared role provides the minimum
    metadata it needs **where applicable**, surfaced as a typed
    `WorldValidationError` that names the entity, the role, and the missing field
    (REQ-001/002); conversion of the two touched handlers (`talk_at`, `confront`)
    to the typed helpers (REQ-003); TOML/content so Mara is valid as
    talkable+oath_giver (REQ-004) and the Bell-Eater as combatant+boss without
    changing boss progression (REQ-005); docs defining the contract vocabulary +
    how code-behind/script hooks attach later (REQ-006); Rust + content tests per
    REQ.
  - **Out:** full code-behind/script dispatch, shop inventory/economy, advanced NPC
    memory, combat AI/turn resolution, and dynamic mod loading. **No protocol/wire
    change, no UI change, no parser change. No change to boss progression or to
    talk/take/look/oath/inventory/map behavior.** No large class hierarchy.
  - **Systems:** engine · content · docs

## Acceptance Criteria (EARS)
Carried verbatim from ticket #21.

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When an entity declares a role such as talkable/oath_giver/shopkeeper/combatant/fixture, validation shall require the minimum metadata needed by that role where applicable. | Rust test (`oathstar-core`) |
| REQ-002 | When an entity lacks required role metadata, world validation shall fail with a typed error naming the entity, role, and missing field. | Rust test (`oathstar-core`) |
| REQ-003 | When command handlers check capabilities, they shall use typed helpers/contracts rather than ad-hoc string checks wherever this ticket touches behavior. | Rust test + review (`talk_at`, `confront`) |
| REQ-004 | When Mara is declared as an oath-giver/talkable actor, her metadata shall satisfy the contract and load from TOML. | content test |
| REQ-005 | When the Bell-Eater is declared combatant/boss, its metadata shall remain valid and future-combat-ready without changing current boss progression. | content test + `bin/gate.sh` |
| REQ-006 | Docs shall define the initial contract vocabulary and explain how code-behind/script hooks will attach later. | docs review |

## Locked-In Decisions
Fixed by the ticket + prior architecture; not to be re-litigated mid-pipeline.
(Per-role metadata choices left open for Phase 2 are listed in the notes.)

- **Implement Decision 004 as typed data, not a class hierarchy** (ticket: "avoid
  turning roles into a large class hierarchy too early"). A `Role` enum maps the
  existing `Entity.roles: Vec<String>` strings (plus `EntityKind` for `fixture`)
  to a typed vocabulary; `Entity` keeps its single shared shape. Typed capability
  helpers (e.g. `Entity::has_role` / `is_talkable` / `is_boss`) are the API
  handlers call.
- **Validate at the construction boundary (Decision 030).** Role-contract checks
  live in `WorldDefinition::validate`, extracted into a focused helper (e.g.
  `validate_entity_contracts`) so `validate` stays under the clippy
  `too_many_lines` ceiling (`PR-claude-validator-length-001`; the recurring
  `BF-clippy-too-many-lines-001/002`). A new `WorldValidationError` variant names
  the **entity id, the role, and the missing field** (REQ-002). **"Minimum
  metadata where applicable"** — v1 validates only what current behavior needs;
  roles whose metadata no handler uses yet (shops are out of scope) carry a
  declared-but-minimal contract rather than inventing required fields.
- **Replace the two ad-hoc role-string checks with typed helpers (REQ-003).**
  `talk_at` (`lib.rs:820`, `role == "conversable"`) and `confront` (`lib.rs:1155`,
  `role == "boss"`) switch to the typed capability helpers. Behavior is preserved
  exactly — talk/confront/oath outputs and the boss progression are unchanged
  (REQ-005).
- **Content is TOML-authored and additive (Decisions 022, 030).** Any new
  role-metadata fields on `Entity` are optional `#[serde(default)]` so existing
  modules and saved state still load; Mara stays valid as talkable+oath_giver and
  the Bell-Eater as combatant+boss, with the same room placements and the same
  confront-to-fulfill flow.
- **No protocol / UI / parser change.** Roles and contracts are engine-internal;
  the snapshot/event wire shapes, the JS client, and the command parser are
  untouched (the explicit "preserve" list). Reuse — don't duplicate — the existing
  role data and `WorldValidationError`/`validate` machinery.
- **100% MSI + clippy discipline.** Every new `Role` variant, capability helper,
  and validation branch must be reachable and mutation-killable; prefer
  `expect`-invariants guaranteed by `validate()` over unreachable arms; run
  `cargo clippy --workspace --all-targets` in implement, not just `cargo check`.
- **Docs (REQ-006).** Extend `docs/entity-model.md` (mark the contract layer
  v1-implemented; define the typed vocabulary and each role's v1 contract) and flip
  the `docs/mechanics-and-systems.md` "role-contract validation … not yet
  implemented" note; explain how the existing code-behind hook list
  (`onTalk`/`onAttack`/…) attaches later.

## Linked Artifacts
- Design docs: `docs/decisions.md` (**004** role contracts / code-behind — the
  decision this implements; 030 construction-boundary validation; 022 TOML-first;
  028/031 wire split); `docs/entity-model.md` (the authored Roles-And-Contracts
  vocabulary + code-behind hooks — the spec source for REQ-006);
  `docs/mechanics-and-systems.md` (Entity Model v1 status note).
- Intake doc: none.
- Ticket doc: `docs/planning/tickets/open/TICKET-21-interaction-metadata-and-entity-contracts-v1.md`
- Forge ticket: `ef9c9854-e3ed-4f86-a9e6-2bd9439456b4` (#21)
- AAR: `bcac7916-003c-4286-8c1f-eee6ee61c77b`

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS (design + test plan in notes) |
| 3 — Implement | PASS (compiles; fmt+clippy green first-run; core 130 + content 16 pass; no fixture breakage) |
| 3.5 — Inspect | PASS (4 critics; clippy green; 0 code defects; as_str coverage → Phase 4) |
| 4 — Validate | PASS (+9 tests; gate --fast 14/14; core 137/content 18; node 32; cov 99.5%) |
| 5 — Complete | PASS (Decision 039; AAR+ADR+rule captured; ticket #21 done; archived) |
