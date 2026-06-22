use anyhow::{bail, Result};
use std::sync::Arc;

use crate::api::{devices, dnd, ApiClient};
use crate::cli::OutputFormat;
use crate::config::Settings;

pub async fn cmd_status(device_name: Option<&str>, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(Arc::clone(&settings)).await?;
    let statuses = dnd::get_dnd_status(&client).await?;

    let filtered: Vec<_> = if let Some(name) = device_name {
        let devs = devices::list_devices(&client).await?;
        if let Some(dev) = devices::find_device(&devs, name) {
            statuses
                .iter()
                .filter(|s| s.device_serial_number.as_deref() == Some(&dev.serial_number))
                .collect()
        } else {
            bail!("Device not found: {}", name);
        }
    } else {
        statuses.iter().collect()
    };

    match output {
        OutputFormat::Json => crate::cli::output::print_json(&filtered),
        _ => {
            if filtered.is_empty() {
                println!("No DND status available.");
            }
            for s in &filtered {
                println!(
                    "  {}  DND: {}",
                    s.device_serial_number.as_deref().unwrap_or("?"),
                    if s.enabled.unwrap_or(false) {
                        "ON"
                    } else {
                        "OFF"
                    }
                );
            }
        }
    }
    Ok(())
}

pub async fn cmd_set(enabled: bool, device_name: Option<&str>, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(Arc::clone(&settings)).await?;
    let devs = devices::list_devices(&client).await?;

    let name = device_name
        .or(settings.default_device.as_deref())
        .unwrap_or("");
    let dev = if name.is_empty() {
        devs.first()
    } else {
        devices::find_device(&devs, name)
    };

    match dev {
        Some(d) => {
            dnd::set_dnd(&client, &d.serial_number, &d.device_type, enabled).await?;
            let state = if enabled { "enabled" } else { "disabled" };
            match output {
                OutputFormat::Json => println!("{{\"dnd\":{} }}", enabled),
                _ => println!("DND {} for {}.", state, d.account_name),
            }
        }
        None => bail!("No device found. Use --device."),
    }
    Ok(())
}
