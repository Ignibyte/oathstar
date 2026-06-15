use axum_extra::extract::cookie::{Cookie, SameSite};

/// The name of the studio session cookie.
pub const SESSION_COOKIE: &str = "oathstar_studio_session";

/// Build the session cookie carrying `id`.
///
/// `HttpOnly`, `SameSite=Strict`, `Path=/`. The `Secure` flag is intentionally
/// OFF for loopback-http dev (a browser drops a `Secure` cookie over http); a TLS
/// deployment turns it on. `SameSite=Strict` is also the CSRF defense — a
/// cross-site POST never carries this cookie.
#[must_use]
pub fn session_cookie(id: String) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, id))
        .http_only(true)
        .same_site(SameSite::Strict)
        .path("/")
        .build()
}

/// Build the cookie that clears the studio session at logout — same name and
/// path so the browser drops the live one.
#[must_use]
pub fn clearing_cookie() -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, String::new()))
        .http_only(true)
        .same_site(SameSite::Strict)
        .path("/")
        .build()
}

/// Constant-time comparison of a submitted secret against the configured one.
///
/// A wrong guess leaks no byte-by-byte timing. Returns `false` when no secret is
/// configured (no owner can log in) — reuses #41's set-but-empty ≠ unset posture.
#[must_use]
pub fn verify_secret(provided: &str, configured: Option<&str>) -> bool {
    let Some(configured) = configured else {
        return false;
    };
    let provided = provided.as_bytes();
    let configured = configured.as_bytes();
    // Fold the length mismatch into the accumulator (so a differing length can
    // never green-light), then XOR the overlapping bytes.
    let mut diff = u8::from(provided.len() != configured.len());
    for (left, right) in provided.iter().zip(configured.iter()) {
        diff |= left ^ right;
    }
    diff == 0
}

/// Read the configured owner secret from `OATHSTAR_OWNER_PASSWORD`.
///
/// A blank or whitespace-only value is treated as absent (no owner can log in) —
/// an empty env var is not "set"
/// (`PR-claude-blank-env-config-is-not-absent-001`).
#[must_use]
pub fn owner_secret_from_env() -> Option<String> {
    std::env::var("OATHSTAR_OWNER_PASSWORD")
        .ok()
        .filter(|secret| !secret.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::{
        clearing_cookie, owner_secret_from_env, session_cookie, verify_secret, SESSION_COOKIE,
    };
    use axum_extra::extract::cookie::SameSite;

    #[test]
    fn session_cookie_is_hardened() {
        // T2: the live session cookie carries the id with the safe flags.
        let cookie = session_cookie("abc123".to_owned());
        assert_eq!(cookie.name(), SESSION_COOKIE);
        assert_eq!(cookie.value(), "abc123");
        assert_eq!(cookie.http_only(), Some(true));
        assert_eq!(cookie.same_site(), Some(SameSite::Strict));
        assert_eq!(cookie.path(), Some("/"));
    }

    #[test]
    fn clearing_cookie_empties_the_value() {
        // T2: logout's cookie matches name+path and carries an empty value.
        let cookie = clearing_cookie();
        assert_eq!(cookie.name(), SESSION_COOKIE);
        assert_eq!(cookie.value(), "");
        assert_eq!(cookie.path(), Some("/"));
        assert_eq!(cookie.same_site(), Some(SameSite::Strict));
    }

    #[test]
    fn verify_secret_matches_only_the_exact_secret() {
        // T3/T9: constant-time compare — true only for the exact secret, and
        // never when none is configured.
        assert!(verify_secret("hunter2", Some("hunter2")));
        assert!(!verify_secret("hunter3", Some("hunter2"))); // same length, wrong
        assert!(!verify_secret("hunter", Some("hunter2"))); // shorter
        assert!(!verify_secret("hunter2x", Some("hunter2"))); // longer
        assert!(!verify_secret("", Some("hunter2"))); // empty guess
        assert!(!verify_secret("anything", None)); // unconfigured → nobody in
        assert!(!verify_secret("", None));
    }

    #[test]
    fn owner_secret_from_env_treats_blank_as_unset() {
        // T4 / PR-blank-env. Sole test touching OATHSTAR_OWNER_PASSWORD in this
        // binary → race-free.
        std::env::set_var("OATHSTAR_OWNER_PASSWORD", "s3cr3t");
        let set = owner_secret_from_env();
        std::env::set_var("OATHSTAR_OWNER_PASSWORD", "   ");
        let blank = owner_secret_from_env();
        std::env::remove_var("OATHSTAR_OWNER_PASSWORD");
        let unset = owner_secret_from_env();

        assert_eq!(set.as_deref(), Some("s3cr3t"));
        assert_eq!(blank, None);
        assert_eq!(unset, None);
    }
}
