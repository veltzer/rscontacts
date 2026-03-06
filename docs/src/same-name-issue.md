# Same Name Issue

## The Problem

You may have multiple contacts with the same name. For example, two friends both called "Mike" whose last names you don't know. The `check-contact-name-duplicate` command will flag these as issues, but how should you actually resolve them?

## What Google Contacts Shows in the List View

The contact list (on both the phone app and web UI) only shows a few fields at a glance:

- **Name** — always visible
- **Photo/avatar** — if one is set
- **Organization** — displayed as a subtitle under the name

All other fields (phone number, email, labels, notes, etc.) are only visible after tapping into the contact's detail view. This means that when you're scanning your contact list or picking a contact to call, only the name, photo, and organization can help you tell two people apart.

## No Separate Display Name

Google Contacts does not support a separate "display name" field. The `display_name` in the People API is a **read-only computed field** that is automatically generated from the structured name fields (given name, family name, etc.). You cannot set a display name independently of the actual name.

## The "File As" Field

Google Contacts has a "File as" field (`fileAses` in the People API), but it is a **sorting hint**, not a display name. It controls where the contact appears in an alphabetically sorted list, but the contact list still shows the actual name. Setting different "File as" values (e.g., "Mike Gym", "Mike Work") would affect sort order but would not help you visually distinguish same-name contacts in the list view.

## Options for Distinguishing Same-Name Contacts

### 1. Modify the Name

Add a distinguishing suffix or identifier to the name itself:

- "Mike R." / "Mike S."
- "Mike (work)" / "Mike (gym)"
- "Mike Tel Aviv" / "Mike Neighbor"

This is the most visible approach since the name is always shown, but it means the stored name no longer reflects the person's real name.

### 2. Use the Organization Field

Set the organization/company field to something descriptive (e.g., "gym", "yoga class", "neighbor"). This shows as a subtitle under the name in the contact list, so you can distinguish contacts without changing the name itself. This is the cleanest approach if you want to keep the real name intact.

Note: the organization field in the API (`organizations`) has multiple sub-fields — company name, job title, and department — but **only the company name** is displayed in the contact list view. Title and department are only visible in the contact detail view. In the Google Contacts UI, this field is labeled "Company". So when distinguishing same-name contacts, make sure to set the company name, not just the title.

### 3. Use the Name Suffix Field

The People API's Name resource has an `honorific_suffix` field (intended for "Jr.", "III", "PhD", etc.). You could set it to "1", "2", "3" or other short identifiers to distinguish same-name contacts. The suffix gets incorporated into the computed display name, so "Mike" with suffix "1" would display as "Mike 1". This works, but it's a hack — the field is meant for real honorific suffixes, not arbitrary identifiers. It's functionally equivalent to just modifying the name directly.

### 4. Set Different Photos

If you have photos of both people, assigning different contact photos makes them instantly recognizable in the list view.

### 5. Leave Them as Duplicates

You can simply leave both contacts as "Mike" and accept the warning from `check-contact-name-duplicate`. Currently rscontacts has no mechanism to suppress or allowlist known duplicates, so this check will continue to flag them on every run.

## Recommendation

Using the **organization field** is generally the best compromise: it keeps the real name intact while providing a visible subtitle in the contact list. If that's not enough, modifying the name with a short qualifier is the most reliable fallback.
