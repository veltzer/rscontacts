# Authentication

rscontacts uses OAuth2 to access the Google People API on your behalf.

## How It Works

1. You provide OAuth2 client credentials (a JSON file from Google Cloud Console)
2. On first use, rscontacts opens your browser to get consent
3. The access token is cached locally for future requests

## Files

| File | Location | Purpose |
|------|----------|---------|
| Credentials | `~/.config/rscontacts/credentials.json` | OAuth2 client ID and secret |
| Token cache | `~/.config/rscontacts/token_cache.json` | Cached access/refresh tokens |

## Commands

Authenticate (opens browser):

```bash
rscontacts auth
```

Authenticate without browser (prints URL):

```bash
rscontacts auth --no-browser
```

Force re-authentication (removes cached token first):

```bash
rscontacts auth --force
```

## Scopes

rscontacts requests the `https://www.googleapis.com/auth/contacts` scope, which provides full read/write access to your Google Contacts.
