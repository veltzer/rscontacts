# Future Ideas

This page collects ideas for future development of rscontacts.

## Web Application

Turn rscontacts into a web application that provides a browser-based UI for auditing and fixing Google Contacts. This would make the tool accessible to non-technical users who are not comfortable with the command line. The web app could:

- Display all check results in a dashboard view with counts and summaries.
- Allow users to review and approve fixes one by one or in bulk.
- Authenticate via OAuth in the browser instead of requiring a local credentials file.
- Provide real-time progress indicators for long-running checks.
- Use a Rust web framework (e.g., Axum or Actix-web) for the backend, reusing the existing check and fix logic from the CLI.
- Use a lightweight frontend (e.g., HTMX, or a simple SPA with a framework like Leptos or Yew for a full-Rust stack).

## Distinguish Companies from Individuals

Currently there is no way to tell whether a contact represents a company or an individual person. Knowing this distinction would allow checks to apply different rules (e.g., company contacts would not need a given/family name split, and name format checks like capitalization or suffix numbering would not apply).

One approach: maintain a local file listing known company names. A check could then compare each contact's organization field (or display name) against this list and flag ambiguous entries. Contacts matching a known company name could be tagged or moved into a dedicated label/group.

Another approach: use a heuristic — contacts that have an organization name but no given/family name are likely companies. This could be combined with the company-names file for better accuracy.

## Better Contact Organization

Based on analysis of typical contact databases, here are ideas for improving contact organization:

### Merge Duplicate Contacts

Contacts that share multiple phone numbers are almost certainly duplicates and should be merged. A check could detect contacts sharing phone numbers and prompt the user to merge them.

### Enrich Organization Field

Most contacts lack an organization field. Contacts with corporate email domains (e.g., `@johnbryce.co.il`, `@sqlink.com`) could have their organization auto-populated from the domain name.

### Group by Employer/Domain

Create labels automatically based on email domains for professional contacts. For example, all contacts with `@example.com` emails could be auto-labeled `company:Example`.

### Group by Country (implemented)

Contacts can be auto-labeled by country based on their phone number country codes (e.g., `country:Israel` for +972, `country:Ukraine` for +380). See [check-phone-country-label](commands/check-phone-country-label.md). This works both ways: missing labels are added, and stale labels (where no phone number matches) are removed.

### Clean Unreachable Contacts

Contacts with a name but no phone number and no email are unreachable and should be enriched or removed.

### Add Birthdays

Typically very few contacts have birthdays set. Adding birthdays to important contacts enables birthday reminders.

### Break Up Large Groups

If a single label contains the vast majority of contacts, it is too broad to be useful. Consider splitting it by relationship type (family, friends, colleagues, service providers, etc.).

### Add Missing Emails

Many contacts have only a phone number. For contacts with an organization, it may be possible to infer or look up work email addresses.

## Performance Improvements

### Set page_size on API fetches

`fetch_all_contacts` and `fetch_all_contact_groups` don't set `page_size`, using the API default (which may be small). Setting it to the maximum (1000 for contacts, 200 for groups) would reduce the number of round trips to Google's API.

## Robustness Improvements

### Improve label creation error messages

When label creation fails mid-fix (e.g., in `check-contact-no-label --fix` or `check-phone-country-label --fix`), the error message doesn't indicate which contact and label were involved. Including the contact name and target label in the error would make debugging easier.

### Proactive stale token detection

When the OAuth token is expired or corrupted, API calls currently wait for the 30-second request timeout before failing. The tool could proactively check token freshness (e.g., by inspecting the `expiry_date` field in `token_cache.json`) and suggest `rscontacts auth` immediately instead of waiting for timeouts.
