# Per-note change popover (dot next to the meta line)

**Date:** 2026-08-07
**Status:** Implemented

## Problem

A note page's meta line shows `Last updated Aug 4, 2026 · Created Aug 31, 2024 · 7 min read`,
but a returning reader can't tell *what* changed on the last updates. For a long, frequently
edited note like `will ai replace humans`, every edit is invisible — people don't know what was
added. We want a compact, hover-revealed history of recent edits, per note, driven by git —
**without** showing commit messages (they are generic: "update", "delete").

This is the companion to the homepage recent-notes badges
(`2026-08-07-recent-notes-change-badges-design.md`), which deliberately scoped note-page badges
out. This design revives that as a hover popover.

## What it looks like

A small green **dot** + muted blue **"recently updated"** label after `min read`. Hover (desktop)
or tap/focus (mobile) reveals a popover titled *Recent changes* listing up to 5 recent editing
sessions, newest first. Each row: **date + relative age** on the left, **`+added / −removed` words**
on the right (green / red). No commit messages, no note-total. If a note has no editing session in
the lookback window, no dot renders — the meta line is unchanged from today.

The `created` date is intentionally omitted from the popover: the meta line already shows "Created",
and git's first-commit date for the public content submodule (seeded later) disagrees with the
frontmatter `createddate`.

## Architecture

Hugo can't run git, so a build step emits a data file and Hugo renders it. Reuses the existing
`utils/recent_updates.py` and its single git scan — no new build step, no Makefile change.

```
content/ (git submodule)
   │  git log -p (one scan, <=180d)  +  git log --diff-filter=A (first-commit dates)
   ▼
utils/recent_updates.py
   ├─ data/recent_updates.json   (homepage badges — unchanged behavior)
   └─ data/note_changes.json     (NEW: per-note session history)   { stem: {sessions:[…]} }
   ▼
Hugo build: layouts/_default/single.html
   │  index site.Data.note_changes .File.BaseFileName
   ▼
dot + "recently updated" + hover popover next to the meta line
```

### Component 1 — `utils/recent_updates.py` (edit)

- `commit_word_stats` now tracks **added and removed words separately** per commit
  (`(datetime, added, removed)`) instead of a single gross number.
- New `group_sessions()` folds a note's commits into a list of sessions (newest first), grouping
  commits whose gap is `<= 24h` (`SESSION_GAP_HOURS`). Each session sums added/removed and records
  its newest commit datetime (`end`).
- Homepage badge output is derived from `sessions[0]` with `gross = added + removed` — **behavior
  identical** to before.
- `note_changes.json`: for each note, up to `MAX_SESSIONS = 5` sessions (skipping zero-word ones),
  each `{date, iso, rel, added, removed}`. `date` is `Aug 4` (or `Aug 4, 2025` for older years),
  `rel` is a coarse humanized age (`today` / `yesterday` / `N days ago` / `last week` /
  `N weeks ago` / `last month` / `N months ago` / `N years ago`), computed at build time.
- Keyed by the on-disk filename **stem** (== Hugo `.File.BaseFileName`), same as the badge file.
- Lookback stays 180 days: a note untouched for >180d simply shows no dot, which is the right signal.

### Component 2 — `layouts/_default/single.html` (edit)

After the `min read` span, guarded by `with (index site.Data.note_changes .File.BaseFileName)`:

```html
<span class="note-changes" tabindex="0" role="button" aria-label="Recent changes to this note">
  <span class="nc-dot"></span><span class="nc-label">recently updated</span>
  <span class="nc-popover" role="tooltip">
    <span class="nc-title">Recent changes</span>
    {{ range .sessions }}
    <span class="nc-row">
      <span class="nc-when"><time datetime="{{ .iso }}">{{ .date }}</time><span class="nc-rel">{{ .rel }}</span></span>
      <span class="nc-delta"><span class="nc-add">+{{ .added }}</span> / <span class="nc-del">−{{ .removed }}</span> words</span>
    </span>
    {{ end }}
  </span>
</span>
```

All elements are `<span>` (block-styled via CSS) because the meta line is a `<p>` — no invalid
block-in-paragraph nesting. `tabindex="0"` makes the wrapper focusable so mobile tap reveals the
popover via `:focus-within` (no JavaScript).

### Component 3 — `assets/styles/custom.scss` (edit, appended)

- Reuses existing `--badge-new` (autumnGreen) for the dot + additions and `--badge-upd` (dragonBlue)
  for the label; adds one token `--badge-del: #C34043` (autumnRed) for deletions. All three read on
  both light and dark, so no per-theme override needed.
- `.nc-popover` is `display:none`, shown by `.note-changes:hover` **or** `:focus-within`
  (desktop hover + mobile tap), absolutely positioned below the dot, `z-index:30`.
- Chunk starts with an ASCII-only comment/rule per the `sass-bom-drops-first-rule` note (a non-ASCII
  char at a chunk top makes Dart Sass emit a BOM that silently drops the first rule). Verified the
  `.note-changes` rule and `--badge-del` survive in the compiled `styles.css`.

### Component 4 — `.gitignore` (edit)

Add `data/note_changes.json` (regenerated every build, like `data/recent_updates.json`).

## Edge cases

- **No session in window** → no data entry → `with` skipped → no dot. Meta line unchanged.
- **Zero-word session** (image/frontmatter-only change) → skipped from the list; if a note has only
  such sessions it gets no entry and no dot.
- **Stem key mismatch** (case/space/unicode) → lookup misses → no dot (graceful).
- **Relative age staleness** → computed at build time; the site rebuilds on each deploy, so "2 days
  ago" is accurate at publish. Acceptable.
- **`make update`** (`git checkout upstream/hugo -- data`) may drop the generated file; regenerated
  on the next `make prepare`.

## Verification

- `python utils/recent_updates.py` → both JSON files written; `will ai replace humans` shows 5
  sessions with sensible deltas (e.g. Jul 3 `+180 / −124`, a real rewrite).
- `hugo` build (exit 0); rendered `will-ai-replace-humans/index.html` contains the popover with the
  5 rows; homepage badges still render.
- Compiled `styles.css` contains the `.note-changes` rule and `--badge-del` (no BOM drop).

## Out of scope (YAGNI)

- A full diff / "what exactly changed" text view (only counts + dates).
- Configurable session gap / lookback / max-sessions via config (hardcoded constants).
- Following renames across history (`git -M`) — sessions are per current path.
