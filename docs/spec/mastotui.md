# mastotui specification

Terminal user interface client for Mastodon. This spec defines requirements for the MVP (v0.1).

## Authentication

### App registration

r[auth.app.register.on-first-login]
Given a valid Mastodon instance URL, when the user runs login for the first time, the client MUST register an app (if needed), store client id and client secret, and initiate OAuth.

r[auth.app.register.skip-when-stored]
Given both client id (in config) and client secret (in secure storage) are already stored for the instance, when the user runs login, the client MUST skip app registration and go straight to OAuth. If only one is present (e.g. secret in keyring but config file missing), the client MUST NOT fail with "no config"; it MUST re-register the app to recover and obtain a valid client id.

### User login

r[auth.login.exchange-code]
Given OAuth has been initiated, when the user completes the flow in the browser and returns, the client MUST exchange the authorization code for an access token (authenticating to the token endpoint with client credentials, e.g. HTTP Basic or form body) and store the token in secure storage (e.g. system keyring), not in plain config.

r[auth.login.use-stored-token]
Given a stored access token exists, when the app starts, the client MUST use it without asking for login again until the token is invalid.

r[auth.login.invalid-token]
Given the stored token is invalid or expired, when any API call is made, the client MUST detect 401, clear the stored token, and prompt for re-login (or redirect to the login flow).

## Timeline

r[timeline.home.fetch]
Given the user is logged in, when the home view is shown, the client MUST fetch the home timeline from the API and display toots (author, content snippet, time).

r[timeline.home.empty-state]
Given the home timeline is loaded, when the API response is empty, the UI MUST show an empty state (e.g. "No toots") instead of an error.

r[timeline.pagination]
Given the home timeline is shown, when the user scrolls to the bottom (or triggers "load more"), the client MUST fetch the next page and append toots to the timeline.

## Toot actions

### View toot

r[toot.view-detail]
Given a toot is visible in the timeline, when the user selects it (e.g. Enter), the client MUST show full content, thread context if any, and reply/boost/favourite actions.

### Compose and reply

r[toot.post.submit]
Given the user is on the compose screen with valid text, when the user submits, the client MUST POST the toot and show success (and optionally refresh the timeline).

r[toot.post.validation]
Given the user submits without content or over the character limit (e.g. 500 characters), the client MUST NOT send the request and MUST show a validation error.

r[toot.reply]
Given a toot is open, when the user chooses Reply and submits, the client MUST POST a reply with the correct in_reply_to_id and show success.

### Boost and favourite

r[toot.boost.toggle]
Given a toot is visible, when the user triggers Boost, the client MUST call the API to boost or un-boost (if already boosted) and update the displayed state.

r[toot.favourite.toggle]
Given a toot is visible, when the user triggers Favourite, the client MUST call the API to favourite or un-favourite (if already favourited) and update the displayed state.

## Configuration and persistence

r[config.persist-after-login]
Given the app has completed login, when the app exits, the instance URL and client id MUST be stored in local config (e.g. under XDG config dir such as ~/.config/mastotui/). The client secret and access token MUST be stored in secure storage (e.g. system keyring), not in the config file.

r[config.first-run]
Given the user has no config, when the app starts, the first screen MUST be "add instance" or login, not the timeline.

## Implementation notes

These are not requirements but document how the current implementation satisfies the spec:

- **OAuth:** The app uses PKCE and out-of-band redirect (`urn:ietf:wg:oauth:2.0:oob`); the user pastes the authorization code into the TUI. The token request sends client credentials in the form body only (client_secret_post). Sending both Basic and form can cause "unsupported authentication method" on some instances (e.g. union.place / Doorkeeper).
- **Secure storage:** On Linux, macOS, and Windows, the client secret and access token are stored in the system credential store (e.g. Secret Service, Keychain, Credential Manager), not in the config file.
- **Skip vs re-register:** Skip is only used when both config (instance URL + client id) and keyring (client secret) exist for the instance. If the keyring has a secret but the config file is missing (e.g. app was closed before first successful login), the app re-registers and overwrites the stored secret so login can proceed.
- **Character limit:** The compose UI enforces a 500-character limit for new toots and replies; instances may have different limits.
- **Timeline fetch (r[timeline.home.fetch]):** The home timeline is fetched when the home view is shown with a client: (1) on cold start, the main loop calls `ensure_timeline_loaded()` each tick, which fetches when view is Timeline, client is present, not loading, statuses empty, and no prior load error; (2) after successful login, the app immediately calls `load_timeline(false)` so the timeline appears without waiting for the next tick. Load errors are shown in the timeline area (`timeline_message`); auto-fetch does not retry every tick after a failure (user presses `r` to retry).
- **Refresh vs load more (r[timeline.pagination]):** `r` = refresh from top (replace statuses); `m` = load more (append next page). This keeps "refresh" and "load more" distinct per spec.
- **Boosted toots (r[toot.view-detail]):** When a timeline item or opened toot is a reblog, the UI shows the original author and full content of the boosted post, with "boosted by @user" context so the booster is still visible. The API returns the wrapper status with `reblog` set to the original; we display the inner status for content and author.
- **Timeline scroll:** The list scrolls so the selected toot stays visible. Visible row count is taken from the terminal each draw (`timeline_visible_rows`); on ↑/↓ or j/k the scroll position is updated so selection remains in view (and is corrected on resize).
