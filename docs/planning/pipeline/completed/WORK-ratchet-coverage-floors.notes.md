# WORK-ratchet-coverage-floors — Notes

## Phase 1 — Plan
- **Request:** ratchet coverage floors to lock in ticket #8's gains.
- **Intake source:** none.
- **Classification / tier:** work pipeline, single shippable slice (`chore`).
- **Forge recall:** own prevention rules surfaced (`PR-claude-coverage-exceptions-001`
  6b6d9158, `PR-claude-loader-testability-001` cc73c333) + ticket #8 failure/ADs.
  CONSTITUTION §0 sanctions raising floors. Bulletins: none.
- **Ticket:** forge #9 `e97fbc02-b958-4156-8a93-f78f434e801b`; claimed by
  `9758d99d-2237-4620-8f73-cfe0fe4d0ed9`; local doc
  `docs/planning/tickets/open/TICKET-9-ratchet-coverage-floors.md`.
- **AAR:** `f226f854-6974-4e4e-997e-02b0c3667ebf`.

### Before → after floors
| Floor | Before | After |
|---|---|---|
| `RUST_COV_FLOOR` | 60 | **94** |
| `JS_COV_FLOOR` | 70 | **75** |
| `MUT_MSI_FLOOR` | 100 | 100 (unchanged) |

Observed coverage (clears the new floors): Rust 94.34%, JS 75.19%, MSI 100%.

### Floor-mention edit map (discovery)
- `bin/gate.sh:57` — `RUST_COV_FLOOR=60; JS_COV_FLOOR=70; MUT_MSI_FLOOR=100` → `94; 75; 100`.
- `bin/gate.sh:38-39` — **stale** header comment (`…clamped back up to 25`; `RUST_COV_MIN=50 JS_COV_MIN=65 MUT_MSI_MIN=25`) → correct to 94/75/100.
- `bin/gate.sh:58-63, 216-264` — dynamic (use the variables); **no change**.
- `CLAUDE.md:57-58` — `RUST_COV_MIN=60, JS_COV_MIN=70, MUT_MSI_MIN=100` → 94/75/100.
- `CONSTITUTION.md:61` — `(RUST_COV_MIN=60, JS_COV_MIN=70, MUT_MSI_MIN=100)` → 94/75/100;
  `CONSTITUTION.md:64` — stale actual "~86% rust lines" → "~94% rust lines".
- `docs/review-harness.md:39-41` — bare "Rust coverage / JS coverage / mutation testing"
  → annotate with "(≥94% lines)/(≥75% lines)/(100% MSI)".
- CONSTITUTION gate table (39-41) uses `$RUST_COV_MIN` etc. — dynamic; no change.

## Phase 2 — Design

### Approach / architecture
- Pure config + docs change. The gate's floor mechanism already does the work:
  `MIN := env-override ?: FLOOR`, then a per-floor clamp that raises `MIN` back up
  to `FLOOR` if an env override is lower. So raising the **constants** is the whole
  behavioral change (AC1/AC2/AC4); the clamp (AC3) is already implemented and
  untouched. No Rust/JS code; no new unit tests (instruction 7).

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `bin/gate.sh` | line 57 `RUST_COV_FLOOR=60; JS_COV_FLOOR=70; MUT_MSI_FLOOR=100` → `94; 75; 100`; lines 38–39 fix the **stale** header comment (`clamped back up to 25` / `RUST_COV_MIN=50 JS_COV_MIN=65 MUT_MSI_MIN=25`) → 94/75/100 wording |
| 2 | `CONSTITUTION.md` | line 61 floor values `60/70/100` → `94/75/100`; line 64 stale actual `~86% rust lines` → `~94% rust lines` |
| 3 | `CLAUDE.md` | lines 57–58 floor values `60/70/100` → `94/75/100` |
| 4 | `docs/review-harness.md` | lines 39–41 annotate gate names → "Rust coverage (≥94% lines) / JS coverage (≥75% lines) / mutation testing (100% MSI)" |
| 5 | planning notes (this file) | before/after floors + gate result (filled at Validate) |

### Regression / verification plan (gate IS the test; one check per AC)
| # | Check | Proves |
|---|---|---|
| V1 | `grep` `bin/gate.sh` line 57 = `RUST_COV_FLOOR=94; JS_COV_FLOOR=75; MUT_MSI_FLOOR=100` | REQ-001/002/004 |
| V2 | FULL `bin/gate.sh`: gate:14 label "(>= 94% lines)" PASS; gate:15 "(>= 75% lines)" PASS; gate:16 "(MSI >= 100%)" PASS | REQ-001/002/004/006 |
| V3 | Clamp smoke: `RUST_COV_MIN=10 bin/gate.sh --fast 2>&1` prints `RUST_COV_MIN below the §0 minimum 94 — clamped`; same for `JS_COV_MIN=10` (floor 75) | REQ-003 |
| V4 | `grep` all four governing files read 94/75/100 consistently | REQ-005 |
| V5 | `bin/gate.sh` → `GATE GREEN [full]` | REQ-006 |
- Genuinely-uncoverable: none. No unit tests added (config-only; instruction 7).

### Risks / decisions
- **Thin headroom (intended):** 94.34% ≥ 94 (0.34 margin); 75.19% ≥ 75 (0.19 margin).
  This is the point of a ratchet — a future coverage dip now fails the gate and
  forces a test rather than silently regressing. Flag as a caveat in the report.
