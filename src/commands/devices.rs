use anyhow::Result;
use std::sync::Arc;

use crate::api::{devices, ApiClient};
use crate::cli::OutputFormat;
use crate::config::Settings;

pub async fn cmd_list(output: OutputFormat, device_filter: Option<&str>) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    let devs = devices::list_devices(&client).await?;

    let filtered: Vec<_> = if let Some(f) = device_filter {
        let lower = f.to_lowercase();
        devs.iter()
            .filter(|d| d.account_name.to_lowercase().contains(&lower))
            .collect()
    } else {
        devs.iter().collect()
    };

    match output {
        OutputFormat::Json => crate::cli::output::print_json(&filtered),
        _ => {
            if filtered.is_empty() {
                println!("No devices found.");
            } else {
                for d in &filtered {
                    println!(
                        "{:<40} {:<20} {}",
                        d.account_name,
                        d.device_family,
                        if d.online.unwrap_or(false) { "online" } else { "offline" }
                    );
                }
            }
        }
    }
    Ok(())
}

pub async fn cmd_get(name: &str, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    let devs = devices::list_devices(&client).await?;

    match devices::find_device(&devs, name) {
        Some(d) => match output {
            OutputFormat::Json => crate::cli::output::print_json(d),
            _ => {
                crate::cli::output::print_pairs(&[
                    ("Name", d.account_name.clone()),
                    ("Family", d.device_family.clone()),
                    ("Type", d.device_type.clone()),
                    ("Serial", d.serial_number.clone()),
                    ("Online", d.online.unwrap_or(false).to_string()),
                    (
                        "Software",
                        d.software_version.clone().unwrap_or_default(),
                    ),
                ]);
            }
        },
        None => anyhow::bail!("Device not found: {}", name),
    }
    Ok(())
}
