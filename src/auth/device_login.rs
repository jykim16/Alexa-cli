//! OAuth + device registration login flow.
//! Mimics the Alexa iOS app: form login → auth code → device register → cookies.

use anyhow::{bail, Context, Result};
use scraper::{Html, Selector};
use std::collections::HashMap;

use crate::auth::cookie_store::cookie_file_path;
use crate::config::Settings;

const DEVICE_TYPE: &str = "A2IVLV5VM2W81";
const APP_VERSION: &str = "2.2.663733.0";
const OS_VERSION: &str = "18.5";
const SOFTWARE_VERSION: &str = "35602678";
const USER_AGENT: &str = "AmazonWebView/Amazon Alexa/2.2.663733.0/iOS/18.5/iPhone";

pub async fn login(email: &str, password: &str, settings: &mut Settings) -> Result<()> {
    let serial = settings.ensure_device_serial_number()?;
    let client_id = build_client_id(&serial);
    let code_verifier = create_code_verifier();
    let code_challenge = create_code_challenge(&code_verifier);

    let http = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::limited(20))
        .build()?;

    // Step 1: Load the OAuth login page
    let oauth_url = format!(
        "https://www.amazon.com/ap/signin?\
         openid.return_to=https%3A%2F%2Fwww.amazon.com%2Fap%2Fmaplanding\
         &openid.oa2.code_challenge_method=S256\
         &openid.assoc_handle=amzn_dp_project_dee_ios\
         &openid.identity=http%3A%2F%2Fspecs.openid.net%2Fauth%2F2.0%2Fidentifier_select\
         &pageId=amzn_dp_project_dee_ios\
         &accountStatusPolicy=P1\
         &openid.claimed_id=http%3A%2F%2Fspecs.openid.net%2Fauth%2F2.0%2Fidentifier_select\
         &openid.mode=checkid_setup\
         &openid.ns.oa2=http%3A%2F%2Fwww.amazon.com%2Fap%2Fext%2Foauth%2F2\
         &openid.oa2.client_id=device%3A{client_id}\
         &openid.ns.pape=http%3A%2F%2Fspecs.openid.net%2Fextensions%2Fpape%2F1.0\
         &openid.oa2.scope=device_auth_access\
         &openid.ns=http%3A%2F%2Fspecs.openid.net%2Fauth%2F2.0\
         &openid.pape.max_auth_age=0\
         &openid.oa2.response_type=code\
         &openid.oa2.code_challenge={code_challenge}\
         &language=en_US"
    );

    eprintln!("Connecting to Amazon...");
    let resp = http.get(&oauth_url).send().await.context("Failed to load login page")?;
    let login_url = resp.url().clone();
    let body = resp.text().await?;

    // Step 2: Extract form fields and POST credentials
    // Amazon may use a two-step flow (email first, then password)
    let mut fields = extract_hidden_fields(&body);
    fields.insert("email".to_string(), email.to_string());
    fields.insert("password".to_string(), password.to_string());
    fields.insert("rememberMe".to_string(), "true".to_string());

    let post_url = extract_form_action(&body).unwrap_or_else(|| login_url.to_string());

    eprintln!("Submitting credentials...");
    eprintln!("[debug] Form action: {:?}", extract_form_action(&body));
    eprintln!("[debug] Hidden fields: {}", fields.len());
    eprintln!("[debug] POST to: {}", &post_url[..post_url.len().min(100)]);
    let resp = http.post(&post_url).form(&fields).send().await.context("Failed to POST login")?;
    let final_url = resp.url().clone();
    let resp_body = resp.text().await?;

    // Step 3: Handle MFA if needed
    let auth_code = if resp_body.contains("auth-mfa-otpcode") || resp_body.contains("auth-mfa-form") {
        eprintln!("Two-factor authentication required.");
        let otp = rpassword::prompt_password("Enter OTP code: ")?;
        let mut mfa_fields = extract_hidden_fields(&resp_body);
        mfa_fields.insert("otpCode".to_string(), otp);
        mfa_fields.insert("mfaSubmit".to_string(), "Submit".to_string());
        mfa_fields.insert("rememberDevice".to_string(), "false".to_string());

        let mfa_url = extract_form_action(&resp_body).unwrap_or_else(|| final_url.to_string());
        let resp = http.post(&mfa_url).form(&mfa_fields).send().await?;
        extract_auth_code(resp.url())?
    } else if resp_body.contains("auth-captcha") {
        bail!("Amazon is showing a CAPTCHA. Try again later or use `auth import-cookies`.");
    } else {
        // Should have redirected to maplanding with the code
        eprintln!("[debug] Final URL: {}", final_url);
        eprintln!("[debug] Body contains 'maplanding': {}", resp_body.contains("maplanding"));
        eprintln!("[debug] Body contains 'error': {}", resp_body.contains("ap_error") || resp_body.contains("auth-error"));
        eprintln!("[debug] Body length: {}", resp_body.len());
        extract_auth_code(&final_url)
            .or_else(|_| extract_auth_code_from_body(&resp_body))?
    };

    eprintln!("Registering device...");

    // Step 4: Register device to get cookies
    let register_body = serde_json::json!({
        "requested_extensions": ["device_info", "customer_info"],
        "cookies": {
            "website_cookies": [],
            "domain": ".amazon.com"
        },
        "registration_data": {
            "domain": "Device",
            "app_version": APP_VERSION,
            "device_type": DEVICE_TYPE,
            "device_name": "%FIRST_NAME%'s%DUPE_STRATEGY_1ST%alexa-cli",
            "os_version": OS_VERSION,
            "device_serial": serial,
            "device_model": "iPhone",
            "app_name": "Amazon Alexa",
            "software_version": SOFTWARE_VERSION
        },
        "auth_data": {
            "use_global_authentication": "true",
            "client_id": client_id,
            "authorization_code": auth_code,
            "code_verifier": code_verifier,
            "code_algorithm": "SHA-256",
            "client_domain": "DeviceLegacy"
        },
        "requested_token_type": [
            "bearer",
            "mac_dms",
            "website_cookies",
            "store_authentication_cookie"
        ]
    });

    let resp = http
        .post("https://api.amazon.com/auth/register")
        .json(&register_body)
        .send()
        .await
        .context("Failed to register device")?;

    let status = resp.status();
    let reg_body = resp.text().await?;

    if !status.is_success() {
        bail!("Device registration failed ({}): {}", status, &reg_body[..reg_body.len().min(300)]);
    }

    let reg_json: serde_json::Value = serde_json::from_str(&reg_body)
        .context("Failed to parse registration response")?;

    // Step 5: Extract cookies and write to file
    let tokens = &reg_json["response"]["success"]["tokens"];
    let mut cookie_lines = Vec::new();

    // Get website_cookies
    if let Some(cookies) = tokens["website_cookies"].as_array() {
        for c in cookies {
            let name = c["Name"].as_str().unwrap_or("");
            let value = c["Value"].as_str().unwrap_or("").replace('"', "");
            if !name.is_empty() {
                cookie_lines.push(format!(
                    "{{\"raw_cookie\":\"{name}={value}; Secure; Path=/; Domain=.amazon.com\",\"path\":[\"/\",true],\"domain\":{{\"Suffix\":\"amazon.com\"}},\"expires\":{{\"AtUtc\":\"2036-01-01T08:00:01Z\"}}}}"
                ));
            }
        }
    }

    // Also save refresh token
    if let Some(rt) = tokens["bearer"]["refresh_token"].as_str() {
        settings.refresh_token = Some(rt.to_string());
    }

    if cookie_lines.is_empty() {
        bail!("No cookies received from device registration");
    }

    let path = cookie_file_path()?;
    std::fs::write(&path, cookie_lines.join("\n")).context("Failed to write cookies")?;

    settings.set_email(email);
    settings.mark_authenticated();
    settings.save()?;

    eprintln!("Login successful.");
    Ok(())
}

