#!/usr/bin/env bash
# Check for spec/code drift: validate refs, require full impl + verify coverage, no stale refs.
# Exit 0 only if all checks pass. Use in CI or pre-push.
set -euo pipefail
ROOT="${1:-.}"
cd "$ROOT"

FAILED=0

# 1. Spec and implementation validate (broken refs, unknown prefixes, etc.)
if ! tracey query validate 2>&1; then
  echo "FAIL: tracey query validate"
  FAILED=1
fi

# 2. Every requirement has an implementation reference
OUT=$(tracey query uncovered 2>&1)
echo "$OUT"
echo "$OUT" | grep -q '0 uncovered' || { echo "FAIL: requirements without implementation (run: tracey query uncovered)"; FAILED=1; }

# 3. Every requirement has a verification reference (test coverage)
OUT=$(tracey query untested 2>&1)
echo "$OUT"
echo "$OUT" | grep -q '0 untested' || { echo "FAIL: requirements without verification (run: tracey query untested)"; FAILED=1; }

# 4. No stale references (code pointing to older rule versions)
OUT=$(tracey query stale 2>&1)
echo "$OUT"
echo "$OUT" | grep -q 'no stale references' || { echo "FAIL: stale references (run: tracey query stale, then update annotations)"; FAILED=1; }

if [[ $FAILED -eq 1 ]]; then
  echo "Spec drift check failed. Fix the issues above or update docs/spec and annotations."
  exit 1
fi
echo "Spec drift check passed."
