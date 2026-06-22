//! Login via aioamazondevices Python library (handles Amazon's anti-bot protections).

use anyhow::{bail, Context, Result};
use std::process::Command;

use crate::auth::cookie_store::cookie_file_path;
use crate::config::Settings;

const PYTHON_LOGIN_SCRIPT: &str = include_str!("../../scripts/login_helper.py");

pub async fn login(email: &str, _password: &str, settings: &mut Settings) -> Result<()> {
    let password = rpassword::prompt_password("Amazon password: ")?;

    eprintln!("Enter your OTP code (or press Enter if 2FA not enabled):");
    let mut otp = String::new();
    std::io::stdin().read_line(&mut otp)?;
    let otp = otp.trim();

    eprintln!("Logging in...");

    // Write the script to a temp file and run it
    let tmp_script = std::env::temp_dir().join("alexa_cli_login.py");
    std::fs::write(&tmp_script, PYTHON_LOGIN_SCRIPT)?;

    let mut cmd = Command::new("python3");
    cmd.arg(&tmp_script).arg(email).arg(&password).arg(otp);

    let output = cmd.output().context("Failed to run Python login helper. Is python3 installed with aioamazondevices? Run: pip3 install aioamazondevices")?;

    // Clean up
    let _ = std::fs::remove_file(&tmp_script);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Login failed: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let cookies: std::collections::HashMap<String, String> = serde_json::from_str(stdout.trim())
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
