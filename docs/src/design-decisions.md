# Design Decisions

## Country Code Detection: Hardcoded Table vs `phonenumber` Crate

The `check-phone-format` command needs to split phone numbers into country code and local number (e.g., `+79268335991` → `+7-9268335991`). Country codes vary in length: 1 digit (`+1` US, `+7` Russia), 2 digits (`+44` UK), or 3 digits (`+972` Israel).

### Approach Chosen: Hardcoded ITU Country Code Table

rscontacts embeds a static list of ~190 ITU country codes and uses longest-prefix matching (3 digits, then 2, then 1) to detect the country code boundary.

### Alternative Considered: `phonenumber` Crate

The [phonenumber](https://crates.io/crates/phonenumber) crate is a Rust port of Google's libphonenumber. It provides:

- Phone number validation (correct digit count per country)
- Multiple formatting options (E.164, international, national)
- Region detection from number

### Why We Chose the Hardcoded Table

- **We only need one thing**: splitting the country code from the local number. The crate would be overkill.
- **Minimal dependency**: the crate bundles ~2MB of per-country metadata for phone number rules we don't use.
- **Simplicity**: a `const` array with prefix matching is trivial to understand and maintain.

### When to Reconsider

If rscontacts ever needs to *validate* that a phone number has the correct number of digits for its country, or needs national formatting, the `phonenumber` crate would be the right choice.
