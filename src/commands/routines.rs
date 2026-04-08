use anyhow::{bail, Result};
use std::sync::Arc;

use crate::api::{automations, behaviors, ApiClient};
use crate::cli::OutputFormat;
use crate::config::Settings;

pub async fn cmd_list(output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    let items = automations::list_automations(&client).await?;

    match output {
        OutputFormat::Json => crate::cli::output::print_json(&items),
        _ => {
            if items.is_empty() {
                println!("No routines found.");
            }
            for a in &items {
                println!(
                    "  {} — {}",
                    a.name.as_deref().unwrap_or("Unnamed"),
                    a.status.as_deref().unwrap_or("?")
                );
            }
        }
    }
    Ok(())
}

pub async fn cmd_run(name: &str, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    let items = automations::list_automations(&client).await?;

    let automation = automations::find_automation(&items, name)
        .ok_or_else(|| anyhow::anyhow!("Routine not found: {}", name))?;

    let seq = automation
        .sequence
        .as_ref()
        .and_then(|s| s.sequence_json.as_deref())
        .ok_or_else(|| anyhow::anyhow!("Routine has no sequence: {}", name))?;

    behaviors::run_routine_sequence(&client, seq).await?;

    match output {
        OutputFormat::Json => println!(
            "{{\"ran\":\"{}\" }}",
            automation.name.as_deref().unwrap_or(name)
        ),
        _ => println!("Ran routine: {}", automation.name.as_deref().unwrap_or(name)),
    }
    Ok(())
}
