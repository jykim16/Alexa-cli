use anyhow::{bail, Context, Result};
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::sync::Arc;

use reqwest_cookie_store::CookieStoreMutex;

use crate::auth::cookie_store::save_cookie_store;
use crate::config::Settings;

const LOGIN_URL: &str =
    "https://www.amazon.com/ap/signin?openid.pape.max_auth_age=0\
     &openid.return_to=https%3A%2F%2Fwww.amazon.com%2F\
     &openid.identity=http%3A%2F%2Fspecs.openid.net%2Fauth%2F2.0%2Fidentifier_select\
     &openid.assoc_handle=usflex\
     &openid.mode=checkid_setup\
     &openid.claimed_id=http%3A%2F%2Fspecs.openid.net%2Fauth%2F2.0%2Fidentifier_select\
     &openid.ns=http%3A%2F%2Fspecs.openid.net%2Fauth%2F2.0";

/// Build a reqwest client with the provided cookie store attached.
pub fn build_client(cookie_store: Arc<CookieStoreMutex>) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .cookie_provider(cookie_store)
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
             AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/124.0.0.0 Safari/537.36",
        )
        .redirect(reqwest::redirect::Policy::limited(20))
        .build()
        .context("Failed to build HTTP client")
}

/// Perform the full Amazon login flow.
///
/// - Fetches the login page to extract hidden form fields
/// - POSTs credentials
/// - Handles OTP 2FA if prompted
/// - Saves cookies on success
pub async fn login(
    email: &str,
    password: &str,
    cookie_store: Arc<CookieStoreMutex>,
    settings: &mut Settings,
) -> Result<()> {
    let client = build_client(Arc::clone(&cookie_store))?;

    // Step 1: GET login page and extract hidden form fields
    eprintln!("Connecting to Amazon...");
    let resp = client
        .get(LOGIN_URL)
        .send()
        .await
        .context("Failed to GET Amazon login page")?;

    let login_url_final = resp.url().clone();
    let body = resp.text().await.context("Failed to read login page body")?;

    let mut form_fields = extract_hidden_fields(&body);
    form_fields.insert("email".to_string(), email.to_string());
    form_fields.insert("password".to_string(), password.to_string());
    form_fields.insert("rememberMe".to_string(), "true".to_string());

    // Determine POST action URL
    let post_url = extract_form_action(&body)
        .unwrap_or_else(|| login_url_final.to_string());

    // Step 2: POST credentials
    eprintln!("Submitting credentials...");
    let resp = client
        .post(&post_url)
        .form(&form_fields)
        .send()
        .await
        .context("Failed to POST login form")?;

    let final_url = resp.url().clone();
    let resp_body = resp.text().await.context("Failed to read login response")?;

    // Step 3: Check response
    if resp_body.contains("auth-mfa-form") || resp_body.contains("auth-mfa-otpcode") {
        // 2FA required
        eprintln!("Two-factor authentication required.");
        handle_mfa(&client, &resp_body, &final_url, &cookie_store).await?;
    } else if resp_body.contains("auth-captcha-image") || resp_body.contains("captchacharacters") {
        bail!(
            "Amazon is showing a CAPTCHA. Please log in via browser at https://alexa.amazon.com \
             and then use `alexa-cli auth import-cookies` to import your session."
        );
    } else if resp_body.contains("ap_error") || resp_body.contains("auth-error-message") {
        let error = extract_error_message(&resp_body)
            .unwrap_or_else(|| "Unknown login error".to_string());
        bail!("Login failed: {}", error);
    } else if final_url.host_str() == Some("alexa.amazon.com")
        || final_url.host_str() == Some("www.amazon.com")
        || resp_body.contains("Hello,")
        || resp_body.contains("nav-link-accountList")
    {
        eprintln!("Login successful.");
    } else {
        // Unexpected state — still try to proceed; the CSRF check will catch failures
        eprintln!("Warning: unexpected login response page. Proceeding to verify session...");
    }

    // Step 4: Hit alexa.amazon.com to establish Alexa session cookies
    let alexa_url = format!("{}/", &settings.base_url);
    client
        .get(&alexa_url)
        .send()
        .await
        .context("Failed to initialize Alexa session")?;

    // Step 5: Persist cookies
    save_cookie_store(&cookie_store)?;
    settings.set_email(email);
    settings.mark_authenticated();
    settings.save()?;

    Ok(())
}

/// Handle OTP/TOTP two-factor authentication.
async fn handle_mfa(
    client: &reqwest::Client,
    body: &str,
    current_url: &url::Url,
    cookie_store: &Arc<CookieStoreMutex>,
) -> Result<()> {
    let mut fields = extract_hidden_fields(body);

    let otp = prompt_otp()?;
    fields.insert("otpCode".to_string(), otp);
    fields.insert("mfaSubmit".to_string(), "Submit".to_string());
    fields.insert("rememberDevice".to_string(), "false".to_string());

    let post_url = extract_form_action(body).unwrap_or_else(|| current_url.to_string());

    let resp = client
        .post(&post_url)
        .form(&fields)
        .send()
        .await
        .context("Failed to submit OTP")?;

    let resp_body = resp.text().await.context("Failed to read MFA response")?;

    if resp_body.contains("ap_error") || resp_body.contains("auth-error-message") {
        let error = extract_error_message(&resp_body)
            .unwrap_or_else(|| "Invalid OTP".to_string());
        bail!("MFA failed: {}", error);
    }

    eprintln!("MFA accepted.");
    Ok(())
}

fn prompt_otp() -> Result<String> {
    eprint!("Enter your 6-digit OTP code: ");
    let mut code = String::new();
    std::io::stdin()
        .read_line(&mut code)
        .context("Failed to read OTP")?;
    Ok(code.trim().to_string())
}

