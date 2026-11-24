# Authentication

rscontacts uses OAuth2 to access the Google People API on your behalf.

## How It Works

1. You provide OAuth2 client credentials (a JSON file from Google Cloud Console)
2. On first use, rscontacts opens your browser to get consent
3. The access token is cached locally for future requests

## Where the secrets live

rscontacts looks in [pass](https://www.passwordstore.org/) first and falls back
to plain files when `pass` is not installed or the entry does not exist.

| What | pass entry (default) | Fallback file |
|------|----------------------|---------------|
| OAuth2 client (the JSON from Google Cloud Console) | `gcp/rscontacts/oauth-client` | `~/.config/rscontacts/credentials.json` |
| Token cache (access/refresh tokens) | `gcp/rscontacts/token` | `~/.config/rscontacts/token_cache.json` |

Users without a pass store need do nothing: the files work exactly as before.

The entries can be renamed in `config.toml` (see [Configuration](configuration.md)).

To use pass, store the content of the two files in the entries:

```bash
pass insert -m gcp/rscontacts/oauth-client < credentials.json
pass insert -m gcp/rscontacts/token < token_cache.json
```

The pass token entry has the same JSON layout as `token_cache.json`.

Rules in pass mode:

- The token entry is used when it exists, or when the OAuth client came from
  pass. In that case `rscontacts auth` creates the token entry and never
  writes a plaintext token file.
- The pass store is usually a git repository, so the token entry is not
  rewritten every time the one-hour access token is refreshed. Refreshed
  access tokens are kept in memory only. The entry is written when you
  authorize (`auth`, `auth --force`) or when Google issues a new refresh token.
- `auth --force` removes the token entry (`pass rm`) before authorizing again.
- Secret values are never printed; messages name the entry only.

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
