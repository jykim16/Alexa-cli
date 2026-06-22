mod api;
mod auth;
mod cli;
mod commands;
mod config;

use anyhow::Result;
use clap::Parser;

use cli::args::{AuthCommands, Cli, Commands, DevicesCommands, SpeakCommands};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    let out = cli.output;
    let dev = cli.device.as_deref();

    match cli.command {
        Commands::Auth { cmd } => match cmd {
            AuthCommands::Login { email } => {
                commands::auth::cmd_login(&email, out).await?;
            }
            AuthCommands::Logout => {
                commands::auth::cmd_logout(out).await?;
            }
            AuthCommands::Status => {
                commands::auth::cmd_status(out).await?;
            }
        },

        Commands::Devices { cmd } => match cmd {
            DevicesCommands::List => {
                commands::devices::cmd_list(out, dev).await?;
            }
        },

        Commands::Speak { cmd } => match cmd {
            SpeakCommands::Say { text } => {
                commands::speak::cmd_say(&text, dev, out).await?;
            }
            SpeakCommands::Announce { text, devices } => {
                commands::speak::cmd_announce(&text, devices.as_deref(), out).await?;
            }
        },

        Commands::Ask { text } => {
            commands::ask::cmd_ask(&text, dev, out).await?;
        }
    }

    Ok(())
}
