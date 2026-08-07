# Recent-notes change badges + newsletter footer on the homepage

**Date:** 2026-08-07
**Status:** Approved design, ready for implementation plan

## Problem

The homepage recent-notes list (`https://www.ssp.sh/brain/#recent-notes`) sorts notes by
last-modified but shows only a date. A reader can't tell whether a note is brand new, got a
full new paragraph, or had a single typo fixed. We want a compact visual signal per note that
answers "new note, or how big was the last change?" — and a newsletter call-to-action at the
bottom, because the newsletter is where the consolidated monthly digest (brain + blog + books +
book chapters + social) lives.

The newsletter generator (`../listmonk-rss/newsletter.py`) already computes exactly this kind of
per-note word delta from git history. We reuse that *logic* (ported, not imported) to drive a
build-time data file that Hugo renders.

## Decisions (locked)

- **What the number means:** the most recent *editing session* for the note — the run of commits
  ending at its last commit, grouping any commits whose gap is <= 24h into one session. This
  coalesces "I fixed it 3x that afternoon" into a single number that matches the single date shown.
- **Change metric:** *gross* words touched = added + deleted words in that session's diffs. Best at
  distinguishing typo (~2) vs reworded sentence (~20) vs new paragraph (~80), and it catches
  rewrites where the note didn't grow.
- **New vs updated:** if the note's first-ever commit falls inside that most-recent session, it's
  `new` and we show the note's **total** current word count; otherwise `updated` and we show the
  **gross** session words.
- **Visual:** a small color-coded pill next to the title. Green `NEW`, blue `UPD`. Labels are
  compact: `NEW · 1,079w` and `UPD · ~80w` (the `~` signals the update count is approximate).
  If a note is in the recent list but has no session in the lookback window, render no pill (the
  row is unchanged from today).
- **Newsletter CTA:** reuse the existing inline Listmonk signup partial `newsletter-footer.html`
  at the bottom of the recent-notes section, with parameterized copy (see below).
- **Scope:** homepage recent list only — not individual note pages. No minimum threshold (a 2-word
  typo still shows `UPD · ~2w`; that is the whole point).

## Architecture

Hugo cannot run git diffs, so a build step produces the numbers and Hugo just renders them.

```
make prepare
  ... copy content from vault ...
  python utils/recent_updates.py        # NEW: reads content/ git log -> data/recent_updates.yaml
  hugo-obsidian ...                      # (existing)
  ...
make run / hugo-generate                 # Hugo reads data/recent_updates.yaml, renders pills
```

### Component 1 — `utils/recent_updates.py` (new, stdlib only)

Standalone script, no third-party deps, no cross-repo import. It **ports** the git-word-stat
logic from `newsletter.py` (`_git_word_stats`, `_first_commit_dates`) and adapts it:

- Run once: `git -C content log --since=<LOOKBACK> -p --format=__COMMIT__%H|%ai -- '*.md'`
  (also `core.quotePath=false`, matching newsletter.py). `LOOKBACK` default **180 days** — more
  than enough to cover the 30 notes shown (`recentNotes = 30`); configurable via a constant / env.
- Parse the `-p` output into, per file, a list of `(commit_datetime, gross_words)` where
  `gross_words = added_words + deleted_words` counted from `+`/`-` diff lines (skip `+++`/`---`/`@@`
  headers and binary files, exactly as newsletter.py does).
- Also run `git log --reverse --diff-filter=A --name-only` once to get each file's first-commit
  date (ported `_first_commit_dates`).
- **Session grouping:** for each file, sort its commits newest->oldest; start a session at the
  newest commit and keep absorbing the next-older commit while the gap to the previous kept commit
  is `<= 24h`; stop at the first larger gap. Sum `gross_words` over the session; the session's
  span is [oldest-in-session, newest].
- **Classify:**
  - `new` if the file's first-ever commit datetime is within the session span -> `words` = total
    current word count of the file (strip frontmatter, whitespace-split; markdown syntax counted,
    consistent with the diff-based counting).
  - else `updated` -> `words` = session gross.
