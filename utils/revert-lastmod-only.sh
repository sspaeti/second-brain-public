#!/usr/bin/env bash
# Revert published notes whose only change vs HEAD is the `lastmod:` line
# (and, since the note-status feature, the `status:` / `status_source:` lines;
# since note categories, the `categories:` / `category_source:` lines too).
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
# The same holds for the `categories:` line `obsidian-quartz note-category`
# injects; when only that line differs (no lastmod bump) nothing is touched.
#
# Restore is done by writing the HEAD blob straight to the working tree
# (git show HEAD:path > path) instead of `git checkout`. This never touches the
# submodule index, so it does not fight the index.lock held by concurrent
# lazygit / gitstatusd watchers on the content submodule.
#
# One `git diff -U0` for all notes and one Python pass over it (instead of two
# git calls plus a python start per dirty note): with every note dirty after a
# category change the old loop took ~6 s, this takes ~0.1 s. Same rules.
# `core.quotepath=false` keeps non-ASCII names (en dash, umlaut) unescaped.
#
# usage: utils/revert-lastmod-only.sh [content-dir]   (default: ../content)
# logic: utils/revert_lastmod_only.py; tests: utils/test_revert_lastmod_only.py (make test)
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
cd "${1:-$HERE/../content}"

# --src-prefix/--dst-prefix pin the a/..b/ headers the parser keys on (the
# user's diff.mnemonicPrefix would emit i/..w/).
git -c core.quotepath=false diff -U0 --no-color --src-prefix=a/ --dst-prefix=b/ -- '*.md' \
  | python3 "$HERE/revert_lastmod_only.py"
