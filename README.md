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
cp target/release/alexa-cli ~/.local/bin/
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

# Ask Alexa a question (device speaks the answer)
alexa-cli ask "what's the weather" --device "Living Room Echo"

# Control smart home
alexa-cli smart-home power "Desk Lamp" off

# Play music
alexa-cli media music "jazz" --device "Living Room Echo"
```

Run `alexa-cli --help` for all commands.
