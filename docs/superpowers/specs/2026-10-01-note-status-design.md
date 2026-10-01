# Note status: started → growing → evergreen

**Date:** 2026-10-01
**Status:** Draft, awaiting review

## Problem

Readers can't tell whether a public note is a quick lookup jotted in five minutes or an essay
tended for two years. The vault has a four-level emoji taxonomy (`📬`, `🗃/🌻`, `🗃/📖`, `🗃/🌳`,
see `content/taxonomy of note types.md`), but:

- `utils/obsidian-quartz/src/file_utils.rs` strips those emoji tags on purpose
  (`EXCLUDED_TAG_EMOJIS`), so no status ever reaches the site. Only 24 of ~1534 public notes carry
  any tag at all.
- The taxonomy is rarely updated by hand; a note that outgrew `📬` keeps its tag for years.
- The only reader-facing expectation setting is `content/secondbrain disclaimer.md`, linked from
  `_index.md`, which nobody reads before landing on a note from search.

We want a per-note signal, in the note's own meta line, that costs the author nothing for the
common case and can be overridden per note when the heuristic is wrong.

## Vocabulary

Three public levels. Lowercase single words, so they fit a pill and read as "a *started* note":

| Level | Reader-facing sentence (popover) | Vault mapping |
|---|---|---|
| `started` | **Started.** Quick capture, barely worked on. May change or vanish. | `📬`, fresh short notes |
| `growing` | **Growing.** Worked on, still rough. Expect bullets, gaps and views that move. | `🗃/🌻`, `🗃/📖` |
| `evergreen` | **Evergreen.** Own words, tended over time. Still grows, never finished. | `🗃/🌳` |

"Finished" is not a level. Evergreen means alive and maintained, not done (the long, still-growing
`will ai replace humans` note is evergreen). The literature-vs-own-words split (`📖` vs `🌳`)
matters for the Zettelkasten, not for readers, so it collapses into `growing`.

## What it looks like

Mockups: https://claude.ai/artifact/8MLBpiVxnp3j8gT3JCBVcX (artboard I row "I1 plain word" for the
trigger, artboard G for its position at the end of the line, artboard E for lists and link cards).

**Note page.** The level word takes the slot the "recently updated" label has today, at the end of
the meta line, as a plain lowercase word with the same dotted underline. No pill, no glyph, no
colour:

```
Last updated Oct 1, 2026 · Created Oct 1, 2026 · 1 min read · growing
```

- The word reuses the `.note-changes` / `.nc-label` trigger markup and styling, so it reads as
  "hover me" exactly like the label it replaces. The "recently updated" label itself goes away;
  `Last updated` two items earlier already carries recency.
- Unlike today, the trigger renders for **every** note (every note has a level), not only notes
  with recent sessions.
- Hover (desktop) or tap/focus (mobile) opens **one merged popover** that replaces today's
  "Recent changes" popover:

```
NOTE STATUS
Growing. Worked on, still rough. Expect bullets, gaps and views that move.
Estimated from edit history · 1 session · 95 words · published today
How my notes grow →                       (link to /brain/taxonomy-of-note-types)
──────────────────────────────
RECENT CHANGES
Oct 1   today          published · 95 words
Sep 23  8 days ago     +212 / −40 words
…
```

- Manual override: second line reads `Set by author` instead of `Estimated from edit history …`.
- The "Recent changes" section is omitted when `recent_updates.json` has no sessions for the note
  (the status section alone still renders).
- The popover is anchored to the right edge of the trigger (`right: 0` instead of today's
  `left: 0`), since the word now sits at the end of the line and a left-anchored 350px box would
  run past the card on narrow widths. Below 640px, where the wrapped meta line can put the word
  near the left edge, the tap-opened popover is a fixed bottom sheet (`left/right: 1rem;
  bottom: 1rem`) instead.

**Homepage recent-notes list** (`layouts/partials/page-list.html`): the level word appears in the
row's muted meta line, `Oct 1, 2026 · growing`. No pill there; the `NEW`/`UPD` badge already
occupies the badge slot.

**Link hover popover** (`assets/js/popover-v2.js`): the trigger is inside `p.meta.popover-hint`.
The script today removes the whole `.note-changes` element from the extracted hint; it must instead
remove only the popover body (`.nc-popover`) and keep the level word, so a hover card reads
`Updated Sep 14, 2026 · … · growing`.

**Graph, search, RSS:** untouched.

## Architecture

