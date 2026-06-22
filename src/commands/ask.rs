use anyhow::{bail, Result};
use std::sync::Arc;

use crate::api::{behaviors, devices, ApiClient};
use crate::cli::OutputFormat;
use crate::config::Settings;

pub async fn cmd_ask(text: &str, device_name: Option<&str>, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let locale = settings.locale.clone();
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
            let owner_id = d.device_owner_customer_id.as_deref().unwrap_or("");
            behaviors::text_command(&client, text, &d.serial_number, &d.device_type, owner_id, &locale)
                .await?;
            match output {
                OutputFormat::Json => println!("{{\"asked\":\"{}\"}}", text),
                _ => println!("Asked: \"{}\"", text),
            }
        }
        None => bail!("No device found. Use --device to specify one."),
    }
    Ok(())
}
