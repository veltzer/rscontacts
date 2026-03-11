# Phone Sync Issues

Sometimes your Google Contacts data goes out of sync with what is actually
displayed on your phone. You may notice missing contacts, stale phone numbers,
or contacts that you have already fixed via `rscontacts` still showing their
old values on the device.

## Symptoms

- Contacts on your phone do not match what you see in Google Contacts on the web.
- Changes made through `rscontacts --fix` or the Google Contacts web UI are not
  reflected on your phone.
- Duplicate or outdated entries keep appearing on the device.

## Solution

1. **Back up your contacts.** Export your contacts from
   [Google Contacts](https://contacts.google.com/) (or use `rscontacts list`)
   so you have a safe copy before making any changes on the device.

2. **Open your phone's Settings** and navigate to **Apps** (or **Application Manager**,
   depending on your Android version).

3. **Show system apps.** Tap the three-dot menu (or filter) and enable
   "Show system apps" so that hidden system applications are visible.

4. **Find "Contacts Storage".** This is the system app that caches contact data
   locally on the device. It is separate from the "Contacts" app you normally use.

5. **Clear cache and data.** Open "Contacts Storage", then:
   - Tap **Clear Cache**.
   - Tap **Clear Data** (or **Clear Storage**).

6. **Wait for re-sync.** Your phone will re-download all contact data from Google.
   This may take a few minutes depending on how many contacts you have. Once the
   sync completes, your phone contacts should match the server and the issue will
   be resolved.

> **Note:** Clearing the data of Contacts Storage only removes the local cache.
> Your actual contacts remain safe in your Google account and will sync back
> automatically.
