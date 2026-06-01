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
* YouTube links in Obsidian image syntax (`![title](https://youtube.com/watch?v=XXX)`) render as embedded video players instead of broken images
* Callout blocks are normalized so compact and spaced forms render identically
* **Mermaid → OG image**: set `ogimage: mermaid` (or `mermaid2`, `mermaid3`, …) in a note's frontmatter to render the Nth ` ```mermaid ` block as the social-media preview image (rendered via `mmdc` + ImageMagick to a 1200×630 WebP using a dark theme that matches the site's OG template)

The content/notes themselves are not published in this repo, only on [ssp.sh/brain](https://ssp.sh/brain).

> **[Explore with RAG → explore.ssp.sh](https://explore.ssp.sh)**
>
> Semantic search, hidden connections, and graph traversal powered by [obsidian-note-taking-assistant](https://github.com/sspaeti/obsidian-note-taking-assistant).

## Utils

### Content processing: obsidian-quartz

Rust CLI tool that processes Obsidian vault notes and outputs Hugo-compatible markdown. Handles frontmatter, tags, images, OG image generation, callout normalization, BASE database views, and more.

Key features:
- **Markdown publishing**: Processes notes tagged with `#publish`
- **BASE database views**: Publishes Obsidian Database Folder plugin `.base` files as HTML tables
- **Filter expressions**: Supports folder filters and exclusion patterns (`!file.path.contains`)
- **Smart descriptions**: Auto-extracts clean descriptions from first paragraph
- **OG image generation**: Creates social media preview images with SVG→WebP conversion
- **Mermaid OG images**: Renders a note's Mermaid diagram as its OG image via `ogimage: mermaid` / `mermaid<N>` (uses `mmdc` + ImageMagick, dark theme matches the site's OG template)

See **[utils/obsidian-quartz/README.md](./utils/obsidian-quartz/README.md)** for details.

### Backlink and graph creation

The tool used is `hugo-obsidian`, a small Go program written by Jacky. Here's the [source](https://github.com/jackyzha0/hugo-obsidian). It is not maintained anymore (as there is now a [v4](https://github.com/jackyzha0/quartz/tree/v4) without it) and it had bugs and didn't show all my backlinks. That's why I forked it and fixed the backlinks. You can find it here: [sspaeti/hugo-obsidian](https://github.com/sspaeti/hugo-obsidian).

### Hugo render hooks

Custom render hooks in `layouts/_default/_markup/`:
* **render-image.html** - Detects YouTube URLs and renders responsive iframe embeds (with timestamp support); all other images pass through normally

## Configs

### Redirects of renamed files
Find these in [.htaccess](static/.htaccess)

## ChangeLog

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
