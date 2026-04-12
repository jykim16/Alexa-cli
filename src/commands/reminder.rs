use anyhow::{bail, Result};
use std::sync::Arc;

use crate::api::{devices, reminders, ApiClient};
use crate::cli::OutputFormat;
use crate::config::Settings;

pub async fn cmd_list(device_name: Option<&str>, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(Arc::clone(&settings)).await?;
    let devs = devices::list_devices(&client).await?;

    let sn = {
        let name = device_name
            .or(settings.default_device.as_deref())
            .unwrap_or("");
        let dev = if name.is_empty() {
            devs.first()
        } else {
            devices::find_device(&devs, name)
        };
        match dev {
            Some(d) => d.serial_number.clone(),
            None => bail!("No device found. Use --device."),
        }
    };

    let items = reminders::list_reminders(&client, &sn).await?;
    match output {
        OutputFormat::Json => crate::cli::output::print_json(&items),
        _ => {
            if items.is_empty() {
                println!("No reminders.");
            }
            for r in &items {
                let ts = r
                    .trigger_time
                    .map(|t| {
                        chrono::DateTime::from_timestamp((t / 1000) as i64, 0)
                            .map(|dt| dt.to_rfc3339())
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();
                println!(
                    "{:<30}  {}  {}",
                    r.id.as_deref().unwrap_or("?"),
                    ts,
                    r.reminder_label.as_deref().unwrap_or("?")
                );
            }
        }
    }
    Ok(())
}

pub async fn cmd_create(
    text: &str,
    time: &str,
    device_name: Option<&str>,
    output: OutputFormat,
) -> Result<()> {
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
    let dev = dev.ok_or_else(|| anyhow::anyhow!("No device found. Use --device."))?;

    let result =
        reminders::create_reminder(&client, &dev.serial_number, &dev.device_type, text, time)
            .await?;
    match output {
        OutputFormat::Json => crate::cli::output::print_json(&result),
        _ => println!("Reminder created: \"{}\" at {}", text, time),
    }
    Ok(())
}

pub async fn cmd_delete(id: &str, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    reminders::delete_reminder(&client, id).await?;
    match output {
        OutputFormat::Json => println!("{{\"deleted\":\"{}\" }}", id),
        _ => println!("Reminder {} deleted.", id),
    }
    Ok(())
}
