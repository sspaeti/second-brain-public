# My Public Second Brain

See on [ssp.sh/brain](https://ssp.sh/brain).

This is a fork of the [Quartz](https://github.com/jackyzha0/quartz) repo ([v3](https://github.com/jackyzha0/quartz/tree/hugo) with Hugo). I added some additional features such as:
* Tagging with `#publish` automatically copies the note from my private second brain in [Obsidian](https://obsidian.md) to this public second brain
* Converts the first header (`# my title`) into frontmatter and removes it (as Quartz expects)
* **Smart description extraction** from first paragraph with automatic cleaning:
  - Removes wikilinks, markdown formatting, list markers
  - Intelligent sentence truncation for complete thoughts
  - Displays on OG images as text overlay (Kanagawa color scheme)
  - Used in meta tags for SEO and social media previews
* **BASE file support** for publishing Obsidian database views:
  - Publishes Database Folder plugin `.base` files as standalone Hugo pages
  - Generates HTML tables from database entries with wikilinks
  - Supports folder filters and exclusion patterns
  - Recursive subdirectory scanning
  - Examples: [Coffee Beans](https://ssp.sh/brain/coffee-beans-base), [Books](https://ssp.sh/brain/books-base)
* **Gallery shortcode** (`layouts/shortcodes/gallery.html`): `{{< gallery folder="_img/todays-office/todays-office-recent" >}}` renders all images in a `content/` subfolder as a CSS grid with Lightbox2 click-to-enlarge. Supports `exclude="file1.jpg,file2.jpg"` to skip individual files. Uses `readDir` instead of page resources so it works with the flat `.md` file structure (no page bundles needed). A similar shortcode exists in the blog at `sspaeti-hugo-blog/layouts/shortcodes/gallery.html`, but that one uses `.Page.Resources.ByType "image"` (page-bundle approach). Run `make compress-gallery` to batch-compress gallery JPEGs in-place via ImageMagick.
* YouTube links in Obsidian image syntax (`![title](https://youtube.com/watch?v=XXX)`) render as embedded video players instead of broken images
* Callout blocks are normalized so compact and spaced forms render identically
* **Obsidian transclusions / embeds** (`![[Note#^block-id]]` and `![[Note#Heading]]`): block and section references render inline as a quoted blockquote (instead of a broken image), with nested `[[wikilinks]]` and `![[images]]` inside the embed resolved, plus a floated top-right link back to the source note that jumps to the exact spot — the heading for section refs, or the block's enclosing heading for block refs (hover shows the popover preview)
* **Mermaid → OG image**: set `ogimage: mermaid` (or `mermaid2`, `mermaid3`, …) in a note's frontmatter to render the Nth ` ```mermaid ` block as the social-media preview image (rendered via `mmdc` + ImageMagick to a 1200×630 WebP using a dark theme that matches the site's OG template)
* **Raw Markdown output**: every note is also published as plain Markdown at `/brain/<slug>/index.md`, so LLMs and scrapers can read the Obsidian source without the site chrome (see [Raw Markdown output per note](#raw-markdown-output-per-note))

The content/notes themselves are not published in this repo, only on [ssp.sh/brain](https://ssp.sh/brain).

> **[Explore with RAG → explore.ssp.sh](https://explore.ssp.sh)**
>
> Semantic search, hidden connections, and graph traversal powered by [obsidian-note-taking-assistant](https://github.com/sspaeti/obsidian-note-taking-assistant).

## Utils

### `obsidian-quartz`: Content processing

Rust CLI tool that processes Obsidian vault notes and outputs Hugo-compatible markdown. Handles frontmatter, tags, images, OG image generation, callout normalization, BASE database views, and more.

Key features:
- **Markdown publishing**: Processes notes tagged with `#publish`
- **BASE database views**: Publishes Obsidian Database Folder plugin `.base` files as HTML tables
- **Filter expressions**: Supports folder filters and exclusion patterns (`!file.path.contains`)
- **Smart descriptions**: Auto-extracts clean descriptions from first paragraph
- **OG image generation**: Creates social media preview images with SVG→WebP conversion
- **Mermaid OG images**: Renders a note's Mermaid diagram as its OG image via `ogimage: mermaid` / `mermaid<N>` (uses `mmdc` + ImageMagick, dark theme matches the site's OG template)

See **[utils/obsidian-quartz/README.md](./utils/obsidian-quartz/README.md)** for details.

### `hugo-obsidian`: Backlink and graph creation

The tool used is `hugo-obsidian`, a small Go program written by Jacky. Here's the [source](https://github.com/jackyzha0/hugo-obsidian). It is not maintained anymore (as there is now a [v4](https://github.com/jackyzha0/quartz/tree/v4) without it) and it had bugs and didn't show all my backlinks. That's why I forked it and fixed the backlinks. You can find it here: [sspaeti/hugo-obsidian](utils/hugo-obsidian).

It scans the `content/` folder for wikilinks and emits two artifacts Hugo consumes to render the interactive graph and per-note backlink lists:
- `assets/indices/linkIndex.json` — every `[[wikilink]]` as a `source → target` edge, lowercased and de-duplicated (powers the graph and "Links to this note" sections)
- `assets/indices/contentIndex.json` — slug → title/content map used for search and link previews

Key fork additions over upstream: case-insensitive link matching, block-reference (`^hash`) handling, slash-in-title normalization, and performance tuning for large vaults.

> [!NOTE]
> `sspaeti/hugo-obsidian` has been integrated directly in this repository at [utils/hugo-obsidian](utils/hugo-obsidian). See **[utils/hugo-obsidian/README.md](./utils/hugo-obsidian/README.md)** for installation, CLI flags, and the full changelog.

### Hugo render hooks

Custom render hooks in `layouts/_default/_markup/`:
* **render-image.html** - Detects YouTube URLs and renders responsive iframe embeds (with timestamp support); all other images pass through normally

### `recent_updates.py`: Change badges & edit history

Stdlib-only Python (`utils/recent_updates.py`, logic ported from the newsletter generator) that scans the `content/` git submodule history **plus its uncommitted working tree** and writes `data/recent_updates.json`, keyed by each note's on-disk filename stem (== Hugo `.File.BaseFileName`). Each entry is `{status, words, sessions:[…]}` and drives two UI features:
- **Recent-notes pills** — a green `NEW · 1,079w` / blue `UPD · ~80w` badge on the homepage recent-notes list (and other listings).
- **Per-note edit history** — a "recently updated" hover popover on each note page listing recent editing sessions with `+added / −removed` word counts. The note's origin session shows `published · N words` when it carries a frontmatter `createddate` (its true creation is already in the meta "Created" line, so the first git commit is the *publish* event — this also holds when it was created and published on the same day), or `new · N words` when it has no `createddate` (born straight on git).

Because it also reads the working tree, a freshly prepared note gets its badge/popover **before** the `content/` submodule is committed: untracked notes count as brand-new creations dated "now", and a tracked note's uncommitted diff folds in as a "now" session (`git diff HEAD`). This decouples the badges from commit timing — previously a just-added note showed nothing until the *next* deploy.

**Two noise filters keep the counts "real content only"** (so tooling/metadata commits don't show as edits):
- **Frontmatter excluded from word counts** — the parser tracks file line numbers through each diff hunk and ignores any `+`/`-` line inside the `---…---` block. So OG `description:` backfills, `createddate:` extraction, and `lastmod:` bumps contribute 0 words.
- **`lastmod` ceiling** — sessions dated *after* a note's frontmatter `lastmod` are dropped. `lastmod` is the pipeline's authoritative "real edit" date (set by `obsidian-quartz` from vault mtime, guarded by `revert-lastmod-only.sh`), so the popover never shows a change newer than the note's "Last updated" line. A note whose only recent git activity is tooling gets no dot. **Two exceptions**: (a) a note's **creation session is never ceilinged** — a note must always show when it was born (and keep its `NEW` badge) even if a later commit that never bumped `lastmod` folded into that session, or the whole note would vanish; (b) a note with a **live uncommitted edit** raises its ceiling to today, since `lastmod` isn't bumped until that edit is committed.

The homepage badge picks the most recent session that is a real content change (or the creation), so a trailing metadata commit never makes it read `0 words`.

Runs in `prepare` / `prepare-python` (one line, no other build change). Tunables live in `config.toml` `[params]` (read via `tomllib`, with in-script fallbacks): `recentUpdatesLookbackDays` (1825 ≈ 5y — history-depth only, no viewer cost since each popover is capped at `recentUpdatesMaxSessions` rows and bounded by `lastmod`), `recentUpdatesSessionGapHours` (24), `recentUpdatesMaxSessions` (7).

## Configs

### Raw Markdown output per note

Every note is published twice: as HTML, and as plain Markdown at `/brain/<slug>/index.md`. The Markdown is the Obsidian source, so wikilinks stay as `[[Cal Newport]]` and callouts stay as `> [!note]`.

```bash
curl https://www.ssp.sh/brain/deep-life/index.md
```

Two pieces, both local additions (not from upstream Quartz):

- **`config.toml`** — `[mediaTypes]` registers the `md` suffix, `[outputFormats.MarkDown]` defines the format, `[outputs]` adds it to the `page` kind. Only `page` is overridden, so `home` / `section` / `taxonomy` / `term` keep Hugo's defaults and the RSS feed at `/brain/index.xml` plus all HTML output are untouched.
- **`layouts/_default/single.md`** — the template, `# {{ .Title }}` followed by `{{ .RawContent }}`.

> [!WARNING]
> `config.toml` is in the upstream checkout list in the `Makefile` (`git checkout upstream/hugo -- … config.toml …`), so a Quartz sync drops the three blocks. They carry a `LOCAL ADDITION` comment; re-add them after any sync. `layouts/` is in that list too, but `single.md` is a new file, so a checkout of tracked paths leaves it alone.

Verified on build: 675 `.md` files generated, `/brain/index.xml` intact, `sitemap.xml` unchanged at 696 URLs with no `.md` entries, no `.md` under `public/tags/`.

The files are not linked from anywhere and are absent from the sitemap by design. They are advertised only in the blog's [`static/llms.txt`](https://www.ssp.sh/llms.txt), which documents the `index.md` convention. Deploy is automatic, the `upload` target rsyncs `public/` with no `.md` exclude. The same setup exists in `../sspaeti-hugo-blog`.

### Redirects of renamed files
Find these in [.htaccess](static/.htaccess)

## ChangeLog

### 2026-08-21: Embeds create backlinks; heading transclusions drop the footer

Two fixes around Obsidian embeds (`![[…]]`):

- **Embeds now count as backlinks** (`utils/hugo-obsidian/parse.go`): an embed
  like `![[Note#Heading]]` never appeared in the link index — goldmark's image
  parser eats the leading `![`, so the wikilink extension never saw it and no
  `<a>` was produced. Fixed by rewriting `![[` → `[[` before conversion, for
  link extraction only. Image embeds (`![[img.webp]]`, PNG/JPG/…) are still
  excluded by the existing extension `filter()`, so nothing new leaks into the
  graph. Full before/after diff of `linkIndex.json`: 0 links removed, 34
  note-embed links added. See
  [utils/hugo-obsidian/README.md](./utils/hugo-obsidian/README.md) changelog.
- **Heading transclusions strip the note footer**
  (`layouts/partials/textprocessing.html`): the `---` +
  `Origin:/Source:/References:` footer strip (and stray `^blockid` cleanup)
  only ran for whole-note embeds. A heading reference whose section runs to the
  end of the note (no later same-or-higher heading) dragged the footer into the
  transcluded quote. The heading-reference branch now applies the same two
  strips as the whole-note branch.

### 2026-08-18: Frontmatter aliases publish redirects again

`aliases:` in a note's frontmatter now produces working URLs. `dag.md` with
`aliases: Directed Acyclic Graphs, Directed Acyclic Graphs (DAGs)` publishes
`/brain/directed-acyclic-graphs` and `/brain/directed-acyclic-graphs-dags`, both
redirecting to `/brain/dag`. 293 alias URLs across 189 notes, generated on every
`make prepare` — no more hand-maintaining `static/.htaccess` for renames that a
note already declares an alias for.

- **Why it was off**: Obsidian writes aliases as ONE comma-separated scalar
  (`aliases: OLAP, OLAP Cubes`). Hugo casts a scalar via `strings.Fields`, i.e.
  splits on **whitespace**, so that line published `/OLAP/`, `/OLAP,/` and
  `/Cubes/`. Hugo writes an alias page with no conflict check, so on a
  case-insensitive filesystem `/OLAP/` overwrote the real `olap` note with a
  redirect. Hence `disableAliases = true`.
- **Hugo never slugifies an alias** — it uses the string verbatim and ignores
  `disablePathToLower`. A correct YAML list alone would publish
  `/brain/Directed Acyclic Graphs/`, so the canonical slug rules have to be
  applied before Hugo sees them.
- **What changed**:
  - `utils/obsidian-quartz/src/slug.rs` (new) — the crate's single copy of
    `UnicodeSanitize`, moved out of `enrich_with_blog.rs` (which re-exports it,
    so `merge_search_index` / `enrich_with_memories` are untouched). Adds
    `brain_slug()` for one URL segment: lowercase, fold `/` and `\` to `-`,
    sanitize. Unit tests pin it to real published note URLs.
  - `file_utils.rs` — splits the alias scalar on `,`, slugs each entry, dedupes,
    and emits a real YAML list (`aliases: [a, b]`, flow style like `tags`). The
    key is matched case-insensitively: `aws s3.md` used `Aliases:`, which Hugo
    honours and an exact-match lookup would have skipped.
  - Self-collisions dropped while writing: `olap.md` aliasing `OLAP` would
    overwrite itself with a redirect. 8 notes lost their alias line this way
    (`quartz`, `todays office`, `pl-sql`, …); `olap` keeps `olap-cubes`.
  - `file_utils.rs::resolve_alias_collisions` — post-pass over `content/` once
    every note exists, since a clash between two notes can't be seen one file at
    a time. Real notes always beat aliases; between two notes the first in sorted
    filename order wins.
  - **A collision fails the build.** The losing alias is stripped from `content/`
    first (so Hugo can never overwrite a real note with a redirect), then
    `make prepare` exits non-zero, which stops `make serve` and `make deploy`
    before `hugo-generate`/`upload` run. An alias is a URL someone may already
    have linked — dropping it and deploying anyway is how a URL dies unnoticed.
    The error names both notes and the contested `/brain/` path. Fix it in the
    vault (`content/` is regenerated every build): drop the alias on the losing
    note, or rename the note that took the URL. Real example: adding a note
    `OLAP Cubes.md` collides with `OLAP.md`'s `OLAP Cubes` alias, since the new
    note now owns `/brain/olap-cubes`.
  - `config.toml` — `disableAliases = false`.
- **Tradeoff**: alias pages are meta-refresh + `<link rel="canonical">`, not
  301s. `static/.htaccess` still holds the hand-written 301s for renames where
  no alias exists. Where both cover the same URL (5 today, e.g.
  `learning-in-public`) Apache rewrites first, so the 301 wins.
- **Files**: `utils/obsidian-quartz/src/slug.rs`, `file_utils.rs`,
  `enrich_with_blog.rs`, `main.rs`, `config.toml`

### 2026-08-14: Raw Markdown output per note

Every note is now published a second time as plain Markdown at `/brain/<slug>/index.md`, matching what the blog has been doing at `/blog/<slug>/index.md`. The point is LLM and scraper access to the source text without nav, graph, backlinks and footer around it.

- **`config.toml`**: added `[mediaTypes."text/plain"]` (`suffixes = ["md"]`), `[outputFormats.MarkDown]`, and `[outputs] page = ["HTML", "MarkDown"]`. Only the `page` kind is overridden so every other kind keeps Hugo's defaults. Marked `LOCAL ADDITION` in the file, since the `Makefile` upstream-sync targets check `config.toml` out from `upstream/hugo`.
- **`layouts/_default/single.md`**: new two-line template, `# {{ .Title }}` + `{{ .RawContent }}`.
- **Output is the Obsidian source**: `.RawContent` runs before `textprocessing.html`, so wikilinks, transclusions and callouts appear in their raw form. Frontmatter is stripped, the title is re-added as an H1.
- **Verified**: 675 `.md` files, `/brain/index.xml` intact (575 KB), `sitemap.xml` still 696 URLs with 0 `.md` entries, 0 `.md` under `public/tags/`, sampled `deep-life/index.html` renders normally.
- **Not discoverable by crawl**: no sitemap entry (a second format of the same content muddies duplicate-content signals) and no `<link rel="alternate">` in the page head. The convention is documented in the blog's `static/llms.txt`.
- **Files**: `config.toml`, `layouts/_default/single.md`.

### 2026-08-11: Change-badge fixes — new notes, creation word counts, publish label

Four fixes to `utils/recent_updates.py` after new notes (e.g. *terminal multiplexer*) showed no `NEW` badge and creation rows read `· 0 words`. Root causes were independent despite surfacing together on fresh notes.

- **Uncommitted content now counts** (`worktree_word_stats`): the generator ran in `prepare` *before* the `content/` submodule was committed, so a just-added note had no git history and produced no JSON entry (no badge, no popover) until the next deploy. It now also scans the working tree — untracked notes as brand-new creations dated "now", tracked edits via `git diff HEAD` folded in as a "now" session. No forced commit; parser is shared with the git-log path.
- **Creation word counts fixed** (`seen_hunk` flag): a file-creation hunk header is `@@ -0,0 +1,N @@`, so `old_ln` stays `0` for the whole hunk and the old `if old_ln == 0: continue` preamble guard silently dropped *every* added line → creation rows read `0 words`. The guard now gates on "have we passed the first `@@` yet", counting the added body of new files. (Notes committed empty and filled later still read `0` — correct.) Cut zero-word creation rows 551 → 8.
- **Creation session exempt from the `lastmod` ceiling**: a real content commit newer than a stale `lastmod` (edit that didn't bump the date) was ceilinged away; when it had merged into the creation session, the whole note disappeared — losing its `NEW` badge *and* popover. The creation session is now always kept; live-edited notes also raise their ceiling to today. The tooling-noise ceiling is unchanged for every other session (verified: 0 post-`lastmod` non-creation rows leak).
- **`published` vs `new` label widened**: the origin row now reads `published` whenever a `createddate` exists (true creation already shown in the meta line), not only when it strictly *predates* the first commit. Same-day create-and-publish notes (155 of them) now read `published` instead of `new`. Homepage `NEW` badge is a separate axis and is unaffected.
- **Files**: `utils/recent_updates.py`.

### 2026-08-07: Change badges + per-note edit history (git word-diffs)

The recent-notes list and every note page now show *how much* a note changed, from the `content/` git history at build time. Recent-notes pills read green `NEW · 1,079w` (brand-new) or blue `UPD · ~80w` (gross words in the last edit); a note page adds a "recently updated" dot whose hover/tap popover lists up to 5 recent editing sessions with `+added / −removed` counts — no commit messages.

- **`utils/recent_updates.py`** (stdlib, ported from the newsletter generator): one `git log` scan → `data/recent_updates.json`, keyed by filename stem (== Hugo `.File.BaseFileName`), each `{status, words, sessions}`. Runs as one line in `prepare` / `prepare-python`. Tunables: `LOOKBACK_DAYS` 180, `SESSION_GAP_HOURS` 24, `MAX_SESSIONS` 5.
- **Sessions**: commits ≤ 24h apart collapse into one, so a same-afternoon burst reads as a single change. Badge size is *gross* (added + deleted); `new` vs `updated` from whether the note's first commit falls in that session.
- **Rendering**: pills in `layouts/partials/page-list.html` (shared, so they also show on tag/section/taxonomy listings); popover in `layouts/_default/single.html`, pure-CSS via `:hover` / `:focus-within`, all-`<span>` to stay valid inside `<p>`.
- **Colors** (`assets/styles/custom.scss`, Kanagawa): `--badge-new` `#76946A`, `--badge-upd` `#658594` (= dark `--secondary`), `--badge-del` `#C34043`; ASCII-only chunk top to avoid the Sass-BOM bug.
- **Newsletter footer** (`layouts/partials/newsletter-footer.html`): now takes optional `label` / `desc` (`safeHTML`), appended below the recent list via `recent.html`; default callers unchanged.
- **Files**: `utils/recent_updates.py`, `layouts/partials/page-list.html`, `layouts/_default/single.html`, `layouts/partials/recent.html`, `layouts/partials/newsletter-footer.html`, `assets/styles/custom.scss`, `Makefile`, `.gitignore`.

### 2026-08-07: Per-note change popover + edit-history refinements

Follow-up to the badges above: the "recently updated" dot on note pages got a hover/tap popover of recent editing sessions, plus several accuracy fixes so it reflects *real content* changes only.

- **Real-content-only counts**: word diffs now **exclude the frontmatter block** (line-number tracking through each hunk), so OG `description:`, `createddate:` extraction, and `lastmod:` bumps count as 0. Sessions are also **ceilinged at the note's `lastmod`** — the popover never shows a change newer than the "Last updated" line, since `lastmod` is the authoritative real-edit date (`obsidian-quartz` from vault mtime + `revert-lastmod-only.sh`). A note whose only recent git activity is tooling shows no dot.
- **Creation row**: the session containing a note's first commit renders `new · N words` (matching the badge) instead of churn — or `published · N words` when frontmatter `createddate` predates the first git commit (note written privately, published later), so it doesn't compete with the meta "Created" date. The badge picks the most recent *content* session so a trailing metadata commit never reads `0 words`.
- **Lookback 180 → 1825 (≈5y)**, `MAX_SESSIONS` 5 → 7 — shows a note's multi-year refinement history. No viewer cost (baked into HTML, popover capped at `MAX_SESSIONS` rows, bounded by `lastmod`); only the local build scan grows (~1.7s). The three tunables now live in `config.toml` `[params]` (`recentUpdates*`), read by the script via `tomllib`.
- **Popover UX** (`layouts/_default/single.html`, `assets/styles/custom.scss`): pure-CSS `:hover` / `:focus-within` on desktop; a small inline script toggles `.nc-open` for tap on touch devices (tap-outside / `Escape` to dismiss). Solid opaque surface (`--nc-bg` `#16161D` dark / `#fff` light) at `z-index:40` — fixes the theme's `article > .meta { opacity: .7 }` which made the popover translucent *and* trapped it under the article body (overridden to `opacity:1`, meta re-muted via color). `+added`/`−removed` in `--badge-new`/`--badge-del`.
- **Files**: `utils/recent_updates.py`, `layouts/_default/single.html`, `assets/styles/custom.scss`, `config.toml`.

### 2026-08-04: `lastmod` no longer bumps when a note's content is unchanged

`obsidian-quartz` sets each note's `lastmod` to `max(existing lastmod, source file mtime)`. Obsidian bumps a file's mtime on *every* save — including an accidental edit that was undone back to identical content — so a note's date would jump to today with no real change. Because `content/` is its own git submodule, a note whose only diff vs `HEAD` is the `lastmod:` line had no real change and can be restored.

- **Post-processing step** (`utils/revert-lastmod-only.sh`): runs in `prepare` right after `obsidian-quartz` regenerates `content/`. For each changed `.md`, it strips diff headers and the `lastmod:` line from `git diff -U0`; if nothing else changed, the note is restored to its committed state.
- **Self-correcting**: the next `make` bumps mtime and `lastmod` again, this step reverts it again — the date stays pinned to the committed value until a *real* content edit lands, at which point `lastmod` updates and gets committed normally.
- **Lock-free restore**: uses `git show HEAD:<path> > <path>` instead of `git checkout`, so it writes only the working-tree file and never grabs the submodule `index.lock` — avoids a `fatal: Unable to create '.git/modules/content/index.lock'` race against concurrent `lazygit` / `gitstatusd` watchers on the submodule.
- **Scope**: only acts on files git already flags as modified inside `content/`; new/untracked notes keep their fresh `lastmod`. If tool formatting ever changes globally, those notes show real-line diffs and are correctly kept.
- **Files**:
  - `utils/revert-lastmod-only.sh` — the revert step
  - `Makefile` — `bash utils/revert-lastmod-only.sh` in `prepare`, after `obsidian-quartz`

### 2026-08-04: Obsidian block & heading transclusions render inline

Obsidian embed transclusions — `![[Note#^block-id]]` (block reference) and `![[Note#Heading]]` (section reference) — used to fall through the image-embed branch and render as a broken `<img>`. They now render inline as a quoted blockquote pulled from the source note, matching how they look in Obsidian.

- **How it works** (`layouts/partials/textprocessing.html`): a new transclusion pre-pass runs *before* the wikilink loop. For each `![[...#...]]` it resolves the target via `GetPage`, reads its `RawContent`, and extracts:
  - **Block ref (`#^id`)** — the paragraph carrying that block id (the `^id` marker is stripped).
  - **Heading ref (`#Heading`)** — everything under the heading until the next same-or-higher heading (deeper subheadings are included).
  - The extracted markdown is rendered with `RenderString` and wrapped in a `<blockquote>`.
- **Nested content resolves for free**: because the pre-pass injects the embedded content *before* the wikilink/image loop computes its matches, any `[[wikilinks]]` and `![[images]]` inside the embedded block/section get resolved by the existing machinery (e.g. an image inside an embedded section becomes a real `<img>`).
- **Back-link affordance**: each embed gets a small arrow-icon link (`.transclusion-link`) floated to the top-right of the quote — the text wraps around it. It carries `data-src` so the hover popover preview works, and it jumps to the exact spot: heading refs link straight to the target's heading anchor (`/brain/<note>#<heading>`), and block refs link to the block's enclosing heading (tracked during extraction; falls back to the note top if the block sits above any heading).
- **Graceful fallback**: an unpublished target note renders as a broken-styled link; a published note whose referenced block/section isn't published renders as a plain working link to the note. The raw `#^id` is never leaked into the visible label.
- **Callout-safe**: the injected `<blockquote>` is intentionally class-less, so the existing callout normalization (which tags every bare `<blockquote>` as `callout` and walks them in order) stays aligned and treats it as an ordinary quote.
- **Block-id placement**: an `^id` marker can sit inline at the end of a block, or on its own line *below* the block (Obsidian allows both, including across a blank line). Extraction tracks the current and previous block so the standalone-below form resolves to the block above it. Fenced code blocks (e.g. Mermaid) are tracked too, so the blank lines inside a fence don't split the block — a `^id` under a diagram captures the whole diagram, not a fragment.
- **Edge cases**:
  - If the referenced block is *itself* a `> ` blockquote or a `> [!callout]`, its leading `>` and any `[!marker]` are stripped before rendering, so it shows as our single quote instead of a quote-inside-a-quote or a literal `[!important]`.
  - The embed is injected as `</p><div class="transclusion">…</div><p>` so the block-level quote becomes a proper *sibling* paragraph rather than a block nested inside a `<p>` (which the browser would auto-close, orphaning any adjacent text out of its paragraph and stripping its normal font/color/spacing). A cleanup pass after the loop drops the leftover leading/trailing `<br>` and empty `<p></p>`. Safe because no embed in the vault sits inside a list item or blockquote — every one is in paragraph context.
- **Files**:
  - `layouts/partials/textprocessing.html` — transclusion pre-pass + source link
  - `assets/styles/custom.scss` — `.transclusion` / `.transclusion-link` styles (float top-right; ASCII-only so no Sass BOM issue)

### 2026-07-31: Interactive graph shows memories (photo feed)

The local graph now surfaces a fourth node type: **memories** from the blog's photo feed at [ssp.sh/memories](https://ssp.sh/memories). When a memory caption references a brain note via `[[wikilink]]`, viewing that note shows the memory as a green node — so a note like *Travel where You Are* surfaces the photos that reference it. Extends the [2026-06-01](#2026-06-01-interactive-graph-shows-blog-posts-and-book-chapters) blog/book graph work.

- **Node type**: **Memories** (`#98bb6c` Kanagawa springGreen) — memory at `ssp.sh/memories/<slug>`. Color, legend label, and filter are driven by the `paths:` entry in `data/graphConfig.yaml`, exactly like blog/book — no per-type code in `graph.js` beyond a prefix branch.
- **Click behavior**: opens the memory in a new tab (external, like blog/book); brain nodes keep SPA navigation.
- **Data flow**: a new `obsidian-quartz enrich-with-memories` subcommand reads `../sspaeti-hugo-blog/content/memories/*/index.md` directly (no hugo-obsidian scan — the blog only indexes `content/posts`), extracts `[[wikilinks]]` (handles `[[Target|Display]]` aliases, skips `_index`, reuses `enrich_with_blog::unicode_sanitize` so slugs match hugo-obsidian), and injects memory→brain edges + `type:"memory"` nodes into `linkIndex.json` / `contentIndex.json`. Same one-directional enrich pattern as `enrich-with-blog` / `enrich-with-book`.
- **Search stays clean**: the step runs *after* `merge-search-index` in `prepare` (same trick as `enrich-with-book`), so memories live in the graph only — never in `Ctrl+K` search.
- **Additive & idempotent**: only inserts new `/memories/*` nodes/edges; existing brain/blog/book ids and edges are untouched. Wikilink targets that aren't published brain notes are silently skipped (no dangling nodes). Re-running adds 0.
- **Files**:
  - `utils/obsidian-quartz/src/enrich_with_memories.rs` — the importer (registered in `main.rs`)
  - `assets/js/graph.js` — `/memories/` click routing, legend label, filter + prefetch guards
  - `data/graphConfig.yaml` — `/memories/` node color via `paths:`
  - `Makefile` — `obsidian-quartz enrich-with-memories` at the end of `prepare` (after `merge-search-index`)

### 2026-06-28: Unified search v2 — blog + brain in one modal

A full-screen FlexSearch modal that searches blog posts (ssp.sh) and brain notes (ssp.sh/brain) together. Triggered by `Ctrl+K` or `/`. Results show source badges (blog / brain), dates, and a highlight of the matching excerpt. Blog entries float to the top; filter buttons narrow by source or date.

**Toggle**: `searchVersion` in `data/config.yaml` — `v2` = unified (default), `v1` = original brain-only FlexSearch.

**Index**: A merged `searchIndex-v2.json` is generated at build time by `obsidian-quartz merge-search-index`. It combines brain's `contentIndex.json` with the blog's `contentIndex.json`, enriching each entry with `source`, `created`, `updated` (from frontmatter `createddate:` / `lastmod:`), and `tags`. The file is written to `assets/indices/` (brain) and synced to `../sspaeti-hugo-blog/static/indices/` automatically.

**Date accuracy**: Brain note dates are read directly from frontmatter (`createddate:` / `lastmod:`) — not from file mtime, which is always today after Obsidian copies.

**Navigation**: Brain results use SPA (`Million.navigate`) with the `/brain` prefix stripped (since `BASE_URL` already includes it); blog results always use plain `href`.

**Files**:
- `assets/js/full-text-search-v2.js` — self-contained FlexSearch IIFE with shims for `removeMarkdown`/`highlight` (brain has them from `util.js`; blog does not)
- `layouts/partials/search.html` — 3-way branch: v2 → semantic → v1
- `assets/styles/base.scss` — `#search-filters`, `.source-badge`, `.result-date` styles
- `utils/obsidian-quartz/src/merge_search_index.rs` — Rust subcommand; walks `content/` for frontmatter dates
- `Makefile` — `obsidian-quartz merge-search-index` runs in both `prepare` and `prepare-python` targets

### 2026-06-24: Gallery shortcode for image folders

`{{< gallery folder="_img/todays-office/todays-office-recent" >}}` in any note renders a CSS grid of lazy-loaded thumbnails with Lightbox2 click-to-enlarge.

- **Shortcode** (`layouts/shortcodes/gallery.html`): takes a `folder` path relative to `content/` and an optional `exclude` parameter (comma-separated filenames). Uses Hugo's `readDir` to list images — no page bundle required, so it works with the brain's flat `.md` structure. Lightbox2 CSS/JS loaded from CDN once per page via `.Page.Scratch`.
- **CSS** (`assets/styles/custom.scss`): `.gallery-grid` — `auto-fill minmax(180px, 1fr)` grid, `aspect-ratio: 4/3`, `object-fit: cover`, hover scale.
- **Image folder**: `content/_img/todays-office/` with subdirs `todays-office-recent/`, `todays-office-archive/`, `todays-office-older/`, `chronology-desk/`, `micro-journal-pics/` (renamed from originals to remove spaces/apostrophes for clean URLs).
- **Compression**: `make compress-gallery` batch-resizes JPEGs in-place via ImageMagick (≤1920px, q78, strip EXIF).
- **Blog parallel**: `sspaeti-hugo-blog/layouts/shortcodes/gallery.html` does the same thing but via `.Page.Resources.ByType "image"` (page-bundle approach). The brain shortcode uses `readDir` instead since the brain doesn't use page bundles.

### 2026-06-03: Hover popover previews + scoped CORS for cross-site embedding

A fetch-based hover popover replaces the legacy `popover.js`. Hovering any internal link opens a scrollable preview of the destination note (title, meta line including reading time, full content) rendered from the actual brain HTML — no precomputed JSON index, so notes always show current content. The same JS/CSS is consumed by the [blog](https://www.ssp.sh) and the [DEDP book](https://www.dedp.online) via build-time copy, giving readers on those sites the same brain-link hover experience.

- **Popover implementation** (`assets/js/popover-v2.js`, `assets/css/popover-v2.css`):
  - Selector parametrized via `window.initPopoverV2({selector: "..."})`. Brain uses the default `a.internal-link[href]`; blog/book pass `a[href*="ssp.sh/brain"]`.
  - Branches on `Content-Type`: HTML extracts elements with class `popover-hint` (title, meta, content body — marked in `single.html` / `textprocessing.html` so site chrome, TOC, tags, footer, graph, backlinks are excluded); image responses render `<img>`; PDF responses embed `<iframe>`.
  - Anchor scroll: links to `#heading` scroll the inner div to the prefixed `#popover-internal-<heading>`.
  - Per-pathname cache; outer 1rem padding + `:hover` keeps the popover open while the cursor crosses from the link to the popover for scrolling.
  - Behind feature flag `enableLinkPreviewV2` in `data/config.yaml`. The v1 popover code path remains available for rollback.
  - Light + dark mode rules cover brain (`[saved-theme="dark"]`), blog (uBlogger `body[theme="dark"]`), and all 9 book themes (`html.ayu`, `html.burgundy`, `html.coal`, `html.kanagawa`, `html.light`, `html.navy`, `html.pinkrose`, `html.rust`, `html.tokyonight`) — book uses `var(--bg)` / `var(--fg)` / `var(--links)` / `var(--inline-code-color)` so the popover adapts to whichever theme the reader picks.
- **CORS** (`static/.htaccess`): `SetEnvIf Origin "^https://(www\.)?(ssp\.sh|dedp\.online)$"` + `Header always set Access-Control-Allow-Origin` + `Header always merge Vary "Origin"`. The `always` keyword is required so headers apply to 3xx canonical-host redirects (otherwise the browser blocks the redirect before the final 200). After deploy, the Bunny brain pull-zone cache must be purged once to evict pre-CORS entries.
- **Image wikilinks** (`layouts/partials/textprocessing.html`, `utils/obsidian-quartz/src/file_utils.rs`): `[[image.ext]]` (without `!`) now renders as a working `<a href>` (was a broken `<a>` with no href). The rust util's regex relaxed from `\[\[...\]\]` requiring `!` to `!?\[\[...\]\]`, so the asset gets copied to public output regardless of which form is used. Combined with the popover's image branch, hovering an `[[img.webp]]` link shows the image directly in the popover.
- **Reading time**: `layouts/_default/single.html` now renders `{{ .ReadingTime }} min read` in the meta line for both the page and the popover preview.
- **Files**:
  - `assets/js/popover-v2.js`, `assets/css/popover-v2.css` — canonical popover (copied verbatim to blog and book by their `sync-popover` Makefile targets, alongside `floating-ui.core.umd.min.js` and `floating-ui.dom.umd.min.js`)
  - `layouts/partials/head.html` — flag-gated v1/v2 toggle, loads CSS + JS, calls init
  - `layouts/partials/textprocessing.html`, `layouts/_default/single.html` — `popover-hint` markers on title/meta/content; non-`!` image wikilink handling; reading time in meta
  - `static/.htaccess` — scoped CORS for `(www.)?(ssp.sh|dedp.online)` with `Header always`
  - `utils/obsidian-quartz/src/file_utils.rs` — image-copy regex catches non-`!` wikilinks
  - `data/config.yaml` — `enableLinkPreviewV2: true`

### 2026-06-01: Interactive graph shows blog posts and book chapters

The local graph on every brain note now surfaces connections to two sister sites — the [blog](https://ssp.sh/blog) and the [DEDP book](https://www.dedp.online) — alongside brain↔brain wikilinks. This makes the second brain a true hub: every note shows what posts cite it and which book chapters reference it.

- **Node types** (legend shown on every local graph that has connections):
  - **Current** (rose) — the page you're on
  - **Note** (orange) — brain note linked via wikilink
  - **Blog** (`#60a5fa` blue) — blog post at `ssp.sh/blog/<slug>`
  - **Book** (`#a78bfa` purple) — DEDP book chapter at `dedp.online/<path>.html`
  - **`···` Outgoing** — dotted line marks brain→external edges (e.g., the brain note has `[text](https://ssp.sh/blog/X)`). Incoming edges from blog/book stay solid.
- **Click behavior**: blog and book nodes open in a new tab, brain nodes use the existing SPA navigation. Search (`Ctrl+K`) excludes blog and book entries — they live in the graph only.
- **Edge direction styling**: outgoing brain→external is dotted, incoming external→brain is solid, brain↔brain is solid.
- **Data flow**:
  - Blog repo's `helper-scripts/enrich-link-index.py` already populates `static/indices/linkIndex.json` with brain↔blog edges (both directions).
  - DEDP book's `utils/dedp-link-index` Rust CLI generates `linkIndex.js` (`window.DEDP_LINK_INDEX = {...}`) with chapter→brain edges (`type: "brain"`, target `brain:<slug>`).
  - Two new `obsidian-quartz` subcommands import these into the brain's indices: `enrich-with-blog` and `enrich-with-book`.
- **Frontmatter URL resolution for blog**: blog posts use `url: /blog/X` frontmatter overrides — the script parses each `index.en.md` / `index.md` and extracts the actual URL instead of guessing from the folder name.
- **Idempotency**: re-running `make prepare` adds 0 new edges if nothing has changed; nodes with the same slug as a brain note are never overwritten (`Map.entry().or_insert_with` semantics).
- **Files**:
  - `utils/obsidian-quartz/src/enrich_with_blog.rs`, `enrich_with_book.rs` — the importers
  - `assets/js/graph.js` — click routing, dasharray on outgoing edges, conditional legend
  - `assets/js/full-text-search.js` — skip `/blog/` and `/book/` keys
  - `data/graphConfig.yaml` — node colors via `paths:`
  - `Makefile` — chains `make -C ../sspaeti-hugo-blog prepare` and `make -C ../../book/dedp link-index` before each enrich step (leading `-` so a missing sister repo is non-fatal)

### 2026-05-24: Mermaid diagrams as OG images
- **Mermaid OG rendering** (`utils/obsidian-quartz/src/svg_generator.rs`, `file_utils.rs`):
  - Set `ogimage: mermaid` in note frontmatter to use the first ` ```mermaid ` block as the social-media preview image. Use `mermaid2`, `mermaid3`, … to target later blocks.
  - Pipeline: extract block → render via `mmdc` (dark theme, `#1F1F28` background matching the OG template, 3× puppeteer scale for crisp anti-aliasing) → composite onto a 1200×630 canvas with 50px padding via ImageMagick (Lanczos filter, WebP quality 92).
  - Output written to `content/_img/feature/mermaid/<slug>.webp` and the frontmatter `ogimage` value is rewritten to the resolved path so the existing `head.html` resolver works unchanged.
  - On failure (block missing, `mmdc` errors) the `ogimage` key is dropped so the title-based generator takes over as a fallback.
  - Existing-file cache: re-renders only when the target WebP doesn't already exist (delete it to force a refresh).
- **Hugo template** (`layouts/partials/head.html`):
  - Added `/mermaid/` to the `summary_large_image` Twitter-card detection (alongside the existing `/gen/` rule).
- **External dependencies**: `@mermaid-js/mermaid-cli` (`mmdc`, Node + Chromium); ImageMagick (`magick`).

### 2026-04-17: Obsidian BASE file support for database views
- **BASE File Publishing** (`utils/obsidian-quartz/src/base_*.rs`):
  - Added support for Obsidian Database Folder plugin `.base` files
  - Parses BASE YAML files with filters, views, properties, and formulas
  - Implements temporary staging workflow to avoid private vault scanning:
    - Copies source files to `/content/BASES/<base-name>/` during processing
    - Queries from staging folder to build tables
    - Cleans up staging folder after generation
  - **Filter Support**:
    - Folder filters: `file.path.contains("path")`
    - Extension filters: `file.ext.contains("md")`
    - Exclusion patterns: `!file.path.contains("path")` to skip folders
    - Recursive subdirectory scanning
  - **Table Generation**:
    - Renders HTML tables with proper styling (`base-table-container`, `base-table`)
    - Generates wikilinks (`[[Name]]`) that resolve to published content
    - Supports multiple columns with custom properties
    - Includes description/intro content before tables
  - **Frontmatter**:
    - Sets `enableToc: false`, `enableBacklinks: false`, `enableGraph: false`
    - Auto-generates title from BASE filename
  - **Examples**: Coffee Beans (46 entries), Books (184 entries, excluding 1256 Study books)
- **Code Structure**:
  - `base_parser.rs`: Parse BASE YAML into Rust structs (serde)
  - `base_query.rs`: Query notes with filter expressions, extract folder/extension/exclusion patterns
  - `base_renderer.rs`: Render notes as HTML tables with formatted values (ratings, prices)
  - `file_utils.rs`: Orchestrate BASE processing workflow with staging and cleanup

### 2026-04-17: Smart description extraction and OG image enhancement
- **Description Extraction** (`utils/obsidian-quartz/src/file_utils.rs`):
  - Automatically extracts clean descriptions from first paragraph after frontmatter
  - Removes wikilinks (`[[Link]]` → `Link`), markdown links (`[Text](URL)` → `Text`)
  - Strips formatting, list markers, blockquotes
  - Smart sentence truncation (prefers complete sentences, max 180 chars)
  - Stores as quoted `description: "..."` in frontmatter for proper YAML syntax
  - Manual override via `desc:` frontmatter field
- **OG Image Enhancement** (`utils/obsidian-quartz/src/svg_generator.rs`):
  - Added description text overlay (24px, Kanagawa oldWhite #C8C093)
  - Reduced title font sizes (44-60px) to make room for description
  - Optimized text wrapping (title: 25 chars/line, description: 60 chars/line)
  - Removed fixed accent line for cleaner layout
- **Hugo Template Updates** (`layouts/partials/head.html`):
  - Created `$cleanDescription` variable with priority: manual → auto-extracted → .Summary
  - Applied wikilink/markdown cleaning to all meta tags
  - Updated og:description, twitter:description, and JSON-LD schema

### 2025-10-16: Fix cross-section navigation (brain ↔ blog)
- Modified [`assets/js/router.js`](assets/js/router.js) to intercept clicks between `/brain/` and other sections, forcing full page loads instead of SPA navigation
- Prevents "null" page errors when navigating from brain to main blog
- Compatible with [`assets/js/external-links.js`](assets/js/external-links.js) which handles truly external links

## Inconsistencies (later TODO's)

### Slug rules — 4 spots, only 1 canonical

Brain note `no meetings (async).md` → URL slug `/no-meetings-async`. Four files do slug work. Two can disagree. One is source of truth.

Canonical: `utils/hugo-obsidian/util.go::UnicodeSanitize`. Strips `()`, `&`, `@`, `–`, `'`, etc. Collapses `-`/whitespace runs to one `-`.

| # | File | Lang | Role |
|---|------|------|------|
| 1 | `utils/hugo-obsidian/util.go::UnicodeSanitize` | Go | **Canonical**. Brain + blog hugo-obsidian use it. |
| 2 | `utils/obsidian-quartz/src/slug.rs` | Rust | **Mirror** of #1, and the only Rust copy. `unicode_sanitize` (paths) + `brain_slug` (one URL segment: lowercases, folds `/` and `\` to `-`). Every Rust caller goes through here — `enrich_with_blog` re-exports it for `merge_search_index` and `enrich_with_memories`; `file_utils` uses `brain_slug` for frontmatter aliases. Unit-tested against real note URLs. |
| 3 | `utils/obsidian-quartz/src/file_utils.rs::process_file` (writing step) | Rust | Filename lowercase only. #1 slugifies after. |
| 4 | `sspaeti-hugo-blog/helper-scripts/enrich-link-index.py:41` | Python | **Divergent**. Only `.lower().replace(" ", "-")`. Keeps parens. Source of paren-drop bugs. |
| 5 | `utils/obsidian-quartz/src/file_utils.rs::process_base_file` | Rust | BASE-page filename. Same as #3. |

Real disagreement: #1 vs #4. #2 compensates by re-canonicalising before matching brain IDs. #3 + #5 not slug generators — feed into #1.

Future cleanup: add `sluggify` subcommand to hugo-obsidian (Go). Blog Python shell out. Collapses #4 into #1. Kills need for #2.

Pending symmetric bug: `utils/obsidian-quartz/src/enrich_with_book.rs:154`. Same `brain_ids.contains()` pattern as fixed blog enricher. Same silent-drop on paren'd notes until same fix applied.

In-code pointers exist between #1 and #2 (`MIRRORED IN` / `KEEP IN SYNC WITH`).