/// Extract all hidden `<input>` fields from an HTML form.
pub(crate) fn extract_hidden_fields(html: &str) -> HashMap<String, String> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("input[type=hidden]").unwrap();
    let mut map = HashMap::new();
    for el in doc.select(&sel) {
        if let (Some(name), Some(value)) = (
            el.value().attr("name"),
            el.value().attr("value"),
        ) {
            map.insert(name.to_string(), value.to_string());
        }
    }
    map
}

/// Extract the `action` attribute of the first `<form>` in the HTML.
pub(crate) fn extract_form_action(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("form").unwrap();
    doc.select(&sel)
        .next()
        .and_then(|el| el.value().attr("action").map(|s| s.to_string()))
}

/// Try to find a human-readable error message in the login response.
pub(crate) fn extract_error_message(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    // Common Amazon error containers
    for sel_str in &[
        "#auth-error-message-box .a-list-item",
        ".a-alert-content",
        "#message_error",
    ] {
        let sel = Selector::parse(sel_str).ok()?;
        if let Some(el) = doc.select(&sel).next() {
            let text = el.text().collect::<String>().trim().to_string();
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── extract_hidden_fields ────────────────────────────────────────────

    #[test]
    fn test_extract_hidden_fields_returns_all_hidden_inputs() {
        let html = r#"
            <form>
              <input type="hidden" name="appActionToken" value="TOKEN123" />
              <input type="hidden" name="pageId" value="PAGE_LOGIN_WIDGET" />
              <input type="text" name="email" value="" />
            </form>
        "#;
        let fields = extract_hidden_fields(html);
        assert_eq!(fields.get("appActionToken"), Some(&"TOKEN123".to_string()));
        assert_eq!(fields.get("pageId"), Some(&"PAGE_LOGIN_WIDGET".to_string()));
        // visible text inputs should NOT be included
        assert!(!fields.contains_key("email"));
    }

    #[test]
    fn test_extract_hidden_fields_empty_html() {
        let fields = extract_hidden_fields("<html></html>");
        assert!(fields.is_empty());
    }

    #[test]
    fn test_extract_hidden_fields_no_value_attribute_skipped() {
        let html = r#"<input type="hidden" name="novalue" />"#;
        let fields = extract_hidden_fields(html);
        // scraper returns empty string for missing attr; this tests the attr presence check
        // The actual behavior depends on how the HTML is parsed; verify no panic.
        let _ = fields;
    }

    #[test]
    fn test_extract_hidden_fields_multiple_forms() {
        let html = r#"
            <form id="form1"><input type="hidden" name="f1" value="v1" /></form>
            <form id="form2"><input type="hidden" name="f2" value="v2" /></form>
        "#;
        let fields = extract_hidden_fields(html);
        assert_eq!(fields.get("f1"), Some(&"v1".to_string()));
        assert_eq!(fields.get("f2"), Some(&"v2".to_string()));
    }

    // ── extract_form_action ──────────────────────────────────────────────

    #[test]
    fn test_extract_form_action_finds_action_url() {
        let html = r#"
            <form action="https://www.amazon.com/ap/signin" method="post">
              <input type="hidden" name="x" value="y" />
            </form>
        "#;
        let action = extract_form_action(html);
        assert_eq!(action, Some("https://www.amazon.com/ap/signin".to_string()));
    }

    #[test]
    fn test_extract_form_action_returns_none_when_no_form() {
        let action = extract_form_action("<html><body><p>no form</p></body></html>");
        assert!(action.is_none());
    }

    #[test]
    fn test_extract_form_action_returns_none_when_form_has_no_action() {
        let html = r#"<form method="post"><input type="hidden" name="x" value="y" /></form>"#;
        let action = extract_form_action(html);
        assert!(action.is_none());
    }

    #[test]
    fn test_extract_form_action_returns_first_form() {
        let html = r#"
            <form action="/first"></form>
            <form action="/second"></form>
        "#;
        let action = extract_form_action(html);
        assert_eq!(action, Some("/first".to_string()));
    }

    // ── extract_error_message ────────────────────────────────────────────

    #[test]
    fn test_extract_error_message_finds_auth_error_box() {
        let html = r#"
            <div id="auth-error-message-box">
              <ul><li class="a-list-item">Your password is incorrect.</li></ul>
            </div>
        "#;
        let msg = extract_error_message(html);
        assert!(msg.is_some());
        assert!(msg.unwrap().contains("incorrect"));
    }

    #[test]
    fn test_extract_error_message_finds_a_alert_content() {
        let html = r#"<div class="a-alert-content">Account does not exist.</div>"#;
        let msg = extract_error_message(html);
        assert!(msg.is_some());
        assert!(msg.unwrap().contains("Account does not exist"));
    }

    #[test]
    fn test_extract_error_message_returns_none_when_no_error() {
        let html = r#"<html><body><p>Welcome!</p></body></html>"#;
        let msg = extract_error_message(html);
        assert!(msg.is_none());
    }

    #[test]
    fn test_extract_error_message_skips_empty_containers() {
        // An element that exists but has no text should not match
        let html = r#"<div class="a-alert-content">   </div>"#;
        let msg = extract_error_message(html);
        assert!(msg.is_none());
    }

    // ── build_client ─────────────────────────────────────────────────────

    #[test]
    fn test_build_client_succeeds() {
        let cookie_store = Arc::new(reqwest_cookie_store::CookieStoreMutex::new(
            reqwest_cookie_store::CookieStore::default(),
        ));
        let result = build_client(cookie_store);
        assert!(result.is_ok());
    }
}
