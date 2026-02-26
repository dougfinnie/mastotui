# mastotui

A distraction-free Mastodon client built on [Ratatui](https://ratatui.rs). Read your home timeline, view toots, post, reply, boost, and favourite from the terminal.

**Repository:** [github.com/dougfinnie/mastotui](https://github.com/dougfinnie/mastotui)

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
- **Timeline**: `↑`/`↓` or `j`/`k` move, `Enter` open toot, `n` new toot, `r` refresh from top, `m` load more, `q` quit
- **Toot detail**: `b` boost, `f` favourite, `r` reply, `Esc` back. Boosted toots show the original post and author with "Boosted by @user" at the top.
- **Compose**: type, `Enter` post, `Esc` cancel

## Spec and Tracey

Requirements are in `docs/spec/mastotui.md`. Use [Tracey](https://github.com/bearcove/tracey) for coverage: `tracey query status`, `tracey web`.

### Pre-commit hook (format, clippy, test, spec)

To avoid CI failures on push, use the git hook that runs the same checks as CI:

```bash
./scripts/setup-git-hooks.sh
```

Then every `git commit` will run `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`, and `./scripts/check-spec.sh`. Requires [Tracey](https://github.com/bearcove/tracey) for the spec check. To skip the hook once: `SKIP_PRE_COMMIT=1 git commit ...`.

### Detecting spec/code drift (CI or local)

Run all checks (validate refs, require full impl + verify coverage, no stale refs):

```bash
./scripts/check-spec.sh
```

Individual Tracey commands you can run in a pipeline:

| Command | Purpose |
|--------|--------|
| `tracey query validate` | Fail if any refs are broken or invalid |
| `tracey query uncovered` | List requirements with no `r[impl]` (fail if not "0 uncovered") |
| `tracey query untested` | List requirements with no `r[verify]` (fail if not "0 untested") |
| `tracey query stale` | List refs pointing to old rule versions (fail if not "no stale references") |
| `tracey pre-commit` | In a git pre-commit hook: fail if spec rule text changed without version bump |

To enforce no drift in CI, run `./scripts/check-spec.sh` after `cargo fmt -- --check`, `cargo clippy -- -D warnings`, and `cargo test`. See `.github/workflows/ci.yml` if present.

## License

Dual-licensed under **MIT** or **Apache-2.0**; see [LICENSE-MIT](LICENSE-MIT) for the MIT text.
