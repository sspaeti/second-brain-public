#!/usr/bin/env bash
# Revert published notes whose only change vs HEAD is the `lastmod:` line
# (and, since the note-status feature, the `status:` / `status_source:` lines).
#
# Rationale: obsidian-quartz sets lastmod = max(existing lastmod, source file
# mtime). Saving a note in Obsidian bumps its mtime even when the content was
# reverted (e.g. accidental edit + undo), so the note's date gets bumped with no
# real change. content/ is a git submodule with its own history, so a note whose
# only diff vs HEAD is the lastmod line had no real change and is restored to its
# committed state. Next `make` bumps it again, this step reverts it again, until
# a real edit lands — self-correcting, dates stay in sync with actual content.
#
# Adding or removing a `#garden/<level>` tag in the vault is such a save: it
# bumps the mtime and changes only the `status:` / `status_source:` frontmatter
# lines (file_utils.rs). That is a real frontmatter change to keep, but not an
# edit a reader should see as "updated today", so for that case only the
# `lastmod:` line is restored from HEAD and the rest of the working tree stays.
#
# Restore is done by writing the HEAD blob straight to the working tree
# (git show HEAD:path > path) instead of `git checkout`. This never touches the
# submodule index, so it does not fight the index.lock held by concurrent
# lazygit / gitstatusd watchers on the content submodule.
#
# usage: utils/revert-lastmod-only.sh [content-dir]   (default: ../content)
# tests: utils/test_revert_lastmod_only.py (make test)
set -euo pipefail

cd "${1:-$(dirname "$0")/../content}"

git diff --name-only -- '*.md' | while IFS= read -r f; do
  # Changed content lines only: drop diff hunk/file headers, then drop the
  # lines a tag-only save may legitimately touch.
  changed=$(git diff -U0 -- "$f" \
    | grep -E '^[+-]' \
    | grep -vE '^(\+\+\+|---)' || true)
  real=$(printf '%s\n' "$changed" | grep -vE '^[+-](lastmod|status|status_source):' || true)
  if [ -n "$real" ]; then
    continue
  fi
  if printf '%s\n' "$changed" | grep -qE '^[+-](status|status_source):'; then
    # status changed: keep the working tree, put HEAD's lastmod back
    head_lastmod=$(git show "HEAD:$f" | grep -m1 -E '^lastmod:' || true)
    if [ -n "$head_lastmod" ]; then
      python3 - "$f" "$head_lastmod" <<'PY'
import re, sys
path, head_lastmod = sys.argv[1], sys.argv[2]
text = open(path, encoding="utf-8").read()
open(path, "w", encoding="utf-8").write(re.sub(r"^lastmod:.*$", lambda _: head_lastmod, text, count=1, flags=re.M))
PY
      echo "restored lastmod (status-only change): $f"
    fi
  else
    git show "HEAD:$f" > "$f"
    echo "reverted (lastmod-only): $f"
  fi
done
