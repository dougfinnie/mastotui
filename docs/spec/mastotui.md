# mastotui specification

Terminal user interface client for Mastodon. This spec defines requirements for the MVP (v0.1).

## Authentication

### App registration

r[auth.app.register.on-first-login]
Given a valid Mastodon instance URL, when the user runs login for the first time, the client MUST register an app (if needed), store client id and client secret, and initiate OAuth.

r[auth.app.register.skip-when-stored]
Given client id and secret are already stored for the instance, when the user runs login, the client MUST skip app registration and go straight to OAuth.

### User login

r[auth.login.exchange-code]
Given OAuth has been initiated, when the user completes the flow in the browser and returns, the client MUST exchange the authorization code for an access token and store the token (e.g. in config).

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
Given the user submits without content or over the character limit, the client MUST NOT send the request and MUST show a validation error.

r[toot.reply]
Given a toot is open, when the user chooses Reply and submits, the client MUST POST a reply with the correct in_reply_to_id and show success.

### Boost and favourite

r[toot.boost.toggle]
Given a toot is visible, when the user triggers Boost, the client MUST call the API to boost or un-boost (if already boosted) and update the displayed state.

r[toot.favourite.toggle]
Given a toot is visible, when the user triggers Favourite, the client MUST call the API to favourite or un-favourite (if already favourited) and update the displayed state.

## Configuration and persistence

r[config.persist-after-login]
Given the app has completed login, when the app exits, the instance URL, client id/secret (if used), and access token MUST be stored in a local config (e.g. under XDG config dir such as ~/.config/mastotui/).

r[config.first-run]
Given the user has no config, when the app starts, the first screen MUST be "add instance" or login, not the timeline.
