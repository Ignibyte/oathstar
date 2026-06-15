//! Shared authentication, session, and role primitives for Oathstar's binaries
//! (ticket #42; Decisions 056/057).
//!
//! Both the game server and the studio sidecar depend on this crate, so there is
//! ONE auth/session model: a [`SessionStore`] maps an opaque key (a bearer token
//! or a session-cookie id) to a [`Principal`]; a generic [`AuthPrincipal`]
//! extractor enforces it for API handlers; and the cookie/login helpers back the
//! studio's browser login. Real accounts, password hashing, OAuth, and
//! persistence are out of scope (deferred to the first hosted deployment).

mod cookie;
mod error;
mod extract;
mod session;

pub use cookie::{
    clearing_cookie, owner_secret_from_env, session_cookie, verify_secret, SESSION_COOKIE,
};
pub use error::AuthError;
pub use extract::{authenticate, principal_from_cookie, require_role, AuthPrincipal};
pub use session::{owner_principal, SessionStore};

// Re-export the wire identity types so consumers need only depend on this crate.
pub use oathstar_protocol::{AuthRole, Principal};
