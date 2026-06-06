# Planning Workspace

This folder holds work planning, not permanent game design prose. Permanent
design docs stay directly under `docs/`.

## Directory Map

- `intake/` - pre-ticket work ideas. These are rough candidates that may become
  forge tickets later.
- `tickets/open/` - forge-backed tickets that are real backlog items but not
  necessarily the active implementation pipeline.
- `tickets/closed/` - local ticket documents after their forge ticket closes.
- `_templates/` - reusable planning templates, including intake and EARS
  requirements.
- `pipeline/active/` - the single active forge-backed work item. There must
  never be more than one active pipeline spec here.
- `pipeline/completed/` - archived pipeline specs and notes after work closes.
- `pipeline/_templates/` - the spec/notes templates used when a forge ticket is
  minted.

## Ticket Document Rule

Every forge ticket created for Oathstar work must have a local ticket document:

- `tickets/open/TICKET-<number>-<slug>.md`

The ticket document frontmatter `ticket:` field is the canonical link from local
docs to the forge ticket. Open ticket docs live in `tickets/open/`; closed ticket
docs move to `tickets/closed/`.

When a ticket becomes active implementation work, `/pipeline:plan` also creates
the active pipeline document pair:

- `<slug>.spec.md`
- `<slug>.notes.md`

The pipeline spec links back to the ticket document and the forge ticket.

Intake docs are different: they capture possible work before a ticket exists.
When an intake item is promoted, `/pipeline:plan` creates the forge ticket,
creates or links the local ticket document, creates the spec/notes pair, and
links the intake doc to the ticket/spec.

## Requirements Style

Pipeline acceptance criteria use EARS. Each requirement should be concrete,
testable, and written as one behavior per row:

- Ubiquitous: `The <system> shall <response>.`
- Event-driven: `When <trigger>, the <system> shall <response>.`
- State-driven: `While <state>, the <system> shall <response>.`
- Unwanted behavior: `If <condition>, then the <system> shall <response>.`
- Optional/contextual: `Where <context>, the <system> shall <response>.`

Use `docs/planning/_templates/ears-requirements.md` when drafting new work.
