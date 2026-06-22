//! Login via aioamazondevices Python library (handles Amazon's anti-bot protections).

use anyhow::{bail, Context, Result};
use std::process::Command;

use crate::auth::cookie_store::cookie_file_path;
use crate::config::Settings;

const PYTHON_LOGIN_SCRIPT: &str = include_str!("../../scripts/login_helper.py");

pub async fn login(email: &str, _password: &str, settings: &mut Settings) -> Result<()> {
    let password = rpassword::prompt_password("Amazon password: ")?;

    eprintln!("Connecting to Amazon...");

    // Write the script to a temp file
    let tmp_script = std::env::temp_dir().join("alexa_cli_login.py");
    std::fs::write(&tmp_script, PYTHON_LOGIN_SCRIPT)?;

    // Step 1: Trigger OTP send (attempt login without real OTP)
    let output = Command::new("python3")
        .arg(&tmp_script)
        .arg(email)
        .arg(&password)
        .output()
        .context("Failed to run Python login helper. Is python3 installed with aioamazondevices? Run: pip3 install aioamazondevices")?;

    if output.status.code() == Some(2) {
        // OTP was triggered, now ask for it
        eprintln!("OTP code sent to your device.");
        let otp = rpassword::prompt_password("Enter OTP code: ")?;

        eprintln!("Logging in...");
        let output = Command::new("python3")
            .arg(&tmp_script)
            .arg(email)
            .arg(&password)
            .arg(otp.trim())
            .output()
            .context("Failed to run login with OTP")?;

        let _ = std::fs::remove_file(&tmp_script);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Login failed: {}", stderr.trim());
        }

        return save_cookies_from_output(&output.stdout, settings, email);
    }

    let _ = std::fs::remove_file(&tmp_script);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Login failed: {}", stderr.trim());
    }

    save_cookies_from_output(&output.stdout, settings, email)
}

fn save_cookies_from_output(stdout: &[u8], settings: &mut Settings, email: &str) -> Result<()> {
    let stdout_str = String::from_utf8_lossy(stdout);
    let cookies: std::collections::HashMap<String, String> = serde_json::from_str(stdout_str.trim())
        .context("Failed to parse login response")?;

    if cookies.is_empty() {
        bail!("No cookies received from login");
    }

    if !cookies.contains_key("at-main") {
        bail!("Login succeeded but 'at-main' cookie not found");
    }

    // Write cookies to file
    let cookie_lines: Vec<String> = cookies
        .iter()
        .map(|(name, value)| {
            format!(
                "{{\"raw_cookie\":\"{name}={value}; Secure; Path=/; Domain=.amazon.com\",\"path\":[\"/\",true],\"domain\":{{\"Suffix\":\"amazon.com\"}},\"expires\":{{\"AtUtc\":\"2036-01-01T08:00:01Z\"}}}}"
            )
        })
        .collect();

    let path = cookie_file_path()?;
    std::fs::write(&path, cookie_lines.join("\n")).context("Failed to write cookies")?;

    settings.set_email(email);
    settings.mark_authenticated();
    settings.save()?;

    eprintln!("Login successful.");
    Ok(())
}
