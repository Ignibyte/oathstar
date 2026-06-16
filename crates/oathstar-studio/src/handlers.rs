use axum::{
    extract::{Form, State},
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::cookie::CookieJar;
use oathstar_auth::{
    clearing_cookie, owner_principal, principal_from_cookie, require_role, session_cookie,
    verify_secret, AuthRole, SESSION_COOKIE,
};
use serde::Deserialize;

use crate::{render, StudioState};

/// The login form body.
#[derive(Deserialize)]
pub struct LoginForm {
    secret: String,
}

/// `GET /login` — render the sign-in form.
pub async fn login_form() -> impl IntoResponse {
    render::login_page(None)
}

/// `POST /login` — verify the owner secret, start a session, redirect to the
/// dashboard; re-render with an error on a bad secret.
pub async fn login_submit(
    State(studio): State<StudioState>,
    jar: CookieJar,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    if verify_secret(&form.secret, studio.owner_secret.as_deref()) {
        let id = studio.sessions.create_session(owner_principal());
        (jar.add(session_cookie(id)), Redirect::to("/")).into_response()
    } else {
        render::login_page(Some("Invalid credentials.")).into_response()
    }
}

/// `POST /logout` — drop the session and clear the cookie.
pub async fn logout(State(studio): State<StudioState>, jar: CookieJar) -> impl IntoResponse {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        studio.sessions.remove_session(cookie.value());
    }
    (jar.remove(clearing_cookie()), Redirect::to("/login"))
}

/// `GET /` — the dashboard, behind the session; redirect to `/login` without a
/// valid owner/editor session.
pub async fn dashboard(State(studio): State<StudioState>, jar: CookieJar) -> impl IntoResponse {
    let Some(principal) = principal_from_cookie(&jar, &studio.sessions) else {
        return Redirect::to("/login").into_response();
    };
    if require_role(&principal, AuthRole::Editor).is_err() {
        return Redirect::to("/login").into_response();
    }
    render::dashboard_page(&principal).into_response()
}

#[cfg(test)]
mod tests {
    use super::{dashboard, login_form, login_submit, logout, LoginForm};
    use crate::StudioState;
    use axum::body::{to_bytes, Body};
    use axum::extract::{Form, FromRequestParts, State};
    use axum::http::{header, Request, StatusCode};
    use axum::response::IntoResponse;
    use axum_extra::extract::cookie::CookieJar;
    use oathstar_auth::{owner_principal, AuthRole, Principal, SessionStore, SESSION_COOKIE};

    fn studio(secret: Option<&str>) -> StudioState {
        StudioState {
            sessions: SessionStore::new(),
            owner_secret: secret.map(str::to_owned),
            catalog: std::sync::Arc::new(oathstar_content::ContentCatalog::default()),
        }
    }

    async fn jar(cookie: Option<&str>) -> CookieJar {
        let mut builder = Request::builder();
        if let Some(value) = cookie {
            builder = builder.header(header::COOKIE, value);
        }
        let request = builder.body(Body::empty()).expect("a request builds");
        let (mut parts, _) = request.into_parts();
        CookieJar::from_request_parts(&mut parts, &())
            .await
            .expect("the cookie jar extractor is infallible")
    }

    async fn body_of(response: axum::response::Response) -> String {
        let bytes = to_bytes(response.into_body(), 1 << 20)
            .await
            .expect("the body collects");
        String::from_utf8(bytes.to_vec()).expect("a utf-8 body")
    }

    fn location(response: &axum::response::Response) -> String {
        response
            .headers()
            .get(header::LOCATION)
            .expect("a Location header")
            .to_str()
            .expect("an ascii Location")
            .to_owned()
    }

    fn cookie_header(id: &str) -> String {
        format!("{SESSION_COOKIE}={id}")
    }

