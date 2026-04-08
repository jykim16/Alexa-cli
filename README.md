# alexa-cli

A Rust CLI for controlling Amazon Alexa from the terminal. Wraps the reverse-engineered `alexa.amazon.com` internal APIs to give full access to your Alexa devices, smart home, lists, routines, and more — including machine-readable JSON output for use with Claude Code, Codex, or any other AI agent.

## Installation

### Build from source

```bash
# Prerequisites: Rust 1.78+, libdbus-1-dev (Linux), pkg-config, libssl-dev
cargo build --release
cp target/release/alexa-cli ~/.local/bin/
```

### Docker (no Rust required)

```bash
docker build -f docker/Dockerfile -t alexa-cli .
docker run --rm alexa-cli devices list --output json
```

---

## Authentication

Amazon uses a cookie-based session. Credentials are stored securely:
- **Password** — keyring (macOS Keychain / GNOME Keyring / Windows Credential Store)
- **Cookies** — `~/.config/alexa-cli/cookies.json` (mode 0600), or keyring if `ALEXA_SKIP_KEYRING` is unset
- **CSRF token** — fetched fresh from `/api/bootstrap` each session

### Login

```bash
alexa-cli auth login --email you@example.com
# Prompts for password (not echoed). Handles 2FA (OTP / Amazon app).
```

### Check status

```bash
alexa-cli auth status
```

### Logout

```bash
alexa-cli auth logout
```

---

## Configuration

Config file: `~/.config/alexa-cli/config.toml`

```toml
email         = "you@example.com"
base_url      = "https://alexa.amazon.com"   # US (default)
# base_url    = "https://alexa.amazon.co.uk" # EU
# base_url    = "https://layla.amazon.de"    # DE
default_device = "Living Room Echo"
locale        = "en-US"
```

Global flags available on every command:

| Flag | Short | Description |
|---|---|---|
| `--output text\|json\|table` | `-o` | Output format (default: `text`) |
| `--device <name>` | `-d` | Target device (substring match; overrides config) |
| `--verbose` | `-v` | Debug output to stderr |

---

## Commands

### Devices

```bash
alexa-cli devices list                         # list all registered devices
alexa-cli devices list --output json           # machine-readable
alexa-cli devices get --device "Kitchen"       # details for one device
```

### Media

```bash
alexa-cli media play    [--device <name>]
alexa-cli media pause   [--device <name>]
alexa-cli media next    [--device <name>]
alexa-cli media prev    [--device <name>]
alexa-cli media volume 50 [--device <name>]    # 0-100
alexa-cli media status  [--device <name>]      # now-playing info
alexa-cli media music "jazz" [--device <name>] [--service amazon-music|spotify|apple-music|pandora|tunein|iheartradio]
```

### Speak / Announce

```bash
alexa-cli speak say "Build complete" --device "Office Echo"
alexa-cli speak announce "Dinner is ready" --devices "Kitchen Echo,Living Room"
```

### Alarms

```bash
alexa-cli alarm list   [--device <name>]
alexa-cli alarm create --time 07:30 [--label "Wake up"] [--device <name>]
alexa-cli alarm delete <id>
alexa-cli alarm enable  <id>
alexa-cli alarm disable <id>
```

### Timers

```bash
alexa-cli timer list   [--device <name>]
alexa-cli timer create --duration 1h30m [--label "Pasta"] [--device <name>]
alexa-cli timer create --duration 90m
alexa-cli timer create --duration 45s
alexa-cli timer cancel <id>
alexa-cli timer pause  <id>
alexa-cli timer resume <id>
```

Duration format: `1h30m`, `90m`, `45s`, or bare seconds (`90`).

### Reminders

```bash
alexa-cli reminder list
alexa-cli reminder create --text "Take medication" --time 2026-06-01T08:00:00Z
alexa-cli reminder delete <id>
```

### Shopping List

```bash
alexa-cli shopping list
alexa-cli shopping add "oat milk"
alexa-cli shopping remove <id>
alexa-cli shopping clear
```

### To-Do List

```bash
alexa-cli todo list
alexa-cli todo add "Review PR #42"
alexa-cli todo complete <id>
alexa-cli todo remove <id>
```

### Routines

```bash
alexa-cli routine list
alexa-cli routine run "Good Morning"   # substring match
```

### Smart Home

```bash
alexa-cli smart-home list                          # all smart home devices
alexa-cli smart-home power "Desk Lamp" on
alexa-cli smart-home power "Desk Lamp" off
alexa-cli smart-home power "Desk Lamp" toggle
alexa-cli smart-home brightness "Bedroom Light" 40  # 0-100
alexa-cli smart-home color "Living Room Light" "warm white"
alexa-cli smart-home thermostat "Nest" 72 --unit F
alexa-cli smart-home lock "Front Door" lock
alexa-cli smart-home lock "Front Door" unlock
```

### Do Not Disturb

