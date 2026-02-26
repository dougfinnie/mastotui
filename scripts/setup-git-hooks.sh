#!/usr/bin/env bash
# Use repo .githooks for commit hooks so pre-commit runs format, clippy, test, and spec check.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
git config core.hooksPath .githooks
echo "Git hooks are now in .githooks (pre-commit will run fmt, clippy, test, check-spec)."
