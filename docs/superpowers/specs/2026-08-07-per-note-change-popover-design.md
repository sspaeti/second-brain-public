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

Hugo can't run git, so a build step emits one data file and Hugo renders it. Reuses the existing
`utils/recent_updates.py` and its single git scan — no new build step, no Makefile change. The
badge fields and the popover history live in **one** file keyed by the same stem, because both
derive from the same commits in the same build run. These `data/*.json` files are build-time only:
Hugo bakes the values into the generated HTML, so nothing is downloaded by visitors and nothing
touches note frontmatter.

```
content/ (git submodule)
   │  git log -p (one scan, <=365d)  +  git log --diff-filter=A (first-commit dates)
   ▼
utils/recent_updates.py
   └─ data/recent_updates.json   { stem: {status, words, sessions:[…]} }
   ▼
Hugo build
   ├─ page-list.html : index site.Data.recent_updates .File.BaseFileName -> .status/.words (badge)
   └─ single.html    : index site.Data.recent_updates .File.BaseFileName -> .sessions (popover)
   ▼
dot + "recently updated" + hover popover next to the meta line
```

### Component 1 — `utils/recent_updates.py` (edit)

- `commit_word_stats` now tracks **added and removed words separately** per commit
  (`(datetime, added, removed)`) instead of a single gross number, and **excludes the frontmatter
  block** from the count: it tracks new/old file line numbers through each diff hunk and skips any
  `+`/`-` line whose line number is `<= _frontmatter_end_line(file)` (the closing `---`). This keeps
  metadata-only commits — OG `description:`, `lastmod:`, moved `createddate:` — from reading as
  content edits. (The `lastmod:` case is also guarded upstream by `revert-lastmod-only.sh`, which
  drops pure-lastmod diffs before commit; this exclusion is broader and catches any frontmatter
  field.)
- `LOOKBACK_DAYS = 365` — the popover history reaches ~1 year back (was 180).
- **Badge session choice:** the homepage badge uses the most recent session that is the note's
  creation *or* has non-zero content churn, so a trailing metadata-only commit doesn't make the
  badge read `0 words`.
- New `group_sessions()` folds a note's commits into a list of sessions (newest first), grouping
  commits whose gap is `<= 24h` (`SESSION_GAP_HOURS`). Each session sums added/removed and records
  its newest commit datetime (`end`).
- Homepage badge output is derived from `sessions[0]` with `gross = added + removed` — **behavior
  identical** to before.
- Each note's entry gains a `sessions` list: up to `MAX_SESSIONS = 5` sessions (skipping zero-word
  non-creation ones), each `{date, iso, rel, added, removed}`. `date` is `Aug 4` (or `Aug 4, 2025`
  for older years); `rel` is a coarse humanized age (`today` / `yesterday` / `N days ago` /
  `last week` / `N weeks ago` / `last month` / `N months ago` / `N years ago`), computed at build
  time. A note whose only edits touched zero words gets a badge but no `sessions` key (no dot).
- **Creation row:** the session that contains the note's first-ever commit is rendered as
  `{kind:"new", words: <total note words>}` instead of `added/removed`, so a fresh note shows
  `new · N words` (matching the badge) rather than reading as a big churny edit. Later,
  separate-day sessions keep their `+added / −removed`.
- Single output `data/recent_updates.json`, keyed by the on-disk filename **stem**
  (== Hugo `.File.BaseFileName`).
- Lookback is 365 days: a note untouched for >1y simply shows no dot, which is the right signal.

### Component 2 — `layouts/_default/single.html` (edit)

After the `min read` span, guarded by
`with (index site.Data.recent_updates .File.BaseFileName)` then `with .sessions`:

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
  for the label; adds `--badge-del: #C34043` (autumnRed) for deletions. All three read on both light
  and dark, so no per-theme override needed.
- **Critical:** the theme has `article > .meta { opacity: .7 }`. `opacity` on an ancestor makes the
  popover translucent (can't be undone by a child) **and** creates a stacking context that traps the
  popover below the article body (callouts/paragraphs paint on top). This chunk overrides it to
  `opacity: 1` and re-mutes the meta via `color: var(--global-font-secondary-color)`.
- Popover surface uses a solid, theme-fitting token `--nc-bg` (`#ffffff` light / `#16161D` dark,
  kanagawa sumiInk0) — fully opaque, no `backdrop-filter` — plus a `1px` border and a
  `0 10px 30px rgba(0,0,0,0.32)` shadow for separation.
- `.nc-popover` is `display:none`, shown by `.note-changes:hover`, `:focus-within`, **or**
  `.nc-open` (see below), absolutely positioned below the dot, `z-index:40`.

### Component 5 — mobile tap toggle (small inline script in `single.html`)

Touch devices have no hover; `:focus-within` opens the popover on tap but nothing dismisses it. A
tiny script toggles a `.nc-open` class on tap, and closes on outside-tap or `Escape` (calling
`blur()` so `:focus-within` also releases). Clicks inside the popover don't close it. Desktop hover
is untouched. The script no-ops when the page has no `.note-changes`.
- Chunk starts with an ASCII-only comment/rule per the `sass-bom-drops-first-rule` note (a non-ASCII
  char at a chunk top makes Dart Sass emit a BOM that silently drops the first rule). Verified the
  `.note-changes` rule and `--badge-del` survive in the compiled `styles.css`.

### Component 4 — `.gitignore`

No change: `data/recent_updates.json` is already ignored (regenerated every build). No second file.

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
