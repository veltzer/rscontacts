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