- **shellcheck (gate:9)** must stay clean — only numbers + comment prose change on
  the floor line; structure unchanged.
- **Stale-comment fix** is a bonus consistency win (the header disagreed with the
  code even before this ticket).
- **MSI floor stays 100** — explicitly unchanged.

## Phase 3 — Implement
- **Built (config + docs; no Rust/JS, no tests):**
  - `bin/gate.sh:57` — `RUST_COV_FLOOR=94; JS_COV_FLOOR=75; MUT_MSI_FLOOR=100`.
  - `bin/gate.sh:38-39` — stale header comment corrected to 94/75/100 (was 50/65/25).
  - `CONSTITUTION.md:61` — floor values → 94/75/100; `:64` — stale actual `~86%` → `~94%`.
  - `CLAUDE.md:57-58` — floor values → 94/75/100.
  - `docs/review-harness.md:39-41` — coverage gate names annotated with `(≥94%/≥75%/100% MSI)`.
- **In-phase checks (green):** grep cross-check — all four files read 94/75/100
  consistently; no stale 60/70/50/65/25 remain. `shellcheck -S info bin/gate.sh` CLEAN.
- **Deviations from design:** none. Bonus consistency fixes folded in (pre-existing
  stale gate.sh header comment + CONSTITUTION's stale "~86% rust" actual).
- Clamp logic untouched (already implements AC3); FULL gate + clamp smoke check at Validate.

## Inspect (Phase 3.5)
- **Lenses run** (2 parallel critics): gate-boundary correctness/clamp; doc-completeness/consistency.
- **Verdicts:** GATE-STAYS-GREEN; DOCS-CONSISTENT.
- **Findings:**
  | # | Severity | Finding | Verdict |
  |---|---|---|---|
  | 1 | (verified) | `cargo llvm-cov --fail-under-lines 94` PASSES at 94.3396% (full-precision, inclusive `>=`); ~0.34 pt headroom. | confirmed |
  | 2 | (verified) | JS `75.19 >= 75` PASSES; ~0.19 pt headroom. | confirmed |
  | 3 | (verified) | Clamp intact: `RUST_COV_MIN=10`/`JS_COV_MIN=10`/`MUT_MSI_MIN=0` each clamp up to 94/75/100; env above floor still raises. `shellcheck` clean; `bash -n` parses; line 57 exact. | confirmed (AC3) |
  | 4 | (verified) | All four governing files state 94/75/100 consistently (AC5); gate.sh header/clamp/const mutually consistent; historical ticket-#2/#8 notes correctly preserved; gate.sh:24-30 aic-mapping numbers (not floors) correctly untouched. | confirmed |
  | 5 | caveat (not a defect) | Thin margins by design (0.34 Rust / 0.19 JS) — a future small coverage dip will correctly fail the gate (the intended ratchet). | accepted; flagged in report |
- **No code/doc defects; no fixes required.** MSI floor unchanged at 100.

## Phase 4 — Validate
- **Tests added:** none (config + docs ticket; instruction 7 — no tests unless a change forces a failure; none did).
- **`cargo test --workspace`:** ok — 40 Rust tests, 0 failed.
- **`node --test tests/*.test.js`:** ok — 4 passed (also re-run inside gate:4).
- **`bin/gate.sh` (FULL):** **GREEN [full] — 16/16** at the new floors. Labels now read
  `gate:14 rust coverage (>= 94% lines)` PASS, `gate:15 js coverage (>= 75% lines)` PASS
  ("JS line coverage 75.19% >= 75%"), `gate:16 mutation (MSI >= 100%)` PASS (29/0).
- **Clamp smoke check (AC3):** `RUST_COV_MIN=5 JS_COV_MIN=5 MUT_MSI_MIN=5 bin/gate.sh --fast`
  printed:
  - `note: RUST_COV_MIN below the §0 minimum 94 — clamped.`
  - `note: JS_COV_MIN below the §0 minimum 75 — clamped.`
  - `note: MUT_MSI_MIN below the §0 minimum 100 — clamped.`
- **Before → after floors:** `RUST_COV_FLOOR` 60→**94**; `JS_COV_FLOOR` 70→**75**;
  `MUT_MSI_FLOOR` 100→100. Observed coverage clears them (Rust 94.34% ≥ 94, JS 75.19% ≥ 75, MSI 100).
- **Caveat (by design):** thin headroom — 0.34 pt (Rust), 0.19 pt (JS). The ratchet now
  fails the gate on a future coverage dip rather than letting it silently regress.
- **Pre-existing failures:** none.

## Phase 5 — Complete
- **Docs updated:** the floor docs (gate.sh header, CONSTITUTION.md, CLAUDE.md,
  docs/review-harness.md) were updated as the Implement step itself. No new design
  decision needed — CONSTITUTION §0 already states the ratchet policy; only the
  values changed.
- **Forge capture:** `aar-submit` f226f854 (completed, effectiveness 5, 1 novel);
  `prevention-rule-record` `PR-claude-ratchet-after-harden-001` (a05e526f). No failures.
- **Ticket closed:** forge #9 → `done`; completion comment a139a89d.
- **Archived:** pipeline pair → `docs/planning/pipeline/completed/`; local ticket doc
  → `docs/planning/tickets/closed/` (cross-links pre-fixed).
