use anyhow::Result;
use std::sync::Arc;

use crate::api::{lists, ApiClient};
use crate::cli::OutputFormat;
use crate::config::Settings;

pub async fn cmd_list(output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    let items = lists::get_shopping_list(&client).await?;

    match output {
        OutputFormat::Json => crate::cli::output::print_json(&items),
        _ => {
            if items.is_empty() {
                println!("Shopping list is empty.");
            }
            for item in &items {
                println!("  [{}] {}", item.id().unwrap_or("?"), item.value);
            }
        }
    }
    Ok(())
}

pub async fn cmd_add(text: &str, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    lists::add_shopping_item(&client, text).await?;
    match output {
        OutputFormat::Json => println!("{{\"added\":\"{}\" }}", text),
        _ => println!("Added \"{}\" to shopping list.", text),
    }
    Ok(())
}

pub async fn cmd_remove(id: &str, output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    lists::delete_item(&client, id).await?;
    match output {
        OutputFormat::Json => println!("{{\"removed\":\"{}\" }}", id),
        _ => println!("Removed item {}.", id),
    }
    Ok(())
}

pub async fn cmd_clear(output: OutputFormat) -> Result<()> {
    let settings = Arc::new(Settings::load()?);
    let client = ApiClient::new(settings).await?;
    let items = lists::get_shopping_list(&client).await?;
    let count = items.len();
    for item in &items {
        if let Some(id) = item.id() {
            lists::delete_item(&client, id).await?;
        }
    }
    match output {
        OutputFormat::Json => println!("{{\"cleared\":{} }}", count),
        _ => println!("Cleared {} items from shopping list.", count),
    }
    Ok(())
}
