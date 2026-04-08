use anyhow::{bail, Result};
use std::sync::Arc;

use crate::api::{devices, timers, ApiClient};
use crate::cli::OutputFormat;
use crate::config::Settings;

async fn default_device(client: &ApiClient, settings: &Settings) -> Result<(String, String)> {
    let devs = devices::list_devices(client).await?;
    let name = settings.default_device.as_deref().unwrap_or("");
    let dev = if name.is_empty() { devs.first() } else { devices::find_device(&devs, name) };
    match dev {
        Some(d) => Ok((d.serial_number.clone(), d.device_type.clone())),
        None => bail!("No device found. Use --device or set default in config."),
    }
}

pub async fn cmd_list(device_name: Option<&str>, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(Arc::clone(&settings)).await?;
    let items = timers::list_timers(&client).await?;

    match output {
        OutputFormat::Json => crate::cli::output::print_json(&items),
        _ => {
            if items.is_empty() { println!("No running timers."); }
            for t in &items {
                println!(
                    "{:<30}  {}s remaining  {}",
                    t.id.as_deref().unwrap_or("?"),
                    t.remaining_time.unwrap_or(0),
                    t.status.as_deref().unwrap_or("?")
                );
            }
        }
    }
    Ok(())
}

pub async fn cmd_create(
    duration: &str,
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
        default_device(&client, &settings).await?
    };
    let secs = timers::parse_duration(duration);
    if secs == 0 {
        bail!("Invalid duration: {}. Use formats like 1h30m, 90m, 45s.", duration);
    }
    let result = timers::create_timer(&client, &sn, &dt, secs, label).await?;
    match output {
        OutputFormat::Json => crate::cli::output::print_json(&result),
        _ => println!("Timer set for {} seconds.", secs),
    }
    Ok(())
}

pub async fn cmd_cancel(id: &str, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    timers::cancel_timer(&client, id).await?;
    match output {
        OutputFormat::Json => println!("{{\"cancelled\":\"{}\" }}", id),
        _ => println!("Timer {} cancelled.", id),
    }
    Ok(())
}

pub async fn cmd_pause(id: &str, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    timers::pause_timer(&client, id).await?;
    match output {
        OutputFormat::Json => println!("{{\"paused\":\"{}\" }}", id),
        _ => println!("Timer {} paused.", id),
    }
    Ok(())
}

pub async fn cmd_resume(id: &str, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    timers::resume_timer(&client, id).await?;
    match output {
        OutputFormat::Json => println!("{{\"resumed\":\"{}\" }}", id),
        _ => println!("Timer {} resumed.", id),
    }
    Ok(())
}
