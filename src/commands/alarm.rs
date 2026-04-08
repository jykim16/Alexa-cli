use anyhow::{bail, Result};
use std::sync::Arc;

use crate::api::{alarms, devices, ApiClient};
use crate::cli::OutputFormat;
use crate::config::Settings;

async fn default_device_info(client: &ApiClient, settings: &Settings) -> Result<(String, String)> {
    let devs = devices::list_devices(client).await?;
    let name = settings.default_device.as_deref().unwrap_or("");
    let dev = if name.is_empty() {
        devs.first()
    } else {
        devices::find_device(&devs, name)
    };
    match dev {
        Some(d) => Ok((d.serial_number.clone(), d.device_type.clone())),
        None => bail!("No device found. Set a default with --device or in config."),
    }
}

pub async fn cmd_list(device_name: Option<&str>, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(Arc::clone(&settings)).await?;
    let items = alarms::list_alarms(&client).await?;

    // Optionally filter by device
    let filtered: Vec<_> = if let Some(name) = device_name {
        let devs = devices::list_devices(&client).await?;
        if let Some(dev) = devices::find_device(&devs, name) {
            items
                .iter()
                .filter(|a| a.device_serial_number == dev.serial_number)
                .collect()
        } else {
            bail!("Device not found: {}", name);
        }
    } else {
        items.iter().collect()
    };

    match output {
        OutputFormat::Json => crate::cli::output::print_json(&filtered),
        _ => {
            if filtered.is_empty() {
                println!("No alarms.");
            }
            for a in &filtered {
                println!(
                    "{:<30}  {}  {}",
                    a.id.as_deref().unwrap_or("?"),
                    a.date_time.as_deref().unwrap_or("?"),
                    a.status.as_deref().unwrap_or("?")
                );
            }
        }
    }
    Ok(())
}

pub async fn cmd_create(
    time: &str,
    label: Option<&str>,
    device_name: Option<&str>,
    output: OutputFormat,
) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(Arc::clone(&settings)).await?;
    let (sn, dt) = if let Some(name) = device_name {
        let devs = devices::list_devices(&client).await?;
        match devices::find_device(&devs, name) {
            Some(d) => (d.serial_number.clone(), d.device_type.clone()),
            None => bail!("Device not found: {}", name),
        }
    } else {
        default_device_info(&client, &settings).await?
    };

    let alarm = alarms::create_alarm(&client, &sn, &dt, time, label).await?;
    match output {
        OutputFormat::Json => crate::cli::output::print_json(&alarm),
        _ => println!(
            "Alarm created: {} at {}",
            alarm.id.as_deref().unwrap_or("?"),
            alarm.date_time.as_deref().unwrap_or(time)
        ),
    }
    Ok(())
}

pub async fn cmd_delete(id: &str, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    alarms::delete_alarm(&client, id).await?;
    match output {
        OutputFormat::Json => println!("{{\"deleted\":\"{}\" }}", id),
        _ => println!("Alarm {} deleted.", id),
    }
    Ok(())
}

pub async fn cmd_enable(id: &str, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    alarms::set_alarm_enabled(&client, id, true).await?;
    match output {
        OutputFormat::Json => println!("{{\"enabled\":\"{}\" }}", id),
        _ => println!("Alarm {} enabled.", id),
    }
    Ok(())
}

pub async fn cmd_disable(id: &str, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    alarms::set_alarm_enabled(&client, id, false).await?;
    match output {
        OutputFormat::Json => println!("{{\"disabled\":\"{}\" }}", id),
        _ => println!("Alarm {} disabled.", id),
    }
    Ok(())
}