- **Key:** the on-disk filename stem, e.g. `mini retirements` (from `Path(path).stem`). This must
  equal Hugo's `.File.BaseFileName` for the lookup to hit. **Verify during implementation** that
  filenames in `content/` are byte-for-byte what Hugo exposes (the ls showed mixed-case names
  preserved, so the raw stem should match; confirm with one known note before finishing).
- **Output:** `data/recent_updates.json` (Hugo reads `.json` and `.yaml` data files identically;
  JSON avoids hand-rolling YAML escaping and is stdlib `json.dump`). Keyed by stem, overwrite every
  run:

  ```json
  {
    "mini retirements": { "status": "new", "words": 1079 },
    "excel never dies": { "status": "updated", "words": 80 }
  }
  ```

  Accessed in templates as `site.Data.recent_updates` (Hugo strips the extension).

- Add `data/recent_updates.json` to `.gitignore` (it's regenerated each build; today `.gitignore`
  only has `public`).

### Component 2 — `layouts/partials/page-list.html` (edit)

After the `<h3>` title link, add the pill:

```html
{{ with (index site.Data.recent_updates .File.BaseFileName) }}
  {{ if eq .status "new" }}
    <span class="update-badge badge-new">NEW · {{ lang.FormatNumberCustom 0 .words }}w</span>
  {{ else }}
    <span class="update-badge badge-upd">UPD · ~{{ lang.FormatNumberCustom 0 .words }}w</span>
  {{ end }}
{{ end }}
```

- `index site.Data.recent_updates .File.BaseFileName` returns nil for notes with no session -> the
  `with` block is skipped -> no pill, row unchanged.
- `lang.FormatNumberCustom 0 .words` renders `1079` as `1,079`. (Confirm helper name in the Hugo
  version; fallback to plain `.words` if unavailable.)
- `page-list.html` is also used by related-pages elsewhere; the `with` guard means non-recent
  contexts (no data entry) simply render nothing. Safe.

### Component 3 — `assets/styles/custom.scss` (edit)

Add two kanagawa-palette variables to `:root` and a `.update-badge` rule set. The two colors read
fine on both light and dark backgrounds, so no `[saved-theme="dark"]` override is needed.

| Variable | Value | Use |
|----------|-------|-----|
| `--badge-new` | `#76946A` (autumnGreen) | NEW pill border + tint |
| `--badge-upd` | `#658594` (dragonBlue, same as dark `--secondary` / internal-link accent) | UPD pill border + tint |

Pill style: small inline-block, **neutral body text** (`color: var(--gray)`, `font-weight: 400`) so
it doesn't look loud; the color lives only in the `border-color` and a low-alpha `color-mix`
background tint. Rounded, `0.62em` font. Place the new SCSS chunk so the **first rule/line is ASCII-only** — see the
`sass-bom-drops-first-rule` memory: a non-ASCII character at the top of a compiled chunk makes Dart
Sass emit a BOM that silently drops the first rule. Keep the kanagawa names in comments (ASCII) and
no emoji/smart-quotes at chunk start.

Example:

```scss
// Recent-notes change badges (kanagawa palette; neutral text, colored border+tint)
:root {
  --badge-new: #76946A; // autumnGreen
  --badge-upd: #658594; // dragonBlue (== dark --secondary)
}
.update-badge {
  display: inline-block;
  margin-left: 0.5em;
  padding: 0.05em 0.5em;
  font-size: 0.62em;
  font-weight: 400;
  line-height: 1.5;
  letter-spacing: 0.02em;
  border-radius: 0.5em;
  white-space: nowrap;
  vertical-align: middle;
  color: var(--gray);
  border: 1px solid transparent;
}
.badge-new { border-color: var(--badge-new); background: color-mix(in srgb, var(--badge-new) 14%, transparent); }
.badge-upd { border-color: var(--badge-upd); background: color-mix(in srgb, var(--badge-upd) 14%, transparent); }
```

