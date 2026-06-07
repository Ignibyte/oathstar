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

    if matches!(verb.as_str(), "swear" | "vow") {
        // Bare verb only — the same strict arity as the movement verbs: `swear
        // oath` is an unknown command, not a silent swear on trailing input.
        if rest.is_empty() {
            return Command::Swear;
        }
        return Command::Unknown {
            input: collapse(input),
        };
    }

    if matches!(verb.as_str(), "confront" | "challenge") {
        if rest.is_empty() {
            return Command::Confront;
        }
        return Command::Unknown {
            input: collapse(input),
        };
    }

    Command::Unknown {
        input: collapse(input),
    }
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
}
