//! The v1 command parser: turns raw MUD-style input into a typed [`Command`].
//!
//! Parsing is a pure, deterministic function ([`parse`]) kept separate from the
//! engine's effectful command handling, so every front-end (UI buttons, Datastar
//! actions, future DM/LLM actions) can share one typed engine path and the
//! grammar stays exhaustively unit-testable. The grammar is the *forgiving
//! symbolic* parser locked in `docs/decisions.md` Decision 002 — not natural
//! language.

/// A compass or vertical movement direction.
///
/// The six canonical directions a room exit can use. Parsing accepts the full
/// word or its single-letter alias (case-insensitive); [`Direction::as_str`]
/// returns the canonical lowercase form used as a room-exit key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    North,
    South,
    East,
    West,
    Up,
    Down,
}

impl Direction {
    /// Map a single lowercase token to a [`Direction`], accepting the full word
    /// or its first-letter alias. Returns `None` for anything else.
    fn from_token(token: &str) -> Option<Self> {
        match token {
            "north" | "n" => Some(Self::North),
            "south" | "s" => Some(Self::South),
            "east" | "e" => Some(Self::East),
            "west" | "w" => Some(Self::West),
            "up" | "u" => Some(Self::Up),
            "down" | "d" => Some(Self::Down),
            _ => None,
        }
    }

    /// The canonical lowercase name, matching the keys used in a room's exits map.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::North => "north",
            Self::South => "south",
            Self::East => "east",
            Self::West => "west",
            Self::Up => "up",
            Self::Down => "down",
        }
    }
}

/// A parsed player command — the typed boundary between raw input and the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Blank input (empty or all-whitespace); the engine prompts and does nothing.
    Empty,
    /// `help` / `h` — list the available commands.
    Help,
    /// `look` / `l` / `examine` / `x`, optionally with a target.
    ///
    /// `target` is `None` for a bare look (describe the room) and `Some(text)`
    /// for `look <target>`, with the target's meaningful text preserved — only
    /// surrounding and repeated whitespace is collapsed; case is kept.
    Look { target: Option<String> },
    /// A movement command produced by a direction alias or `go <dir>`.
    Move(Direction),
    /// `swear` / `vow` — swear the module's offered oath.
    Swear,
    /// `confront` / `challenge` — resolve the boss at the current room's endpoint.
    Confront,
    /// `attack` / `strike` / `fight`, optionally with a target (ticket #22). A bare
    /// verb engages the active foe (or the only hostile present); `attack <name>`
    /// names the hostile to engage. Starts combat in a combat-enabled room and
    /// advances it once underway. Target text is preserved like a `Look` target.
    Attack { target: Option<String> },
    /// `flee` — break away from the active encounter (ticket #24). A bare,
    /// strict-arity verb like `swear`: it queues the between-pulse flee action,
    /// which the next combat pulse's skill window resolves into a fled outcome.
    Flee,
    /// `guard` — brace for the next enemy blow (ticket #25). A bare,
    /// strict-arity battle verb queued for the next pulse's skill window;
    /// outside combat it refuses cleanly.
    Guard,
    /// `power strike` — the two-token heavy blow (ticket #25), queued for the
    /// next pulse's skill window like `guard`. Strict arity: exactly the two
    /// tokens — `power` alone, trailing tokens, and the fused `powerstrike`
    /// are unknown commands.
    PowerStrike,
    /// `talk` / `speak` with a target — address a nearby actor. The target text is
    /// preserved (case kept, surrounding/repeated whitespace collapsed) exactly as
    /// a `Look` target; a bare `talk` with no target is [`Unknown`](Self::Unknown).
    Talk { target: String },
    /// `take` / `get` / `pick up` with a target — pick up a nearby world item. The
    /// target text is preserved like a `Look` target; a bare verb with no target
    /// is [`Unknown`](Self::Unknown).
    Take { target: String },
    /// `drop` with a target — place a carried item into the current room/cell
    /// (ticket #20). Target text is preserved like a `Take` target; a bare `drop`
    /// with no target is [`Unknown`](Self::Unknown).
    Drop { target: String },
    /// `inventory` / `pack` / `i` — list carried items (ticket #20). Bare verb,
    /// strict arity; trailing tokens make it [`Unknown`](Self::Unknown).
    Inventory,
    /// Input that matched no known command; carries the collapsed echo of the raw
    /// input for a helpful failure message. The engine mutates no state for it.
    Unknown { input: String },
}

