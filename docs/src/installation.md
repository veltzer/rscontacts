# Installation

## Prerequisites

- Rust toolchain (edition 2024)
- Google Cloud project with People API enabled
- OAuth2 credentials (Desktop application type)

## Building from Source

```bash
git clone https://github.com/veltzer/rscontacts.git
cd rscontacts
cargo build --release
```

The binary will be at `target/release/rscontacts`.

## Google Cloud Setup

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project (or use an existing one)
3. Enable the **People API** under APIs & Services
4. Create OAuth2 credentials:
   - Go to APIs & Services > Credentials
   - Click "Create Credentials" > "OAuth client ID"
   - Choose "Desktop application" as the application type
   - Download the JSON file
5. Place the credentials file at `~/.config/rscontacts/credentials.json`
