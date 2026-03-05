# check-email-caps

Find contacts with uppercase letters in their email addresses.

Email addresses are case-insensitive in practice, so `Mark@Gmail.com` should be `mark@gmail.com`.

## Usage

```bash
rscontacts check-email-caps
rscontacts check-email-caps --fix
rscontacts check-email-caps --fix --dry-run
```

## Fix Behavior

With `--fix`, automatically lowercases all email addresses for each affected contact. No interactive prompt since the fix is unambiguous.