/// Parse raw player input into a typed [`Command`].
///
/// Pure and deterministic. Surrounding and repeated whitespace is collapsed and
/// the *verb* is matched case-insensitively (Decision 002); a `look`/`examine`
/// target keeps its original case. Unrecognized input becomes
/// [`Command::Unknown`] rather than an error — the parser never fails.
#[must_use]
pub fn parse(input: &str) -> Command {
    let mut tokens = input.split_whitespace();
    let Some(first) = tokens.next() else {
        return Command::Empty;
    };
    let verb = first.to_lowercase();
    let rest: Vec<&str> = tokens.collect();

    if let Some(direction) = Direction::from_token(&verb) {
        // A bare direction takes no trailing tokens — the same strict arity as the
        // `go <dir>` form below: `north now` / `n guard` are an unknown command,
        // not a silent move that could mutate state on malformed input.
        if rest.is_empty() {
            return Command::Move(direction);
        }
        return Command::Unknown {
            input: collapse(input),
        };
    }

    if verb == "go" {
        // `go` takes exactly one direction token (`go east`). Missing or trailing
        // tokens (`go`, `go banana`, `go east now`) are an unknown command, not a
        // silent move — the grammar stays precise rather than ignoring extra input.
        if let [token] = rest.as_slice() {
            if let Some(direction) = Direction::from_token(&token.to_lowercase()) {
                return Command::Move(direction);
            }
        }
        return Command::Unknown {
            input: collapse(input),
        };
    }

    if verb == "help" || verb == "h" {
        return Command::Help;
    }

    if matches!(verb.as_str(), "look" | "l" | "examine" | "x") {
        let target = if rest.is_empty() {
            None
        } else {
            Some(rest.join(" "))
        };
        return Command::Look { target };
    }

    if let Some(command) = parse_bare_verb(&verb, &rest, input) {
        return command;
    }

    if let Some(command) = parse_combat_verb(&verb, &rest) {
        return command;
    }

    // `talk`/`speak` and `take`/`get` take a required target — a bare verb with no
    // target is an unknown command (strict arity), so the typed command always
    // carries non-empty target text. The target keeps its case (only the verb is
    // case-folded), matching `look <target>`.
    if matches!(verb.as_str(), "talk" | "speak") {
        if rest.is_empty() {
            return Command::Unknown {
                input: collapse(input),
            };
        }
        return Command::Talk {
            target: rest.join(" "),
        };
    }

    if matches!(verb.as_str(), "take" | "get") {
        if rest.is_empty() {
            return Command::Unknown {
                input: collapse(input),
            };
        }
        return Command::Take {
            target: rest.join(" "),
        };
    }

    if verb == "drop" {
        // Required target (strict arity), mirroring `take` — a bare `drop` is an
        // unknown command, not a no-op (ticket #20).
        if rest.is_empty() {
            return Command::Unknown {
                input: collapse(input),
            };
        }
        return Command::Drop {
            target: rest.join(" "),
        };
    }

    // The two-token battle verb `power strike` (ticket #25): only the exact
    // pair queues the heavy blow. `power` alone, `power <other>`, trailing
    // tokens, and the fused `powerstrike` are all unknown — the grammar stays
    // precise rather than guessing (the `pick up` pattern).
    if verb == "power" {
        if let [strike] = rest.as_slice() {
            if strike.eq_ignore_ascii_case("strike") {
                return Command::PowerStrike;
            }
        }
        return Command::Unknown {
            input: collapse(input),
        };
    }

    // The two-token verb `pick up <target>`: only `pick up` is a take. `pick`
    // alone, `pick <non-up>`, `pick up` with no target, and the one-word `pickup`
    // are all unknown — the grammar stays precise rather than guessing.
    if verb == "pick" {
        if let [up, target @ ..] = rest.as_slice() {
            if up.eq_ignore_ascii_case("up") && !target.is_empty() {
                return Command::Take {
                    target: target.join(" "),
                };
            }
        }
        return Command::Unknown {
            input: collapse(input),
        };
    }

    Command::Unknown {
        input: collapse(input),
    }
}

