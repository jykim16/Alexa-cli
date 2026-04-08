use anyhow::{bail, Result};
use std::sync::Arc;

use crate::api::{behaviors, devices, ApiClient};
use crate::cli::OutputFormat;
use crate::config::Settings;

pub async fn cmd_say(
    text: &str,
    device_name: Option<&str>,
    output: OutputFormat,
) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let locale = settings.locale.clone();
    let client = ApiClient::new(Arc::clone(&settings)).await?;
    let devs = devices::list_devices(&client).await?;

    let name = device_name.or(settings.default_device.as_deref()).unwrap_or("");
    let dev = if name.is_empty() {
        devs.first()
    } else {
        devices::find_device(&devs, name)
    };

    match dev {
        Some(d) => {
            behaviors::speak(&client, text, &d.serial_number, &d.device_type, &locale).await?;
            match output {
                OutputFormat::Json => println!("{{\"said\":\"{}\"}}", text),
                _ => println!("Said: \"{}\"", text),
            }
        }
        None => bail!("No device found. Use --device to specify one."),
    }
    Ok(())
}

pub async fn cmd_announce(
    text: &str,
    device_names: Option<&str>,
    output: OutputFormat,
) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let locale = settings.locale.clone();
    let client = ApiClient::new(Arc::clone(&settings)).await?;
    let all_devs = devices::list_devices(&client).await?;

    let targets: Vec<(String, String)> = if let Some(names_str) = device_names {
        names_str
            .split(',')
            .filter_map(|n| {
                let n = n.trim();
                devices::find_device(&all_devs, n)
                    .map(|d| (d.serial_number.clone(), d.device_type.clone()))
            })
            .collect()
    } else {
        all_devs
            .iter()
            .map(|d| (d.serial_number.clone(), d.device_type.clone()))
            .collect()
    };

    if targets.is_empty() {
        bail!("No devices found to announce to.");
    }

    behaviors::announce(&client, text, &targets, &locale).await?;

    match output {
        OutputFormat::Json => println!(
            "{{\"announced\":\"{}\",\"devices\":{}}}",
            text,
            targets.len()
        ),
        _ => println!("Announced to {} device(s): \"{}\"", targets.len(), text),
    }
    Ok(())
}
