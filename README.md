# mastotui

Terminal UI client for Mastodon. Read your home timeline, view toots, post, reply, boost, and favourite from the terminal.

## Build

```bash
cargo build --release
```

### Linux (secret storage)

Credentials are stored in the system keyring (Secret Service). You need D-Bus development files:

- **Fedora**: `sudo dnf install dbus-devel pkgconf-pkg-config`
- **Debian/Ubuntu**: `sudo apt install libdbus-1-dev pkg-config`

Then:

```bash
cargo build --release
```

## Run

```bash
cargo run
# or
./target/release/mastotui
```

On first run you’ll see the login screen. Enter your instance URL (e.g. `https://mastodon.social`), press Enter to open the browser and authorize, then paste the code back and press Enter again.

## Keys

- **Login**: type instance URL or code, Enter to submit, `q` quit
- **Timeline**: `↑`/`↓` or `j`/`k` move, `Enter` open toot, `n` new toot, `r` refresh, `q` quit
- **Toot detail**: `b` boost, `f` favourite, `r` reply, `Esc` back
- **Compose**: type, `Enter` post, `Esc` cancel

## Spec and Tracey

Requirements are in `docs/spec/mastotui.md`. Use [Tracey](https://github.com/bearcove/tracey) for coverage: `tracey query status`, `tracey web`.
