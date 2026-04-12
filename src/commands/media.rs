use anyhow::{bail, Result};
use std::sync::Arc;

use crate::api::{behaviors, devices, media, ApiClient};
use crate::cli::OutputFormat;
use crate::config::Settings;

async fn get_device_info(
    client: &ApiClient,
    device_name: Option<&str>,
    settings: &Settings,
) -> Result<(String, String)> {
    let devs = devices::list_devices(client).await?;
    let name = device_name
        .or(settings.default_device.as_deref())
        .unwrap_or("");

    let dev = if name.is_empty() {
        devs.first()
    } else {
        devices::find_device(&devs, name)
    };

    match dev {
        Some(d) => Ok((d.serial_number.clone(), d.device_type.clone())),
        None => bail!("No device found. Use --device to specify one."),
    }
}

pub async fn cmd_play(device_name: Option<&str>, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(Arc::clone(&settings)).await?;
    let (sn, dt) = get_device_info(&client, device_name, &settings).await?;
    media::play(&client, &sn, &dt).await?;
    match output {
        OutputFormat::Json => println!("{{\"status\":\"playing\"}}"),
        _ => println!("Playback resumed."),
    }
    Ok(())
}

pub async fn cmd_pause(device_name: Option<&str>, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(Arc::clone(&settings)).await?;
    let (sn, dt) = get_device_info(&client, device_name, &settings).await?;
    media::pause(&client, &sn, &dt).await?;
    match output {
        OutputFormat::Json => println!("{{\"status\":\"paused\"}}"),
        _ => println!("Paused."),
    }
    Ok(())
}

pub async fn cmd_next(device_name: Option<&str>, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(Arc::clone(&settings)).await?;
    let (sn, dt) = get_device_info(&client, device_name, &settings).await?;
    media::next(&client, &sn, &dt).await?;
    match output {
        OutputFormat::Json => println!("{{\"status\":\"next\"}}"),
        _ => println!("Skipped to next."),
    }
    Ok(())
}

pub async fn cmd_prev(device_name: Option<&str>, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(Arc::clone(&settings)).await?;
    let (sn, dt) = get_device_info(&client, device_name, &settings).await?;
    media::previous(&client, &sn, &dt).await?;
    match output {
        OutputFormat::Json => println!("{{\"status\":\"previous\"}}"),
        _ => println!("Went to previous."),
    }
    Ok(())
}

pub async fn cmd_volume(level: u8, device_name: Option<&str>, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(Arc::clone(&settings)).await?;
    let (sn, dt) = get_device_info(&client, device_name, &settings).await?;
    media::set_volume(&client, &sn, &dt, level).await?;
    match output {
        OutputFormat::Json => println!("{{\"volume\":{}}}", level),
        _ => println!("Volume set to {}.", level),
    }
    Ok(())
}

pub async fn cmd_status(device_name: Option<&str>, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(Arc::clone(&settings)).await?;
    let (sn, dt) = get_device_info(&client, device_name, &settings).await?;
    let np = media::get_now_playing(&client, &sn, &dt).await?;

    match output {
        OutputFormat::Json => crate::cli::output::print_json(&np),
        _ => match np {
            Some(info) => {
                crate::cli::output::print_pairs(&[
                    ("State", info.state.unwrap_or_else(|| "unknown".to_string())),
                    ("Title", info.title.unwrap_or_else(|| "—".to_string())),
                    (
                        "Header",
                        info.header_text.unwrap_or_else(|| "—".to_string()),
                    ),
                ]);
            }
            None => println!("Nothing playing."),
        },
    }
    Ok(())
}

pub async fn cmd_music(
    query: &str,
    service: Option<&str>,
    device_name: Option<&str>,
    output: OutputFormat,
) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let locale = settings.locale.clone();
    let client = ApiClient::new(Arc::clone(&settings)).await?;
    let (sn, dt) = get_device_info(&client, device_name, &settings).await?;
    behaviors::play_music(&client, query, &sn, &dt, &locale, service).await?;
    match output {
        OutputFormat::Json => println!("{{\"playing\":\"{}\"}}", query),
        _ => println!("Playing: {}", query),
    }
    Ok(())
}
