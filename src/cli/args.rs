use clap::{Parser, Subcommand};

use super::output::OutputFormat;

#[derive(Parser, Debug)]
#[command(
    name = "alexa-cli",
    version,
    about = "Control Amazon Alexa from your terminal",
    long_about = "A CLI tool that wraps the alexa.amazon.com internal APIs, \
                  giving you full control over your Alexa devices, skills, \
                  smart home, lists, routines, and more."
)]
pub struct Cli {
    /// Output format
    #[arg(long, short = 'o', global = true, default_value = "text")]
    pub output: OutputFormat,

    /// Default device name (overrides config)
    #[arg(long, short = 'd', global = true)]
    pub device: Option<String>,

    /// Enable verbose/debug logging to stderr
    #[arg(long, short = 'v', global = true)]
    pub verbose: bool,

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
    /// Control media playback and volume
    Media {
        #[command(subcommand)]
        cmd: MediaCommands,
    },
    /// Make Alexa speak or broadcast announcements
    Speak {
        #[command(subcommand)]
        cmd: SpeakCommands,
    },
    /// Manage alarms
    Alarm {
        #[command(subcommand)]
        cmd: AlarmCommands,
    },
    /// Manage timers
    Timer {
        #[command(subcommand)]
        cmd: TimerCommands,
    },
    /// Manage reminders
    Reminder {
        #[command(subcommand)]
        cmd: ReminderCommands,
    },
    /// Manage your shopping list
    Shopping {
        #[command(subcommand)]
        cmd: ShoppingCommands,
    },
    /// Manage your to-do list
    Todo {
        #[command(subcommand)]
        cmd: TodoCommands,
    },
    /// List and run Alexa routines
    Routine {
        #[command(subcommand)]
        cmd: RoutineCommands,
    },
    /// Control smart home devices
    SmartHome {
        #[command(subcommand)]
        cmd: SmartHomeCommands,
    },
    /// Manage Do Not Disturb
    Dnd {
        #[command(subcommand)]
        cmd: DndCommands,
    },
    /// View Alexa activity history
    History {
        #[command(subcommand)]
        cmd: HistoryCommands,
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
    /// Show details for a specific device
    Get,
}

// ── Media ─────────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum MediaCommands {
    /// Resume playback
    Play,
    /// Pause playback
    Pause,
    /// Skip to next track
    Next,
    /// Go to previous track
    Prev,
    /// Set volume (0-100)
    Volume {
        /// Volume level (0-100)
        level: u8,
    },
    /// Show now-playing info
    Status,
    /// Play music by search query
    Music {
        /// Search query (e.g. "jazz" or "Taylor Swift")
        query: String,
        /// Music service (amazon-music, spotify, apple-music, pandora, tunein, iheartradio)
        #[arg(long, short = 's')]
        service: Option<String>,
    },
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

// ── Alarm ─────────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum AlarmCommands {
    /// List alarms
    List,
    /// Create a new alarm
    Create {
        /// Time in HH:MM (24-hour, e.g. 07:30)
        #[arg(long)]
        time: String,
        /// Optional label
        #[arg(long)]
        label: Option<String>,
    },
    /// Delete an alarm
    Delete {
        /// Alarm ID
        id: String,
    },
    /// Enable an alarm
    Enable {
        /// Alarm ID
        id: String,
    },
    /// Disable an alarm
    Disable {
        /// Alarm ID
        id: String,
    },
}

// ── Timer ─────────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum TimerCommands {
    /// List running timers
    List,
    /// Create a new timer
    Create {
        /// Duration (e.g. 1h30m, 90m, 45s)
        #[arg(long)]
        duration: String,
        /// Optional label
        #[arg(long)]
        label: Option<String>,
    },
    /// Cancel a timer
    Cancel {
        /// Timer ID
        id: String,
    },
    /// Pause a timer
    Pause {
        /// Timer ID
        id: String,
    },
    /// Resume a paused timer
    Resume {
        /// Timer ID
        id: String,
    },
}

// ── Reminder ──────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum ReminderCommands {
    /// List reminders
    List,
    /// Create a new reminder
    Create {
        /// Reminder text
        #[arg(long)]
        text: String,
        /// Trigger time in ISO 8601 format (e.g. 2025-12-25T09:00:00Z)
        #[arg(long)]
        time: String,
    },
    /// Delete a reminder
    Delete {
        /// Reminder ID
        id: String,
    },
}

// ── Shopping ──────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum ShoppingCommands {
    /// Show shopping list
    List,
    /// Add an item to the shopping list
    Add {
        /// Item name
        item: String,
    },
    /// Remove an item from the shopping list
    Remove {
        /// Item ID
        id: String,
    },
    /// Remove all items from the shopping list
    Clear,
}

// ── Todo ──────────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum TodoCommands {
    /// Show to-do list
    List,
    /// Add a task
    Add {
        /// Task description
        item: String,
    },
    /// Mark a task as complete
    Complete {
        /// Task ID
        id: String,
    },
    /// Remove a task
    Remove {
        /// Task ID
        id: String,
    },
}

// ── Routine ───────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum RoutineCommands {
    /// List all routines
    List,
    /// Run a routine by name
    Run {
        /// Routine name (substring match)
        name: String,
    },
}

// ── Smart Home ────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum SmartHomeCommands {
    /// List all smart home devices
    List,
    /// Turn a device on or off
    Power {
        /// Device name (substring match)
        device_name: String,
        /// on | off | toggle
        state: String,
    },
    /// Set brightness (0-100)
    Brightness {
        /// Device name
        device_name: String,
        /// Brightness level 0-100
        level: u8,
    },
    /// Set device color
    Color {
        /// Device name
        device_name: String,
        /// Color name (e.g. "red", "warm white")
        color: String,
    },
    /// Set thermostat target temperature
    Thermostat {
        /// Device name
        device_name: String,
        /// Target temperature
        temp: f64,
        /// Temperature unit (F or C)
        #[arg(long, default_value = "F")]
        unit: String,
    },
    /// Lock or unlock a smart lock
    Lock {
        /// Device name
        device_name: String,
        /// lock | unlock
        state: String,
    },
}

// ── DND ───────────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum DndCommands {
    /// Show Do Not Disturb status
    Status,
    /// Enable Do Not Disturb
    Enable,
    /// Disable Do Not Disturb
    Disable,
}

// ── History ───────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum HistoryCommands {
    /// Show recent Alexa activity
    List {
        /// Number of entries to show
        #[arg(long, default_value = "20")]
        limit: usize,
    },
}
