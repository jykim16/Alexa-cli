use clap::{Parser, Subcommand};

use super::output::OutputFormat;

#[derive(Parser, Debug)]
#[command(
    name = "alexa-cli",
    version,
    about = "Control Amazon Alexa from your terminal",
    long_about = "A CLI tool to control your Alexa devices.\n\n\
                  For commands like timers, alarms, shopping lists, and smart home control,\n\
                  use the 'ask' command: alexa-cli ask \"set a 5 minute timer\""
)]
pub struct Cli {
    /// Output format
    #[arg(long, short = 'o', global = true, default_value = "text")]
    pub output: OutputFormat,

    /// Default device name (overrides config)
    #[arg(long, short = 'd', global = true)]
    pub device: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Authenticate with your Amazon account
    Auth {
        #[command(subcommand)]
        cmd: AuthCommands,
    },
    /// List and inspect your Alexa devices
    Devices {
        #[command(subcommand)]
        cmd: DevicesCommands,
    },
    /// Make Alexa speak
    Speak {
        #[command(subcommand)]
        cmd: SpeakCommands,
    },
    /// Ask Alexa a question or give a command (device speaks the answer)
    Ask {
        /// Question or command to send to Alexa (e.g. "what's the weather", "set a 5 minute timer")
        text: String,
    },
}

// ── Auth ─────────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum AuthCommands {
    /// Log in with your Amazon account
    Login {
        /// Amazon account email
        #[arg(long)]
        email: String,
    },
    /// Clear stored session cookies
    Logout,
    /// Show current authentication status
    Status,
}

// ── Devices ──────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum DevicesCommands {
    /// List all registered Alexa devices
    List,
}

// ── Speak ─────────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum SpeakCommands {
    /// Make one device say something (TTS)
    Say {
        /// Text to speak
        text: String,
    },
    /// Broadcast an announcement to all (or specified) devices
    Announce {
        /// Text to announce
        text: String,
        /// Comma-separated device names (omit for all)
        #[arg(long)]
        devices: Option<String>,
    },
}