```
vault note                     "Tags: #garden/growing #publish"   (author-only, optional)
   │  obsidian-quartz (copy)   → frontmatter status: growing, status_source: manual
   ▼                             (garden/* removed from tags; emoji tags still stripped)
content/*.md ──┐
               │  python utils/recent_updates.py → data/recent_updates.json   (unchanged)
               │
               ▼
obsidian-quartz note-status  (new Rust subcommand, runs after recent_updates.py in `make prepare`)
   reads: content/*.md (frontmatter status, body word count)
          data/recent_updates.json (sessions)
          config.toml [params] noteStatus* thresholds
   writes: data/note_status.json      → Hugo (pill, popover, list)
           note_status_report.md → author (gitignored, never consumed by Hugo)
```

Everything stays build-time: Hugo bakes the level into HTML, readers download nothing extra, and
nothing is written back to the vault. The git scan is not duplicated: the Rust step consumes the
sessions `recent_updates.py` already produces (keyed by the same filename stem Hugo uses as
`.File.BaseFileName`).

### 1. Manual override: `#garden/*` tags

In `file_utils.rs`, where the bottom `Tags:` line is parsed:

- A tag matching `garden/(started|growing|evergreen)` sets frontmatter `status: <level>` and
  `status_source: manual`, and is dropped from the `tags` list (no Hugo taxonomy page for it).
- Any other `garden/*` value is ignored with a warning printed (`unknown garden level "x" in
  <note>`), not published.
- Existing emoji tags (`📬`, `🗃/🌳`, …) keep being stripped and do **not** map to a level. They are
  stale on many notes; silently promoting them to overrides would bake old mistakes in. The
  one-off calibration (step 4) is where the author decides which notes get a `#garden/*` tag.

### 2. Heuristic: `obsidian-quartz note-status`

Per note, inputs:

| Input | Source |
|---|---|
| `words` | body word count of `content/<note>.md`, frontmatter excluded, same split as `recent_updates.py` (whitespace) |
| `sessions` | `len(sessions)` from `recent_updates.json`; `0` when the note has no entry |
| `span_days` | days between oldest and newest session `iso`; when there is no entry, `lastmod − createddate` from frontmatter; `0` if neither |
| `age_days` | today − `createddate` (fallback: oldest session) |
| `manual` | frontmatter `status` when `status_source == manual` |

Rules, evaluated in order, thresholds from `config.toml`:

```toml
[params]
noteStatusEvergreenWords    = 600   # all three must hold for evergreen
noteStatusEvergreenSessions = 5
noteStatusEvergreenSpanDays = 180
noteStatusStartedWords      = 120   # started when words < this AND sessions <= 1
```

1. `manual` set → that level, `source = manual`.
2. `words ≥ 600 && sessions ≥ 5 && span_days ≥ 180` → `evergreen`.
3. `words < 120 && sessions ≤ 1` → `started`.
4. otherwise → `growing`.

`source = git` for 2–4. Thresholds are a first guess; the calibration run (step 4) tunes them before
the feature ships. Notes with no `recent_updates.json` entry (older than the 5-year lookback with no
edits since) fall to rule 3/4 on words alone, which is the honest answer for a note nobody touched.

Output `data/note_status.json`, keyed by filename stem:

```json
{
  "format usb drive on linux": {
    "status": "started",
    "source": "git",
    "words": 95,
    "sessions": 1,
    "span_days": 0,
    "reason": "1 session · 95 words · published today"
  },
  "will ai replace humans": {
    "status": "evergreen",
    "source": "git",
    "words": 3120,
    "sessions": 23,
    "span_days": 753,
    "reason": "23 sessions over 2 years · 3,120 words"
  }
}
```

`reason` is the human sentence shown in the popover, prebuilt in Rust so the Hugo template stays
dumb. Manual notes get `"reason": "set by author"`.

### 3. Author report: `note_status_report.md`

Written on every `make prepare`, gitignored. Sections:

1. **Summary** — counts per level, how many manual.
2. **Manual tag disagrees with heuristic** — the note, both levels, the heuristic reason. Sorted by
   the size of the disagreement (manual `started` vs git `evergreen` first). This is the "I never
   upgrade my tags" fix: the report nags.
3. **Per level** — every git-classified note with its reason, sorted by words descending, so the
   borderline cases at the bottom of `evergreen` and the top of `growing` are easy to eyeball.

Markdown, not JSON, because it is for reading, and Hugo ignores `.md` in `data/`.

