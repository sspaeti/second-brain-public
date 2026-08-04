#!/usr/bin/env bash
# Revert published notes whose ONLY change vs HEAD is the `lastmod:` line.
#
# Rationale: obsidian-quartz sets lastmod = max(existing lastmod, source file
# mtime). Saving a note in Obsidian bumps its mtime even when the content was
# reverted (e.g. accidental edit + undo), so the note's date gets bumped with no
# real change. content/ is a git submodule with its own history, so a note whose
# only diff vs HEAD is the lastmod line had no real change and is restored to its
# committed state. Next `make` bumps it again, this step reverts it again, until
# a real edit lands — self-correcting, dates stay in sync with actual content.
#
# Restore is done by writing the HEAD blob straight to the working tree
# (git show HEAD:path > path) instead of `git checkout`. This never touches the
# submodule index, so it does not fight the index.lock held by concurrent
# lazygit / gitstatusd watchers on the content submodule.
set -euo pipefail

cd "$(dirname "$0")/../content"

git diff --name-only -- '*.md' | while IFS= read -r f; do
  # Changed content lines only: drop diff hunk/file headers, then drop lastmod.
  real=$(git diff -U0 -- "$f" \
    | grep -E '^[+-]' \
    | grep -vE '^(\+\+\+|---)' \
    | grep -vE '^[+-]lastmod:' || true)
  if [ -z "$real" ]; then
    git show "HEAD:$f" > "$f"
    echo "reverted (lastmod-only): $f"
  fi
done