    #[tokio::test]
    async fn login_valid_creates_session_and_sets_cookie() {
        // T7/REQ-002
        let state = studio(Some("hunter2"));
        let sessions = state.sessions.clone();
        let response = login_submit(
            State(state),
            jar(None).await,
            Form(LoginForm {
                secret: "hunter2".to_owned(),
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&response), "/");
        let set = response
            .headers()
            .get(header::SET_COOKIE)
            .expect("a session cookie is set")
            .to_str()
            .expect("an ascii cookie");
        assert!(set.contains(&format!("{SESSION_COOKIE}=")));
        assert!(set.contains("HttpOnly"));
        assert!(set.contains("SameSite=Strict"));

        let id = set
            .split(&format!("{SESSION_COOKIE}="))
            .nth(1)
            .and_then(|rest| rest.split(';').next())
            .expect("the cookie carries the id");
        assert!(sessions
            .resolve(id)
            .expect("the new session is stored")
            .grants(AuthRole::Owner));
    }

    #[tokio::test]
    async fn login_invalid_re_renders_without_a_cookie() {
        // T8/REQ-003
        let state = studio(Some("hunter2"));
        let response = login_submit(
            State(state),
            jar(None).await,
            Form(LoginForm {
                secret: "wrong".to_owned(),
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response.headers().get(header::SET_COOKIE).is_none(),
            "a failed login must not set a session cookie",
        );
        assert!(body_of(response).await.contains("Invalid credentials."));
    }

    #[tokio::test]
    async fn login_refused_when_secret_unconfigured() {
        // T9/REQ-009: no secret configured → nobody logs in.
        let state = studio(None);
        let response = login_submit(
            State(state),
            jar(None).await,
            Form(LoginForm {
                secret: "anything".to_owned(),
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK); // re-render, not a redirect
        assert!(response.headers().get(header::SET_COOKIE).is_none());
    }

    #[tokio::test]
    async fn logout_invalidates_session_and_clears_cookie() {
        // T10/REQ-004
        let state = studio(Some("pw"));
        let sessions = state.sessions.clone();
        let id = sessions.create_session(owner_principal());

        let response = logout(State(state), jar(Some(&cookie_header(&id))).await)
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&response), "/login");
        assert!(response
            .headers()
            .get(header::SET_COOKIE)
            .expect("a clearing cookie is set")
            .to_str()
            .expect("an ascii cookie")
            .contains(&format!("{SESSION_COOKIE}=")));
        assert!(
            sessions.resolve(&id).is_none(),
            "the session is gone server-side",
        );
    }

    #[tokio::test]
    async fn dashboard_redirects_without_a_valid_session() {
        // T11/REQ-001: no cookie and an unknown cookie both redirect to /login.
        let state = studio(Some("pw"));

        let response = dashboard(State(state.clone()), jar(None).await)
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&response), "/login");

        let response = dashboard(State(state), jar(Some(&cookie_header("bogus"))).await)
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&response), "/login");
    }

    #[tokio::test]
    async fn dashboard_renders_for_an_owner_session() {
        // T12/REQ-002
        let state = studio(Some("pw"));
        let id = state.sessions.create_session(owner_principal());
        let response = dashboard(State(state), jar(Some(&cookie_header(&id))).await)
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_of(response).await;
        assert!(body.contains("Sign out"));
        assert!(body.contains("Owner"));
    }

    #[tokio::test]
    async fn dashboard_admits_an_editor_but_not_a_player() {
        // The role gate (defense in depth): an Editor sees the shell, a Player is
        // redirected. Pins `require_role(Editor)` so the gate isn't dead weight.
        let state = studio(Some("pw"));
        let editor = state.sessions.create_session(Principal {
            id: "e".to_owned(),
            name: "Edda".to_owned(),
            roles: vec![AuthRole::Editor],
        });
        let player = state.sessions.create_session(Principal {
            id: "p".to_owned(),
            name: "Pip".to_owned(),
            roles: vec![AuthRole::Player],
        });

        let editor_response = dashboard(
            State(state.clone()),
            jar(Some(&cookie_header(&editor))).await,
        )
        .await
        .into_response();
        assert_eq!(editor_response.status(), StatusCode::OK);

        let player_response = dashboard(State(state), jar(Some(&cookie_header(&player))).await)
            .await
            .into_response();
        assert_eq!(player_response.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&player_response), "/login");
    }

    #[tokio::test]
    async fn login_form_renders_the_sign_in_page() {
        // GET /login serves the form (no error banner).
        let html = body_of(login_form().await.into_response()).await;
        assert!(html.contains(r#"action="/login""#));
        assert!(html.contains(r#"name="secret""#));
    }

    #[tokio::test]
    async fn logout_without_a_session_is_a_harmless_redirect() {
        // The no-cookie branch: logout with no session still clears + redirects.
        let state = studio(Some("pw"));
        let response = logout(State(state), jar(None).await).await.into_response();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&response), "/login");
    }
}