fn build_client_id(serial: &str) -> String {
    let mut source = Vec::new();
    source.extend_from_slice(serial.as_bytes());
    source.push(b'#');
    source.extend_from_slice(DEVICE_TYPE.as_bytes());
    source.iter().map(|b| format!("{:02x}", b)).collect()
}

fn create_code_verifier() -> String {
    let mut bytes = [0u8; 32];
    rand::fill(&mut bytes);
    base64_url_encode(&bytes)
}

fn create_code_challenge(verifier: &str) -> String {
    use sha2::Digest;
    let hash = sha2::Sha256::digest(verifier.as_bytes());
    base64_url_encode(&hash)
}

fn base64_url_encode(input: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(input)
}

fn extract_hidden_fields(html: &str) -> HashMap<String, String> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("input[type=hidden]").unwrap();
    let mut map = HashMap::new();
    for el in doc.select(&sel) {
        if let (Some(name), Some(value)) = (el.value().attr("name"), el.value().attr("value")) {
            map.insert(name.to_string(), value.to_string());
        }
    }
    map
}

fn extract_form_action(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    // Try signIn form first
    let sel = Selector::parse("form[name=signIn]").unwrap();
    if let Some(form) = doc.select(&sel).next() {
        if let Some(action) = form.value().attr("action") {
            return Some(action.to_string());
        }
    }
    // Fall back to any form with action
    let sel = Selector::parse("form[action]").unwrap();
    doc.select(&sel)
        .next()
        .and_then(|el| el.value().attr("action").map(|s| s.to_string()))
}

fn extract_auth_code(url: &url::Url) -> Result<String> {
    let query = url.query().unwrap_or("");
    for pair in query.split('&') {
        if let Some(value) = pair.strip_prefix("openid.oa2.authorization_code=") {
            return Ok(urlencoding::decode(value)?.to_string());
        }
    }
    bail!("No authorization code found in redirect URL: {}", url)
}

fn extract_auth_code_from_body(body: &str) -> Result<String> {
    // Sometimes the code is in a hidden form field on the maplanding page
    let fields = extract_hidden_fields(body);
    if let Some(code) = fields.get("openid.oa2.authorization_code") {
        return Ok(code.clone());
    }
    bail!("Could not extract authorization code from response")
}
