use anyhow::Result;
use std::sync::Arc;

use crate::api::{history, ApiClient};
use crate::cli::OutputFormat;
use crate::config::Settings;

pub async fn cmd_list(limit: usize, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    let items = history::get_history(&client, limit).await?;

    match output {
        OutputFormat::Json => crate::cli::output::print_json(&items),
        _ => {
            if items.is_empty() {
                println!("No activity history found.");
            }
            for item in &items {
                let ts = item
                    .creation_timestamp
                    .map(|t| {
                        chrono::DateTime::from_timestamp((t / 1000) as i64, 0)
                            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_else(|| t.to_string())
                    })
                    .unwrap_or_default();

                let device = item
                    .device
                    .as_ref()
                    .and_then(|d| d.device_name.as_deref())
                    .unwrap_or("?");

                let summary = item
                    .description
                    .as_ref()
                    .and_then(|d| d.summary_text.as_deref())
                    .unwrap_or("—");

                println!("  {}  {:<30}  {}", ts, device, summary);
            }
        }
    }
    Ok(())
}