### 4. One-off calibration with Jev (throwaway)

A Python script in the scratchpad (not in `utils/`, not in the Makefile) runs once:

- For every public note, ask Jev one `choice` question (`started` / `growing` / `evergreen`) with
  the three reader-facing sentences as criteria, and one `score` (polish 1–5) on the note body.
- Join with the heuristic output; print notes where Jev's choice ≠ heuristic with confidence
  ≥ 0.7, plus the heuristic reason.
- The author reviews the list, adds `#garden/*` to the notes that are really wrong in the vault,
  and adjusts thresholds if a whole band is off.

After that, Jev is not run again. New notes get git or a manual tag. Requires `JEV_API_KEY`
(the Jev skill is installed; cost is per input token on ~1534 notes, roughly 1.5M tokens).

### 5. Hugo

- `layouts/_default/single.html`: the `.note-changes` block renders whenever the note has an entry
  in `note_status.json` (not only with sessions); its label is the level word; the popover gains
  the status section above the existing sessions list. Existing tap-to-toggle script unchanged.
- `layouts/partials/page-list.html`: ` · <level>` appended to the row meta.
- `assets/js/popover-v2.js`: strip `.nc-popover` instead of `.note-changes`.
- `assets/styles/custom.scss`: `.nc-status*` rows for the new section, popover anchored right,
  dark-mode via existing tokens only. `.update-badge` untouched.
- `config.toml`: the four thresholds, next to the `recentUpdates*` params.
- `Makefile` `prepare`: `obsidian-quartz note-status` right after `python utils/recent_updates.py`.
- `.gitignore`: `data/note_status.json`, `note_status_report.md`.
- README changelog entry.

## Performance budget

A recent Lighthouse pass (`make lighthouse`, `utils/lighthouse.sh`) must not regress. This feature
adds no request, no script and no stylesheet:

- The level and the popover are baked into the HTML at build time from `data/note_status.json`;
  nothing is fetched by the visitor.
- The popover reuses the existing `.note-changes` markup and its inline tap-toggle script; the
  only JS change is one selector in `popover-v2.js`.
- CSS adds a handful of rules to the existing bundle (`custom.scss`), no new file.
- Per page the extra HTML is the status section (~300 bytes). The sessions list already ships
  today for notes with recent edits; notes without sessions gain only the status section.

Verification includes a `make lighthouse P=will-ai-replace-humans/` run before and after,
comparing mobile performance score and total transfer size.

## Edge cases

- Note with manual tag but malformed level → warning, treated as no tag.
- Note renamed → `recent_updates.py` already resolves renames; the Rust step keys by current stem.
- `_index.md` and other `hideLastUpdate` pages → no pill (the whole meta block is skipped today).
- A note whose only sessions touched zero words (`sessions` omitted upstream) → `sessions = 0`,
  words decide.
- Word count counts code blocks. A 400-word shell cheat sheet is `growing`, which is right: it is
  useful and worked on, just not prose.
- `createddate` missing and no sessions → `span_days = 0`, `age_days = 0`; rule 3/4 on words.

## Verification

- Rust unit tests: table-driven cases for the four rules and the `reason` strings
  (`utils/obsidian-quartz/src/note_status.rs`), plus the `garden/*` tag → frontmatter mapping in
  `file_utils.rs` (valid level, unknown level, mixed with `#publish` and normal tags).
- Hugo render fixture in `utils/obsidian-quartz/tests/hugo-render/` with a `data/note_status.json`
  stub: level word present at the end of the meta line, popover has both sections, manual note
  says "Set by author", note absent from the data file renders no trigger, a note without
  sessions renders the status section only. Assertions in `tests/hugo_render.rs`, run by
  `make test`.
- `python -m unittest utils/test_recent_updates.py` stays green (file untouched).
- Manual: `make serve`, open `format usb drive on linux` and `will ai replace humans`, check the
  level word, popover on hover and tap, dark mode, link hover card from another note.

## Out of scope (YAGNI)

- A fourth level, a "kind" axis (lookup vs essay), or a polish score on the site. Jev's polish
  score is for the one-off calibration only.
- Writing `#garden/*` tags back into the vault. Author-only, by hand.
- Status in RSS, search index, or graph colouring.
- Porting the git scan from Python to Rust.
- Updating `content/secondbrain disclaimer.md` and `taxonomy of note types.md` to the new words
  (vault content; the author edits them).
