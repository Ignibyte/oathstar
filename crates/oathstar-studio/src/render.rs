use axum::response::Html;
use oathstar_auth::Principal;

const STUDIO_CSS: &str = include_str!("../static/studio.css");

/// Render the sign-in page.
///
/// `error` is a server-controlled constant (e.g. "Invalid credentials."); no
/// caller input is ever reflected into the page, so there is no injection surface.
pub fn login_page(error: Option<&str>) -> Html<String> {
    let banner = error.map_or_else(String::new, |message| {
        format!(r#"<p class="error" role="alert">{message}</p>"#)
    });
    Html(format!(
        r#"<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Oathstar Studio — Sign in</title><style>{STUDIO_CSS}</style></head>
<body class="login">
  <main class="card">
    <h1>Oathstar Studio</h1>
    {banner}
    <form method="post" action="/login">
      <label for="secret">Owner password</label>
      <input id="secret" name="secret" type="password" autocomplete="current-password" autofocus>
      <button type="submit">Sign in</button>
    </form>
  </main>
</body>
</html>"#
    ))
}

/// Render the authenticated dashboard shell for `principal`.
///
/// `principal` is server-constructed (the owner) in v1; when a real user store
/// lands, escape its fields before they reach the page.
pub fn dashboard_page(principal: &Principal) -> Html<String> {
    Html(format!(
        r#"<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Oathstar Studio</title><style>{STUDIO_CSS}</style></head>
<body class="dashboard">
  <header>
    <h1>Oathstar Studio</h1>
    <form method="post" action="/logout"><button type="submit">Sign out</button></form>
  </header>
  <main>
    <p class="who">Signed in as <strong>{name}</strong>.</p>
    <section class="panel"><h2>World management</h2><p class="soon">Coming soon.</p></section>
    <section class="panel"><h2>Map editor</h2><p class="soon">Coming soon (ticket #43).</p></section>
  </main>
</body>
</html>"#,
        name = principal.name
    ))
}

#[cfg(test)]
mod tests {
    use super::{dashboard_page, login_page};
    use oathstar_auth::owner_principal;

    #[test]
    fn login_page_has_form_and_embedded_css() {
        // T14/REQ-008: the login page is self-contained (form + embedded CSS).
        let html = login_page(None).0;
        assert!(html.contains(r#"action="/login""#));
        assert!(html.contains(r#"name="secret""#));
        assert!(html.contains("Oathstar Studio"));
        assert!(html.contains(".card")); // embedded CSS marker
        assert!(!html.contains(r#"role="alert""#)); // no banner without an error
    }

    #[test]
    fn login_page_renders_the_error_banner() {
        // T8/REQ-003: an invalid login re-renders with a visible error.
        let html = login_page(Some("Invalid credentials.")).0;
        assert!(html.contains("Invalid credentials."));
        assert!(html.contains(r#"class="error""#));
    }

    #[test]
    fn dashboard_page_has_shell_markers() {
        // T12/T14: the dashboard shows the owner + sign-out + the shell sections.
        let html = dashboard_page(&owner_principal()).0;
        assert!(html.contains("Sign out"));
        assert!(html.contains("Owner")); // principal.name, server-constructed
        assert!(html.contains("Map editor"));
        assert!(html.contains(".panel")); // embedded CSS marker
    }
}
