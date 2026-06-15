use core::fmt::Write as _;
use std::{
    collections::HashMap,
    sync::{Arc, PoisonError, RwLock},
};

use oathstar_protocol::{AuthRole, Principal};

/// A registry mapping an opaque key — a bearer token (game server) or a session
/// cookie id (studio) — to its authenticated [`Principal`].
///
/// Cheap to clone (an `Arc` inside) and safe to share across handlers. Sessions
/// are created at login and removed at logout; the dev-owner bearer seed (#41)
/// is inserted at construction. v1 keeps sessions in memory (lost on restart).
#[derive(Clone, Default)]
pub struct SessionStore {
    sessions: Arc<RwLock<HashMap<String, Principal>>>,
}

impl SessionStore {
    /// An empty store — no key resolves until a session is seeded or created.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed a store with a single deterministic owner reachable via `owner_token`
    /// (the local-dev bearer bypass, #41). A blank/whitespace-only token is
    /// treated as absent and seeds nothing — an empty env var is not "set"
    /// (`PR-claude-blank-env-config-is-not-absent-001`).
    #[must_use]
    pub fn from_owner_token(owner_token: Option<String>) -> Self {
        let store = Self::new();
        if let Some(token) = owner_token.filter(|candidate| !candidate.trim().is_empty()) {
            store.insert(token, owner_principal());
        }
        store
    }

    /// Build a store seeded from `OATHSTAR_DEV_OWNER` — the game server's
    /// deterministic local-dev bearer bypass.
    ///
    /// Unset or blank yields an empty store (the production default).
    #[must_use]
    pub fn from_env() -> Self {
        Self::from_owner_token(std::env::var("OATHSTAR_DEV_OWNER").ok())
    }

    /// Resolve a key (bearer token or session id) to its principal, if known.
    #[must_use]
    pub fn resolve(&self, key: &str) -> Option<Principal> {
        self.sessions
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(key)
            .cloned()
    }

    /// Create a fresh session for `principal`, returning its opaque id — a
    /// 256-bit OS-CSPRNG value, hex-encoded. Used by login.
    pub fn create_session(&self, principal: Principal) -> String {
        let id = new_session_id();
        self.insert(id.clone(), principal);
        id
    }

    /// Remove a session by id (logout). Idempotent — an unknown id is a no-op.
    pub fn remove_session(&self, id: &str) {
        self.sessions
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(id);
    }

    fn insert(&self, key: String, principal: Principal) {
        self.sessions
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key, principal);
    }
}

/// The deterministic owner principal — full server authority (Decision 056).
/// Both the dev-owner bearer seed and a successful studio login resolve to this.
#[must_use]
pub fn owner_principal() -> Principal {
    Principal {
        id: "owner".to_owned(),
        name: "Owner".to_owned(),
        roles: vec![AuthRole::Owner],
    }
}

/// A fresh, unguessable session id: 32 OS-CSPRNG bytes, lowercase hex.
fn new_session_id() -> String {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).expect("the OS CSPRNG must be available");
    let mut id = String::new();
    for byte in bytes {
        let _ = write!(id, "{byte:02x}");
    }
    id
}

#[cfg(test)]
impl SessionStore {
    /// Test-only: seed an explicit `token` → `principal`, including edge keys
    /// (e.g. `""`) the public API never creates — used to prove the bearer
    /// blank-token guard fires *before* resolution.
    pub(crate) fn seeded(token: &str, principal: Principal) -> Self {
        let store = Self::new();
        store.insert(token.to_owned(), principal);
        store
    }
}

#[cfg(test)]
mod tests {
    use super::{owner_principal, SessionStore};
    use oathstar_protocol::AuthRole;

    #[test]
    fn create_resolve_remove_roundtrip() {
        // T1: a created session resolves, then is gone after removal.
        let store = SessionStore::new();
        let id = store.create_session(owner_principal());
        assert!(!id.is_empty());
        assert!(store
            .resolve(&id)
            .expect("a live session resolves")
            .grants(AuthRole::Owner));
        store.remove_session(&id);
        assert!(store.resolve(&id).is_none());
    }

    #[test]
    fn session_ids_are_unique_and_full_width() {
        // T1: distinct 256-bit ids (32 CSPRNG bytes → 64 hex chars).
        let store = SessionStore::new();
        let a = store.create_session(owner_principal());
        let b = store.create_session(owner_principal());
        assert_ne!(a, b, "each session id must be unique");
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn remove_session_is_idempotent() {
        // an unknown id is a harmless no-op.
        let store = SessionStore::new();
        store.remove_session("never-existed");
    }

    #[test]
    fn from_owner_token_seeds_owner_and_ignores_blank() {
        // T4 / #41 BF-auth-empty-env-owner-bypass: blank/None seeds nothing.
        let seeded = SessionStore::from_owner_token(Some("devtok".to_owned()));
        assert!(seeded
            .resolve("devtok")
            .expect("owner seeded")
            .grants(AuthRole::Owner));
        assert!(seeded.resolve("other").is_none());

        assert!(SessionStore::from_owner_token(None).resolve("").is_none());
        assert!(SessionStore::from_owner_token(Some(String::new()))
            .resolve("")
            .is_none());
        assert!(SessionStore::from_owner_token(Some("   ".to_owned()))
            .resolve("")
            .is_none());
    }

    #[test]
    fn from_env_reads_the_dev_owner_var() {
        // moved #41 + PR-blank-env. This is the *only* test touching
        // OATHSTAR_DEV_OWNER, so the set/remove is race-free within the binary.
        std::env::set_var("OATHSTAR_DEV_OWNER", "env-owner-tok");
        let seeded = SessionStore::from_env();
        std::env::set_var("OATHSTAR_DEV_OWNER", "   ");
        let blank = SessionStore::from_env();
        std::env::remove_var("OATHSTAR_DEV_OWNER");
        let unset = SessionStore::from_env();

        assert!(seeded
            .resolve("env-owner-tok")
            .expect("from_env seeds the dev owner")
            .grants(AuthRole::Owner));
        assert!(blank.resolve("").is_none());
        assert!(blank.resolve("   ").is_none());
        assert!(unset.resolve("env-owner-tok").is_none());
    }

    #[test]
    fn owner_principal_is_the_full_authority_owner() {
        let owner = owner_principal();
        assert_eq!(owner.id, "owner");
        assert_eq!(owner.name, "Owner");
        assert_eq!(owner.roles, vec![AuthRole::Owner]);
        assert!(owner.grants(AuthRole::Editor), "owner implies all roles");
    }
}