/// Parse the bare, strict-arity verbs (`swear`/`vow`, `confront`/`challenge`,
/// `flee`, `guard`, `inventory`/`pack`/`i`) — each takes no trailing tokens. Returns
/// `Some(command)` when `verb` is one of them (a trailing token yields
/// `Some(Unknown)`); `None` when `verb` is not a bare verb so `parse` keeps
/// trying. Grouping these keeps `parse` under the clippy line ceiling (#20).
fn parse_bare_verb(verb: &str, rest: &[&str], input: &str) -> Option<Command> {
    let command = match verb {
        "swear" | "vow" => Command::Swear,
        "confront" | "challenge" => Command::Confront,
        "flee" => Command::Flee,
        "guard" => Command::Guard,
        "inventory" | "pack" | "i" => Command::Inventory,
        _ => return None,
    };
    if rest.is_empty() {
        Some(command)
    } else {
        Some(Command::Unknown {
            input: collapse(input),
        })
    }
}

/// Parse the optional-target combat verbs (`attack`/`strike`/`fight`, ticket
/// #22). A bare verb yields `Attack{None}` (engage the active/only foe); trailing
/// tokens become the target (`Attack{Some(target)}`), preserved like a `look`
/// target. Returns `None` when `verb` is not a combat verb, so `parse` keeps
/// trying. Split out to keep `parse` under the clippy line ceiling.
fn parse_combat_verb(verb: &str, rest: &[&str]) -> Option<Command> {
    if !matches!(verb, "attack" | "strike" | "fight") {
        return None;
    }
    let target = if rest.is_empty() {
        None
    } else {
        Some(rest.join(" "))
    };
    Some(Command::Attack { target })
}

