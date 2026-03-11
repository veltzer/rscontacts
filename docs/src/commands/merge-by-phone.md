# merge-by-phone

Find and merge contacts that share the same phone number across different contacts.

## Usage

```bash
rscontacts merge-by-phone
rscontacts merge-by-phone --fix
rscontacts merge-by-phone --fix --dry-run
```

## What It Does

Scans all contacts and identifies groups of contacts that share one or more phone numbers. Phone numbers are normalized (digits only, stripping international prefix `00`) before comparison, so `+972-501234567` and `00972501234567` are treated as the same number.

Contacts are grouped using connected components: if contact A shares a phone with contact B, and contact B shares a different phone with contact C, all three are in the same merge group.

Without `--fix`, displays each group showing:
- All contacts in the group with full details
- Which phone numbers are shared

## Fix Behavior

With `--fix`, for each group you are prompted:

```
[m]erge / [s]kip:
```

If you choose **merge**, you pick which contact to keep. The command then:

1. **Merges phone numbers** — adds any phones from source contacts not already on the target (compared by normalized digits)
2. **Merges email addresses** — adds any emails from source contacts not already on the target (compared case-insensitively)
3. **Merges addresses** — adds any addresses not already on the target (compared by formatted value)
4. **Merges organization** — copies from source only if the target has no organization
5. **Merges birthdays** — copies from source only if the target has no birthday
6. **Merges biographies** — copies from source only if the target has no biography
7. **Copies labels** — adds all contact group memberships from source contacts to the target
8. **Deletes source contacts** — removes the merged-away contacts

With `--fix --dry-run`, shows what would happen without making changes.

## Notes

- Only "fixable" phone numbers are considered (see `is_fixable_phone` — star codes, short codes, and alphanumeric entries are skipped).
- The merge is additive for multi-value fields (phones, emails, addresses, labels) and first-wins for single-value fields (organization, birthday, biography).
- The target contact is re-fetched before updating to ensure a fresh etag, avoiding conflicts.
- This is NOT included in `check-all` since it is a destructive operation requiring careful interactive review.
