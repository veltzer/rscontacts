# merge-by-email

Find and merge contacts that share the same email address across different contacts.

## Usage

```bash
rscontacts merge-by-email
rscontacts merge-by-email --fix
rscontacts merge-by-email --fix --dry-run
```

## What It Does

Scans all contacts and identifies groups of contacts that share one or more email addresses. Emails are compared case-insensitively, so `John@Example.com` and `john@example.com` are treated as the same address.

Contacts are grouped using connected components: if contact A shares an email with contact B, and contact B shares a different email with contact C, all three are in the same merge group.

Without `--fix`, displays each group showing:
- All contacts in the group with full details
- Which email addresses are shared

## Fix Behavior

With `--fix`, for each group of N contacts you are prompted with:

```
[d1]elete [e1]dit [d2]elete [e2]dit ... [m]erge / [n]ext
```

- **d1, d2, ...** — Delete contact 1, 2, etc. (asks for confirmation). The contact is removed from the group and the prompt re-displays with remaining contacts.
- **e1, e2, ...** — Edit contact 1, 2, etc. using the interactive editor (same as `edit-contact`). Useful for cleaning up a contact before merging.
- **m** (merge) — Pick which contact to keep. All fields from the other contacts are merged into it, then the others are deleted.
- **n** (next) — Move on to the next group.

If you delete contacts until only one remains, the group is resolved automatically.

### Merge Details

When merging, the command:

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

- The merge is additive for multi-value fields (phones, emails, addresses, labels) and first-wins for single-value fields (organization, birthday, biography).
- The target contact is re-fetched before updating to ensure a fresh etag, avoiding conflicts.
- This is NOT included in `check-all` since it is a destructive operation requiring careful interactive review.
