# Getting Started

## First-Time Setup

After installing rscontacts and placing your OAuth2 credentials, authenticate:

```bash
rscontacts auth
```

This opens your browser for Google OAuth2 consent. The token is cached at `~/.config/rscontacts/token_cache.json` for future use.

If you're on a headless machine:

```bash
rscontacts auth --no-browser
```

This prints the auth URL instead of opening a browser.

## Basic Usage

List all contacts:

```bash
rscontacts list
```

Include email addresses:

```bash
rscontacts list --emails
```

Show phone labels (mobile/home/work):

```bash
rscontacts list --labels
```

## Running Checks

Run all checks at once:

```bash
rscontacts check-all
```

Get a summary of issues:

```bash
rscontacts check-all --stats
```

Fix issues interactively:

```bash
rscontacts check-all --fix
```

Preview what would change without modifying anything:

```bash
rscontacts check-all --fix --dry-run
```

## Inspecting a Contact

Show all details for a specific contact:

```bash
rscontacts show-contact "John"
```

This does a case-insensitive substring search and displays all available fields.