/// Trim and collapse internal whitespace runs to single spaces, preserving case.
fn collapse(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::{parse, Command, Direction};

    // P5: empty / all-whitespace input → Empty.
    #[test]
    fn empty_or_whitespace_input_parses_to_empty() {
        assert_eq!(parse(""), Command::Empty);
        assert_eq!(parse("   "), Command::Empty);
        assert_eq!(parse("\t \n"), Command::Empty);
    }

    // P1: every direction word + single-letter alias → Move(that direction).
    // One distinct direction per token, so deleting any `from_token` arm fails here.
    #[test]
    fn every_direction_alias_parses_to_move() {
        let cases = [
            ("north", Direction::North),
            ("n", Direction::North),
            ("south", Direction::South),
            ("s", Direction::South),
            ("east", Direction::East),
            ("e", Direction::East),
            ("west", Direction::West),
            ("w", Direction::West),
            ("up", Direction::Up),
            ("u", Direction::Up),
            ("down", Direction::Down),
            ("d", Direction::Down),
        ];
        for (token, expected) in cases {
            assert_eq!(parse(token), Command::Move(expected), "token: {token}");
        }
    }

    // P1/P3: the verb is matched case-insensitively.
    #[test]
    fn direction_verbs_are_case_insensitive() {
        assert_eq!(parse("N"), Command::Move(Direction::North));
        assert_eq!(parse("NORTH"), Command::Move(Direction::North));
        assert_eq!(parse("Up"), Command::Move(Direction::Up));
    }

    // P2: `go <dir>` with exactly one direction token → Move (case-insensitive).
    #[test]
    fn go_with_one_direction_moves() {
        assert_eq!(parse("go east"), Command::Move(Direction::East));
        assert_eq!(parse("go N"), Command::Move(Direction::North));
        assert_eq!(parse("GO down"), Command::Move(Direction::Down));
    }

    // P2 (inspect carry-forward): `go` alone / non-direction / trailing tokens
    // are unknown — the grammar is precise, not "ignore extra input".
    #[test]
    fn go_without_exactly_one_direction_is_unknown() {
        assert_eq!(
            parse("go"),
            Command::Unknown {
                input: "go".to_string()
            }
        );
        assert_eq!(
            parse("go banana"),
            Command::Unknown {
                input: "go banana".to_string()
            }
        );
        assert_eq!(
            parse("go east now"),
            Command::Unknown {
                input: "go east now".to_string()
            }
        );
    }

    // Review fix: a BARE direction with trailing tokens is unknown, not a silent
    // move — same strict arity as `go <dir>` (prevents state mutation on
    // malformed input like `north now` / `n guard`).
    #[test]
    fn bare_direction_with_trailing_tokens_is_unknown() {
        assert_eq!(
            parse("north now"),
            Command::Unknown {
                input: "north now".to_string()
            }
        );
        assert_eq!(
            parse("n guard"),
            Command::Unknown {
                input: "n guard".to_string()
            }
        );
    }

    // P2: each look-verb alias with a target → Look{Some(target)}, text preserved.
    #[test]
    fn look_verbs_with_target_preserve_text() {
        assert_eq!(
            parse("look warden"),
            Command::Look {
                target: Some("warden".to_string())
            }
        );
        assert_eq!(
            parse("examine the gate"),
            Command::Look {
                target: Some("the gate".to_string())
            }
        );
        assert_eq!(
            parse("x lantern"),
            Command::Look {
                target: Some("lantern".to_string())
            }
        );
    }

    // P3: bare look-verb aliases → Look{None}.
    #[test]
    fn bare_look_verbs_have_no_target() {
        for verb in ["look", "l", "examine", "x"] {
            assert_eq!(parse(verb), Command::Look { target: None }, "verb: {verb}");
        }
    }

    // P4 (REQ-003): verb is case-folded, target keeps its case, whitespace collapses.
    #[test]
    fn normalize_casefolds_verb_only_and_collapses_whitespace() {
        assert_eq!(
            parse("  LOOK   Warden "),
            Command::Look {
                target: Some("Warden".to_string())
            }
        );
        assert_eq!(
            parse("ExAmInE  Black  Lantern"),
            Command::Look {
                target: Some("Black Lantern".to_string())
            }
        );
    }

    // P6: help aliases (case-insensitive) → Help. Both `help` and `h` so the
    // `||` / second-arm mutants die.
    #[test]
    fn help_aliases_parse_to_help() {
        assert_eq!(parse("help"), Command::Help);
        assert_eq!(parse("h"), Command::Help);
        assert_eq!(parse("HELP"), Command::Help);
    }

    // P5: unrecognized input → Unknown with a collapsed, case-preserving echo.
    #[test]
    fn unknown_input_is_unknown_with_collapsed_echo() {
        assert_eq!(
            parse("xyzzy"),
            Command::Unknown {
                input: "xyzzy".to_string()
            }
        );
        assert_eq!(
            parse("  Foo   bar "),
            Command::Unknown {
                input: "Foo bar".to_string()
            }
        );
    }

    // P7: as_str returns the canonical lowercase exit-key for every variant
    // (concrete literals — kills whole-body constant-replacement mutants).
    #[test]
    fn direction_as_str_is_canonical_lowercase() {
        assert_eq!(Direction::North.as_str(), "north");
        assert_eq!(Direction::South.as_str(), "south");
        assert_eq!(Direction::East.as_str(), "east");
        assert_eq!(Direction::West.as_str(), "west");
        assert_eq!(Direction::Up.as_str(), "up");
        assert_eq!(Direction::Down.as_str(), "down");
    }

    // ---- ticket #7: swear / confront verbs ----

    // Both swear aliases (case-insensitive) → Swear; kills the `||`/dropped-alias.
    #[test]
    fn swear_and_vow_parse_to_swear() {
        assert_eq!(parse("swear"), Command::Swear);
        assert_eq!(parse("vow"), Command::Swear);
        assert_eq!(parse("SWEAR"), Command::Swear);
    }

    // Strict arity: trailing tokens → Unknown with the exact collapsed echo.
    #[test]
    fn swear_with_trailing_tokens_is_unknown() {
        assert_eq!(
            parse("swear oath"),
            Command::Unknown {
                input: "swear oath".to_string()
            }
        );
    }

    // Both confront aliases (case-insensitive) → Confront.
    #[test]
    fn confront_and_challenge_parse_to_confront() {
        assert_eq!(parse("confront"), Command::Confront);
        assert_eq!(parse("challenge"), Command::Confront);
        assert_eq!(parse("CONFRONT"), Command::Confront);
    }

    // Strict arity: trailing tokens → Unknown with the exact collapsed echo.
    #[test]
    fn confront_with_trailing_tokens_is_unknown() {
        assert_eq!(
            parse("confront now"),
            Command::Unknown {
                input: "confront now".to_string()
            }
        );
    }

    // ---- ticket #18: talk / take parsing ----

    // REQ-001: talk/speak + target → Talk{target}; verb case-folded, target case kept.
    #[test]
    fn talk_and_speak_with_target_parse_to_talk() {
        assert_eq!(
            parse("talk mara"),
            Command::Talk {
                target: "mara".to_string()
            }
        );
        assert_eq!(
            parse("speak warden"),
            Command::Talk {
                target: "warden".to_string()
            }
        );
        // Verb folded; target case preserved.
        assert_eq!(
            parse("TALK Mara"),
            Command::Talk {
                target: "Mara".to_string()
            }
        );
    }

    // REQ-002: take/get and the two-token `pick up`, each + target → Take{target}.
    #[test]
    fn take_get_and_pick_up_with_target_parse_to_take() {
        assert_eq!(
            parse("take coin"),
            Command::Take {
                target: "coin".to_string()
            }
        );
        assert_eq!(
            parse("get coin"),
            Command::Take {
                target: "coin".to_string()
            }
        );
        assert_eq!(
            parse("pick up black candle"),
            Command::Take {
                target: "black candle".to_string()
            }
        );
        assert_eq!(
            parse("GET Coin"),
            Command::Take {
                target: "Coin".to_string()
            }
        );
    }

    // REQ-001/002 boundary: bare verbs (no target) are Unknown (strict arity), so a
    // typed Talk/Take always carries non-empty target text.
    #[test]
    fn bare_talk_and_take_verbs_are_unknown() {
        for verb in ["talk", "speak", "take", "get"] {
            assert_eq!(
                parse(verb),
                Command::Unknown {
                    input: verb.to_string()
                },
                "bare verb: {verb}"
            );
        }
    }

    // REQ-002 boundary: only the two-token `pick up <target>` is a take. `pick`
    // alone, `pick up` with no target, `pick <non-up>`, and one-word `pickup` are
    // all Unknown — the grammar guesses nothing.
    #[test]
    fn pick_arity_is_strict() {
        assert_eq!(
            parse("pick up candle"),
            Command::Take {
                target: "candle".to_string()
            }
        );
        for input in ["pick", "pick up", "pick something", "pickup candle"] {
            assert_eq!(
                parse(input),
                Command::Unknown {
                    input: input.to_string()
                },
                "input: {input}"
            );
        }
    }

    // REQ-001/002: targets preserve case and collapse internal whitespace like a
    // look target; the `up` sentinel matches case-insensitively but the target
    // tokens keep their case (even when the target is literally "UP").
    #[test]
    fn talk_take_targets_preserve_case_and_collapse_whitespace() {
        assert_eq!(
            parse("talk  Mara   Candlekeep"),
            Command::Talk {
                target: "Mara Candlekeep".to_string()
            }
        );
        assert_eq!(
            parse("pick   up   Black  Candle"),
            Command::Take {
                target: "Black Candle".to_string()
            }
        );
        assert_eq!(
            parse("pick up UP"),
            Command::Take {
                target: "UP".to_string()
            }
        );
    }

    // T4 (REQ-003): the inventory aliases parse to the bare `Inventory` command;
    // trailing tokens are `Unknown` (strict arity, via `parse_bare_verb`).
    #[test]
    fn inventory_aliases_parse_to_inventory() {
        assert_eq!(parse("inventory"), Command::Inventory);
        assert_eq!(parse("pack"), Command::Inventory);
        assert_eq!(parse("i"), Command::Inventory);
        assert_eq!(
            parse("inventory all"),
            Command::Unknown {
                input: "inventory all".to_string()
            }
        );
    }

    // T7 (REQ-005): `drop` takes a required target; a bare `drop` is `Unknown`.
    #[test]
    fn drop_with_target_parses_to_drop() {
        assert_eq!(
            parse("drop wax stub"),
            Command::Drop {
                target: "wax stub".to_string()
            }
        );
        assert_eq!(
            parse("drop"),
            Command::Unknown {
                input: "drop".to_string()
            }
        );
    }

    // C13 (REQ-002): the three combat verbs take an OPTIONAL target. A bare verb is
    // `Attack{None}` (engage the active/only foe); trailing tokens become the target
    // (preserved like a `look` target). Each alias maps so the `matches!` arm's
    // dropped-alias mutants die.
    #[test]
    fn attack_verbs_parse_with_optional_target() {
        for verb in ["attack", "strike", "fight"] {
            assert_eq!(
                parse(verb),
                Command::Attack { target: None },
                "bare verb: {verb}"
            );
        }
        assert_eq!(
            parse("attack stray"),
            Command::Attack {
                target: Some("stray".to_string())
            }
        );
        // Verb case-folded; multi-word target keeps its case + collapses whitespace.
        assert_eq!(
            parse("FIGHT  Ashen  Stray"),
            Command::Attack {
                target: Some("Ashen Stray".to_string())
            }
        );
        // A non-combat verb that merely starts with these letters is NOT an attack.
        assert_eq!(
            parse("attacker"),
            Command::Unknown {
                input: "attacker".to_string()
            }
        );
    }

    // T14 (ticket #24, REQ-004): `flee` is a bare, strict-arity verb like `swear`.
    #[test]
    fn flee_parses_as_bare_strict_arity_verb() {
        assert_eq!(parse("flee"), Command::Flee);
        assert_eq!(parse("FLEE"), Command::Flee, "the verb is case-folded");
        assert_eq!(
            parse("flee now"),
            Command::Unknown {
                input: "flee now".to_string()
            },
            "trailing tokens refuse — strict arity"
        );
    }

    // V15 (ticket #25, REQ-001/004): `guard` is a bare strict-arity battle verb.
    #[test]
    fn guard_parses_as_bare_strict_arity_verb() {
        assert_eq!(parse("guard"), Command::Guard);
        assert_eq!(parse("GUARD"), Command::Guard, "the verb is case-folded");
        assert_eq!(
            parse("guard now"),
            Command::Unknown {
                input: "guard now".to_string()
            },
            "trailing tokens refuse — strict arity"
        );
    }

    // V15 (ticket #25): `power strike` is exactly the two tokens — partial,
    // fused, and trailing forms stay unknown, and bare `strike` is still the
    // #22 attack verb.
    #[test]
    fn power_strike_parses_as_exactly_two_tokens() {
        assert_eq!(parse("power strike"), Command::PowerStrike);
        assert_eq!(parse("POWER Strike"), Command::PowerStrike, "case-folded");
        assert_eq!(
            parse("power   strike"),
            Command::PowerStrike,
            "whitespace runs collapse"
        );
        for bad in ["power", "powerstrike", "power slam", "power strike now"] {
            assert!(
                matches!(parse(bad), Command::Unknown { .. }),
                "{bad} stays unknown"
            );
        }
        assert_eq!(
            parse("strike"),
            Command::Attack { target: None },
            "bare strike is still the #22 attack verb"
        );
    }
}
