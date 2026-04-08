use anyhow::Result;
use std::sync::Arc;

use crate::api::{lists, ApiClient};
use crate::cli::OutputFormat;
use crate::config::Settings;

pub async fn cmd_list(output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    let items = lists::get_todo_list(&client).await?;

    match output {
        OutputFormat::Json => crate::cli::output::print_json(&items),
        _ => {
            if items.is_empty() {
                println!("To-do list is empty.");
            }
            for item in &items {
                let done = item.completed.unwrap_or(false);
                println!(
                    "  [{}] {} {}",
                    item.id().unwrap_or("?"),
                    if done { "[x]" } else { "[ ]" },
                    item.value
                );
            }
        }
    }
    Ok(())
}

pub async fn cmd_add(text: &str, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    lists::add_todo_item(&client, text).await?;
    match output {
        OutputFormat::Json => println!("{{\"added\":\"{}\" }}", text),
        _ => println!("Added task: \"{}\"", text),
    }
    Ok(())
}

pub async fn cmd_complete(id: &str, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    lists::complete_item(&client, id, lists::task_type()).await?;
    match output {
        OutputFormat::Json => println!("{{\"completed\":\"{}\" }}", id),
        _ => println!("Marked task {} as complete.", id),
    }
    Ok(())
}

pub async fn cmd_remove(id: &str, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    lists::delete_item(&client, id).await?;
    match output {
        OutputFormat::Json => println!("{{\"removed\":\"{}\" }}", id),
        _ => println!("Removed task {}.", id),
    }
    Ok(())
}
