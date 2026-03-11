# check-phone-country-label

Ensure every contact has the correct `country:<Name>` labels matching their phone number country codes — and no stale country labels for countries where they have no phone numbers.

## Usage

```bash
rscontacts check-phone-country-label
rscontacts check-phone-country-label --fix
rscontacts check-phone-country-label --fix --dry-run
```

## What It Checks

Two things:

1. **Missing country labels**: If a contact has a phone number with country code +972 (Israel), they should have a `country:Israel` label. If the label is missing, it is flagged.
2. **Stale country labels**: If a contact has a `country:Russia` label but none of their phone numbers have a +7 country code, the label is flagged.

## Fix Behavior

With `--fix`:

- **Missing labels**: The `country:<Name>` label is created (if it doesn't exist) and automatically assigned to the contact.
- **Stale labels**: The `country:<Name>` label is automatically removed from the contact.

With `--fix --dry-run`, shows what would be changed without modifying anything.

## Supported Countries

All ITU-T E.164 country codes are recognized, including:

| Code | Country | Code | Country |
|------|---------|------|---------|
| +1   | USA     | +44  | UK      |
| +7   | Russia  | +49  | Germany |
| +33  | France  | +86  | China   |
| +34  | Spain   | +91  | India   |
| +39  | Italy   | +380 | Ukraine |
| +41  | Switzerland | +972 | Israel |
| +971 | UAE     | +966 | Saudi Arabia |

And many more — see the full mapping in `src/helpers.rs`.

## Notes

- Only phone numbers with a recognized country code prefix are considered. Phones without country codes are ignored (fix those first with [check-phone-countrycode](check-phone-countrycode.md)).
- Only `country:` labels that match a recognized country name are considered for removal. Custom labels that happen to start with `country:` but use a different name will not be touched.
- This check is included in [check-all](check-all.md).
