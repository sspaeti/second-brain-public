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
* **Local video embeds** (`![[video.mp4]]`): Obsidian video wikilinks render as native HTML5 `<video controls>` players — the mp4 is copied from the vault alongside images and served from `/brain/`. Optional width via `![[video.mp4|400]]`
* Callout blocks are normalized so compact and spaced forms render identically
* **Obsidian transclusions / embeds** (`![[Note#^block-id]]` and `![[Note#Heading]]`): block and section references render inline as a quoted blockquote (instead of a broken image), with nested `[[wikilinks]]` and `![[images]]` inside the embed resolved, plus a floated top-right link back to the source note that jumps to the exact spot — the heading for section refs, or the block's enclosing heading for block refs (hover shows the popover preview)
* **Mermaid → OG image**: set `ogimage: mermaid` (or `mermaid2`, `mermaid3`, …) in a note's frontmatter to render the Nth ` ```mermaid ` block as the social-media preview image (rendered via `mmdc` + ImageMagick to a 1200×630 WebP using a dark theme that matches the site's OG template)
* **Raw Markdown output**: every note is also published as plain Markdown at `/brain/<slug>/index.md`, so LLMs and scrapers can read the Obsidian source without the site chrome (see [Raw Markdown output per note](#raw-markdown-output-per-note))

The content/notes themselves are not published in this repo, only on [ssp.sh/brain](https://ssp.sh/brain).

> [!NOTE]
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

### RSS: full HTML content, versioned guid

`layouts/_default/rss.xml` (`rssFullContent = true` in `config.toml`) renders each item's `<description>` through `layouts/partials/textprocessing.html` — the same partial `single.html` uses — so wikilinks, embeds, and callouts show up as real HTML in the feed instead of raw Obsidian syntax.

Each item's `<guid>` is `{{ .Permalink }}?v={{ .Lastmod.Format "20060102" }}` (`isPermaLink="false"`), not the bare permalink. RSS 2.0 has no "updated item" concept (that's Atom's `<updated>`), so a stable guid means readers never resurface an edited note. Bumping the guid on every `lastmod` change is the workaround: readers (FreshRSS, Newsboat, …) treat each edit as a new item, so followers see every meaningfully-updated note again — at the cost of the same note appearing multiple times in reader history across edits. Deliberate tradeoff, chosen so updates aren't silently missed.

### Redirects of renamed files
Find these in [.htaccess](static/.htaccess)

## ChangeLog

See seperate file [CHANGELOG.md](CHANGELOG.md).

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
