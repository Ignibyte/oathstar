# EARS Requirements Template

Use this template for pipeline acceptance criteria and intake candidates.

## Rules

- Use `shall`.
- Write one behavior per requirement.
- Make the result observable.
- Avoid vague verbs like "support", "improve", "handle", or "work" unless the
  observable behavior is also stated.
- Include a verification method for each requirement.

## Patterns

| Pattern | Form |
|---|---|
| Ubiquitous | The `<system>` shall `<response>`. |
| Event-driven | When `<trigger>`, the `<system>` shall `<response>`. |
| State-driven | While `<state>`, the `<system>` shall `<response>`. |
| Unwanted behavior | If `<condition>`, then the `<system>` shall `<response>`. |
| Optional/contextual | Where `<context>`, the `<system>` shall `<response>`. |

## Requirement Table

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When `<trigger>`, the `<system>` shall `<observable response>`. | `<test, smoke check, doc check, or review check>` |
| REQ-002 | The `<system>` shall `<observable response>`. | `<test, smoke check, doc check, or review check>` |

## Notes

- Prefer several small requirements over one compound requirement.
- Requirements describe what must be true, not how code should be structured.
- Design and implementation notes belong in the paired pipeline notes file.
