#![allow(dead_code, unused_imports, unused_variables, deprecated)]
mod api;
mod auth;
mod cli;
mod commands;
mod config;

use anyhow::Result;
use clap::Parser;

use cli::args::{
    AlarmCommands, AuthCommands, Cli, Commands, DevicesCommands, DndCommands, HistoryCommands,
    MediaCommands, ReminderCommands, RoutineCommands, ShoppingCommands, SmartHomeCommands,
    SpeakCommands, TimerCommands, TodoCommands,
};

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

    if cli.verbose {
        eprintln!("[verbose] output={:?} device={:?}", out, dev);
    }

    match cli.command {
        // ── Auth ─────────────────────────────────────────────────────────
        Commands::Auth { cmd } => match cmd {
            AuthCommands::Login { email } => {
                commands::auth::cmd_login(&email, out).await?;
            }
            AuthCommands::ImportCookies => {
                commands::auth::cmd_import_cookies(out).await?;
            }
            AuthCommands::Logout => {
                commands::auth::cmd_logout(out).await?;
            }
            AuthCommands::Status => {
                commands::auth::cmd_status(out).await?;
            }
        },

        // ── Devices ──────────────────────────────────────────────────────
        Commands::Devices { cmd } => match cmd {
            DevicesCommands::List => {
                commands::devices::cmd_list(out, dev).await?;
            }
            DevicesCommands::Get => {
                let name = dev.unwrap_or_default();
                commands::devices::cmd_get(name, out).await?;
            }
        },

        // ── Media ─────────────────────────────────────────────────────────
        Commands::Media { cmd } => match cmd {
            MediaCommands::Play => commands::media::cmd_play(dev, out).await?,
            MediaCommands::Pause => commands::media::cmd_pause(dev, out).await?,
            MediaCommands::Next => commands::media::cmd_next(dev, out).await?,
            MediaCommands::Prev => commands::media::cmd_prev(dev, out).await?,
            MediaCommands::Volume { level } => {
                commands::media::cmd_volume(level, dev, out).await?;
            }
            MediaCommands::Status => commands::media::cmd_status(dev, out).await?,
            MediaCommands::Music { query, service } => {
                commands::media::cmd_music(&query, service.as_deref(), dev, out).await?;
            }
        },

        // ── Speak ─────────────────────────────────────────────────────────
        Commands::Speak { cmd } => match cmd {
            SpeakCommands::Say { text } => {
                commands::speak::cmd_say(&text, dev, out).await?;
            }
            SpeakCommands::Announce { text, devices } => {
                commands::speak::cmd_announce(&text, devices.as_deref(), out).await?;
            }
        },

        // ── Ask ──────────────────────────────────────────────────────────
        Commands::Ask { text } => {
            commands::ask::cmd_ask(&text, dev, out).await?;
        },

        // ── Alarm ─────────────────────────────────────────────────────────
        Commands::Alarm { cmd } => match cmd {
            AlarmCommands::List => commands::alarm::cmd_list(dev, out).await?,
            AlarmCommands::Create { time, label } => {
                commands::alarm::cmd_create(&time, label.as_deref(), dev, out).await?;
            }
            AlarmCommands::Delete { id } => commands::alarm::cmd_delete(&id, out).await?,
            AlarmCommands::Enable { id } => commands::alarm::cmd_enable(&id, out).await?,
            AlarmCommands::Disable { id } => commands::alarm::cmd_disable(&id, out).await?,
        },

        // ── Timer ─────────────────────────────────────────────────────────
        Commands::Timer { cmd } => match cmd {
            TimerCommands::List => commands::timer::cmd_list(dev, out).await?,
            TimerCommands::Create { duration, label } => {
                commands::timer::cmd_create(&duration, label.as_deref(), dev, out).await?;
            }
            TimerCommands::Cancel { id } => commands::timer::cmd_cancel(&id, out).await?,
            TimerCommands::Pause { id } => commands::timer::cmd_pause(&id, out).await?,
            TimerCommands::Resume { id } => commands::timer::cmd_resume(&id, out).await?,
        },

        // ── Reminder ──────────────────────────────────────────────────────
        Commands::Reminder { cmd } => match cmd {
            ReminderCommands::List => commands::reminder::cmd_list(dev, out).await?,
            ReminderCommands::Create { text, time } => {
                commands::reminder::cmd_create(&text, &time, dev, out).await?;
            }
            ReminderCommands::Delete { id } => commands::reminder::cmd_delete(&id, out).await?,
        },

        // ── Shopping ──────────────────────────────────────────────────────
        Commands::Shopping { cmd } => match cmd {
            ShoppingCommands::List => commands::shopping::cmd_list(out).await?,
            ShoppingCommands::Add { item } => commands::shopping::cmd_add(&item, out).await?,
            ShoppingCommands::Remove { id } => commands::shopping::cmd_remove(&id, out).await?,
            ShoppingCommands::Clear => commands::shopping::cmd_clear(out).await?,
        },

        // ── Todo ──────────────────────────────────────────────────────────
        Commands::Todo { cmd } => match cmd {
            TodoCommands::List => commands::todo::cmd_list(out).await?,
            TodoCommands::Add { item } => commands::todo::cmd_add(&item, out).await?,
            TodoCommands::Complete { id } => commands::todo::cmd_complete(&id, out).await?,
            TodoCommands::Remove { id } => commands::todo::cmd_remove(&id, out).await?,
        },

        // ── Routine ───────────────────────────────────────────────────────
        Commands::Routine { cmd } => match cmd {
            RoutineCommands::List => commands::routines::cmd_list(out).await?,
            RoutineCommands::Run { name } => commands::routines::cmd_run(&name, out).await?,
        },

        // ── Smart Home ────────────────────────────────────────────────────
        Commands::SmartHome { cmd } => match cmd {
            SmartHomeCommands::List => commands::smart_home::cmd_list(out).await?,
            SmartHomeCommands::Power { device_name, state } => {
                commands::smart_home::cmd_power(&device_name, &state, out).await?;
            }
            SmartHomeCommands::Brightness { device_name, level } => {
                commands::smart_home::cmd_brightness(&device_name, level, out).await?;
            }
            SmartHomeCommands::Color { device_name, color } => {
                commands::smart_home::cmd_color(&device_name, &color, out).await?;
            }
            SmartHomeCommands::Thermostat {
                device_name,
                temp,
                unit,
            } => {
                commands::smart_home::cmd_thermostat(&device_name, temp, &unit, out).await?;
            }
            SmartHomeCommands::Lock { device_name, state } => {
                commands::smart_home::cmd_lock(&device_name, &state, out).await?;
            }
        },

        // ── DND ───────────────────────────────────────────────────────────
        Commands::Dnd { cmd } => match cmd {
            DndCommands::Status => commands::dnd::cmd_status(dev, out).await?,
            DndCommands::Enable => commands::dnd::cmd_set(true, dev, out).await?,
            DndCommands::Disable => commands::dnd::cmd_set(false, dev, out).await?,
        },

        // ── History ───────────────────────────────────────────────────────
        Commands::History { cmd } => match cmd {
            HistoryCommands::List { limit } => commands::history::cmd_list(limit, out).await?,
        },
    }

    Ok(())
}