```bash
alexa-cli dnd status  [--device <name>]
alexa-cli dnd enable  [--device <name>]
alexa-cli dnd disable [--device <name>]
```

### History

```bash
alexa-cli history list              # last 20 interactions
alexa-cli history list --limit 50
alexa-cli history list --output json
```

---

## AI Agent Integration (Claude Code / Codex)

All commands support `--output json` for machine-readable output:

```bash
# Discover devices
alexa-cli devices list --output json

# Notify on build success
alexa-cli speak say "Tests passed, build succeeded" --device "Office Echo"

# Add items from a script
alexa-cli shopping add "coffee beans"

# Check what Alexa heard recently
alexa-cli history list --limit 5 --output json

# Control smart home from a script
alexa-cli smart-home power "Desk Lamp" on --output json
```

Errors print to stderr; exit code is non-zero on failure. JSON output is always a valid JSON value (array, object, or string).

---

## Environment Variables

| Variable | Description |
|---|---|
| `ALEXA_BASE_URL` | Override API base URL (used in tests / CI) |
| `ALEXA_SKIP_KEYRING` | Set to `1` to disable keyring; fall back to file-based cookie store |

---

## Testing

### Unit tests

141 unit tests covering all API modules, auth, config, and output formatting. Run with:

```bash
cargo test
```

Tests use [mockito](https://github.com/lipanski/mockito) for in-process HTTP stubs — no network or Docker required.

### Integration tests

42 end-to-end tests that invoke the compiled binary against a [WireMock](https://wiremock.org/) container. Each test group covers a full command round-trip (list, create, delete, etc.).

**Run locally with Docker:**

```bash
docker compose -f docker/docker-compose.test.yml up --build --abort-on-container-exit
```

The compose file starts WireMock first (waits for its health check), then runs the test binary with `ALEXA_BASE_URL` pointed at the mock server.

**Run a single integration test file:**

```bash
# Requires Docker — start WireMock manually:
docker run -d -p 8888:8080 wiremock/wiremock:3.9.1

ALEXA_BASE_URL=http://localhost:8888 \
  cargo test --test integration_devices -- --include-ignored
```

Integration test files (all in `tests/`):

| File | Tests |
|---|---|
| `integration_devices.rs` | list, get, auth failure |
| `integration_media.rs` | status, play, pause, volume, music |
| `integration_alarms.rs` | list, create, delete, enable, disable |
| `integration_timers.rs` | list, create, cancel, pause, resume |
| `integration_reminders.rs` | list, create, delete |
| `integration_lists.rs` | shopping & todo — list, add, remove, complete, clear |
| `integration_speak.rs` | say, announce |
| `integration_routines.rs` | list, run, not-found error |
| `integration_smart_home.rs` | list, power, brightness, thermostat |
| `integration_dnd.rs` | status, enable, disable |
| `integration_history.rs` | list, limit, empty response |

### Linting

```bash
cargo fmt --check
cargo clippy -- -D warnings
```

---

## Project Structure

```
alexa-cli/
├── src/
│   ├── main.rs              # CLI entry point and dispatch
│   ├── cli/
│   │   ├── args.rs          # All clap subcommand definitions
│   │   └── output.rs        # --output text|json|table formatting
│   ├── auth/
│   │   ├── login.rs         # Amazon login flow (form scraping, MFA)
│   │   ├── cookie_store.rs  # Persistent cookie jar (keyring / file)
│   │   └── csrf.rs          # Bootstrap CSRF token fetch
│   ├── config/
│   │   └── settings.rs      # Config struct + TOML load/save
│   ├── api/
│   │   ├── mod.rs           # ApiClient (reqwest + cookies + CSRF headers)
│   │   ├── devices.rs
│   │   ├── media.rs
│   │   ├── behaviors.rs     # TTS, music, routines (behaviors API)
│   │   ├── alarms.rs
│   │   ├── timers.rs
│   │   ├── reminders.rs
│   │   ├── lists.rs         # Shopping + to-do
│   │   ├── automations.rs
│   │   ├── smart_home.rs    # Phoenix API
│   │   ├── dnd.rs
│   │   ├── history.rs
│   │   └── errors.rs
│   └── commands/            # One module per command group
├── tests/
│   ├── common/mod.rs        # WireMock container helper + run_binary()
│   └── integration_*.rs     # Per-command integration tests
└── docker/
    ├── Dockerfile           # Production multi-stage image
    ├── Dockerfile.test      # Test runner image
    └── docker-compose.test.yml
```

---

## Caveats

- These are **unofficial, reverse-engineered APIs**. Amazon can change or remove them without notice.
- Intended for **personal use**. Automated access may violate Amazon's Terms of Service for commercial purposes.
- Amazon may require **CAPTCHA** on first login from a new IP. If this occurs, import cookies from your browser manually.
- Cookies expire in ~14 days; the tool will prompt you to re-authenticate.
- **Regions**: US (`alexa.amazon.com`), EU (`alexa.amazon.co.uk` / `layla.amazon.de`), configured in `config.toml`.
