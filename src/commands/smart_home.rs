use anyhow::{bail, Result};
use std::sync::Arc;

use crate::api::{smart_home, ApiClient};
use crate::cli::OutputFormat;
use crate::config::Settings;

pub async fn cmd_list(output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    let devs = smart_home::list_smart_home_devices(&client).await?;

    match output {
        OutputFormat::Json => crate::cli::output::print_json(&devs),
        _ => {
            if devs.is_empty() {
                println!("No smart home devices found.");
            }
            for d in &devs {
                let types = d
                    .appliance_types
                    .as_ref()
                    .map(|v| v.join(", "))
                    .unwrap_or_default();
                println!(
                    "  {:<40} {}",
                    d.friendly_name.as_deref().unwrap_or("?"),
                    types
                );
            }
        }
    }
    Ok(())
}

pub async fn cmd_power(device_name: &str, state: &str, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    let devs = smart_home::list_smart_home_devices(&client).await?;
    let dev = smart_home::find_device(&devs, device_name)
        .ok_or_else(|| anyhow::anyhow!("Device not found: {}", device_name))?;
    let id = dev
        .appliance_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("Device has no ID"))?;

    let action = match state.to_lowercase().as_str() {
        "on" => "turnOn",
        "off" => "turnOff",
        "toggle" => {
            // Check current state (not always available, default to turnOn)
            "turnOn"
        }
        _ => bail!("Invalid state: {}. Use on, off, or toggle.", state),
    };

    smart_home::power(&client, id, action).await?;
    match output {
        OutputFormat::Json => println!(
            "{{\"device\":\"{}\",\"state\":\"{}\" }}",
            device_name, state
        ),
        _ => println!("{} turned {}.", device_name, state),
    }
    Ok(())
}

pub async fn cmd_brightness(device_name: &str, level: u8, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    let devs = smart_home::list_smart_home_devices(&client).await?;
    let dev = smart_home::find_device(&devs, device_name)
        .ok_or_else(|| anyhow::anyhow!("Device not found: {}", device_name))?;
    let id = dev
        .appliance_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("Device has no ID"))?;
    smart_home::set_brightness(&client, id, level).await?;
    match output {
        OutputFormat::Json => println!(
            "{{\"device\":\"{}\",\"brightness\":{}  }}",
            device_name, level
        ),
        _ => println!("{} brightness set to {}%.", device_name, level),
    }
    Ok(())
}

pub async fn cmd_color(device_name: &str, color: &str, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    let devs = smart_home::list_smart_home_devices(&client).await?;
    let dev = smart_home::find_device(&devs, device_name)
        .ok_or_else(|| anyhow::anyhow!("Device not found: {}", device_name))?;
    let id = dev
        .appliance_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("Device has no ID"))?;
    smart_home::set_color(&client, id, color).await?;
    match output {
        OutputFormat::Json => println!(
            "{{\"device\":\"{}\",\"color\":\"{}\" }}",
            device_name, color
        ),
        _ => println!("{} color set to {}.", device_name, color),
    }
    Ok(())
}

pub async fn cmd_thermostat(
    device_name: &str,
    temp: f64,
    unit: &str,
    output: OutputFormat,
) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    let devs = smart_home::list_smart_home_devices(&client).await?;
    let dev = smart_home::find_device(&devs, device_name)
        .ok_or_else(|| anyhow::anyhow!("Device not found: {}", device_name))?;
    let id = dev
        .appliance_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("Device has no ID"))?;
    let scale = if unit.to_uppercase() == "C" {
        "CELSIUS"
    } else {
        "FAHRENHEIT"
    };
    smart_home::set_thermostat(&client, id, temp, scale).await?;
    match output {
        OutputFormat::Json => println!(
            "{{\"device\":\"{}\",\"temp\":{},\"unit\":\"{}\" }}",
            device_name, temp, unit
        ),
        _ => println!("{} set to {}°{}.", device_name, temp, unit.to_uppercase()),
    }
    Ok(())
}

pub async fn cmd_lock(device_name: &str, state: &str, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    let devs = smart_home::list_smart_home_devices(&client).await?;
    let dev = smart_home::find_device(&devs, device_name)
        .ok_or_else(|| anyhow::anyhow!("Device not found: {}", device_name))?;
    let id = dev
        .appliance_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("Device has no ID"))?;
    let locked = match state.to_lowercase().as_str() {
        "lock" | "locked" => true,
        "unlock" | "unlocked" => false,
        _ => bail!("Invalid state: {}. Use lock or unlock.", state),
    };
    smart_home::lock(&client, id, locked).await?;
    match output {
        OutputFormat::Json => {
            println!("{{\"device\":\"{}\",\"locked\":{}  }}", device_name, locked)
        }
        _ => println!("{} {}ed.", device_name, state),
    }
    Ok(())
}
