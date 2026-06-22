# alexa-cli

Control Amazon Alexa from your terminal.

## Setup

### Prerequisites

**Two-Factor Authentication (2FA)** must be enabled on your Amazon account.
Enable it at: https://www.amazon.com/a/settings/approval

You'll need the OTP code from your authenticator app each time you log in.

### Build

```bash
cargo build --release
cargo install --path .
```

## Authentication

```bash
alexa-cli auth login --email you@example.com
```

You'll be prompted for your Amazon password and OTP code. The session lasts ~14 days.

## Usage

```bash
# List devices
alexa-cli devices list

# Make Alexa speak
alexa-cli speak say "hello" --device "Living Room Echo"

# Ask Alexa anything (just like voice)
alexa-cli ask "what's the weather" --device "Living Room Echo"

# Set timers, alarms, reminders
alexa-cli ask "set a 5 minute timer" -d "Kitchen Echo"
alexa-cli ask "set an alarm for 7am" -d "Bedroom Echo"
alexa-cli ask "remind me to call mom at 3pm" -d "Office Echo"

# Control smart home
alexa-cli ask "turn on the living room lights" -d "Living Room Echo"
alexa-cli ask "set thermostat to 72" -d "Living Room Echo"

# Shopping and to-do lists
alexa-cli ask "add milk to my shopping list" -d "Kitchen Echo"
alexa-cli ask "what's on my to-do list" -d "Office Echo"

# Play music
alexa-cli ask "play jazz music" -d "Living Room Echo"

# Broadcast to all devices
alexa-cli speak announce "Dinner is ready"
```

Run `alexa-cli --help` for all commands.
