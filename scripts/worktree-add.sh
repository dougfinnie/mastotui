#!/usr/bin/env bash
# Create a new git worktree for a feature branch.
# Usage: ./scripts/worktree-add.sh <short-name> [base-branch]
# Example: ./scripts/worktree-add.sh timeline-ui main
#   -> creates ../mastotui-timeline-ui on branch feat/timeline-ui (from main)

set -euo pipefail
NAME="${1:?Usage: $0 <short-name> [base-branch]}"
BASE="${2:-main}"
BRANCH="feat/${NAME}"
REPO_ROOT="$(git -C "$(dirname "$0")/.." rev-parse --show-toplevel)"
PARENT="$(dirname "$REPO_ROOT")"
WORKTREE_PATH="${PARENT}/mastotui-${NAME}"

if [[ -d "$WORKTREE_PATH" ]]; then
  echo "Already exists: $WORKTREE_PATH" >&2
  exit 1
fi

git -C "$REPO_ROOT" worktree add "$WORKTREE_PATH" -b "$BRANCH" "$BASE"
echo "Created worktree: $WORKTREE_PATH (branch $BRANCH)"
echo "  cd $WORKTREE_PATH   # work there, then ask for agent review before merging"