### Component 4 — Newsletter footer (parameterize + include)

Make `layouts/partials/newsletter-footer.html` accept an optional dict with `label` and `desc`,
defaulting to today's copy so the two existing callers keep working unchanged:

```html
{{ $label := .label | default "Data Engineering & Second Brain Newsletter" }}
{{ $desc := .desc | default "This note is part of my Second Brain, a 650+ public notes on data engineering, life, and more. I send a digest of new notes, blog posts, and book chapters, and general updates by email. Subscribe below if you'd like to get the next one." }}
```

Replace the hardcoded `nl-footer-label` / `nl-footer-desc` text with `{{ $label }}` / `{{ $desc }}`.

- Existing callers (`_default/single.html`, `shortcodes/newsletter-form.html`) pass the Page `.`;
  `.label`/`.desc` are nil on a Page, so `default` kicks in — backward compatible.
- `layouts/partials/recent.html` appends, after the list:

  ```html
  {{ partial "newsletter-footer.html" (dict
      "label" "Get the monthly digest"
      "desc" "These are the latest changes to my Second Brain. Once a month I send a consolidated summary by email — not just the brain, but new blog posts, books I'm reading, chapters of my own book, and social highlights. Subscribe below to get the next one.") }}
  ```

### Component 5 — Makefile (edit)

Add one line to the `prepare` target after content is copied (after `obsidian-quartz` / the
`revert-lastmod-only.sh` step, before or after `hugo-obsidian` — order-independent since it only
reads `content/` git history):

```make
python utils/recent_updates.py   # per-note change badges -> data/recent_updates.json
```

## Data flow

```
content/ (git submodule history)
      |  git log -p / --diff-filter=A
      v
utils/recent_updates.py  --(session grouping, classify)-->  data/recent_updates.json  { stem: {status, words} }
      |
      v
Hugo build: recent.html -> page-list.html
      |  index site.Data.recent_updates .File.BaseFileName
      v
  pill rendered next to each recent-note title  +  newsletter-footer partial below the list
```

## Edge cases

- **No session in window** (note older than lookback, or only lastmod-bump commits that
  `revert-lastmod-only.sh` already undid): no data entry -> no pill. Row unchanged.
- **Gross == 0** (e.g. only an image/frontmatter change survived): render `UPD` with no number, or
  `UPD · ~0w`. Decision: show `UPD` with no number when gross is 0.
- **Filename-stem key mismatch** (case/space/unicode differences between on-disk name and
  `.File.BaseFileName`): the lookup silently misses -> no pill (graceful, not an error). Verify one
  known note during implementation; if mismatches are common, switch the key to the slug derived
  from `.RelPermalink` on both sides.
- **`make update`** does `git checkout upstream/hugo -- data`, which could remove the generated
  file. It's regenerated on the next `make prepare`, and `update` is a rare manual op — acceptable.
- **Deleted notes** appearing in git log: skip if the file no longer exists (ported behavior).

## Testing / verification

- Run `python utils/recent_updates.py` standalone; spot-check that the notes in the current
  newsletter draft (`Mini Retirements` new/1079, `Excel Never Dies` new/533, `Smart Note Taking`
  updated) get sensible `status`/`words`. Numbers won't match the newsletter exactly (newsletter
  sums a different window and uses net, not gross) — check they're in the right ballpark and the
  new/updated split is correct.
- `make serve` and view the homepage: pills render, colors correct in light and dark mode, footer
  form appears once below the list with the custom copy.
- Confirm the SCSS chunk didn't drop a rule (BOM check): the first `.update-badge` rule actually
  applies.

## Out of scope (YAGNI)

- Badges on individual note pages.
- A per-note "history" or diff view.
- Configurable metric/threshold via config.toml (hardcoded constants in the script for now).
- Handling renamed notes across history (git `-M` follow) — sessions are per current path.
