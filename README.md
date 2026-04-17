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
* YouTube links in Obsidian image syntax (`![title](https://youtube.com/watch?v=XXX)`) render as embedded video players instead of broken images
* Callout blocks are normalized so compact and spaced forms render identically

The content/notes themselves are not published in this repo, only on [ssp.sh/brain](https://ssp.sh/brain).

> **[Explore with RAG → explore.ssp.sh](https://explore.ssp.sh)**
>
> Semantic search, hidden connections, and graph traversal powered by [obsidian-note-taking-assistant](https://github.com/sspaeti/obsidian-note-taking-assistant).

## Utils

### Content processing: obsidian-quartz

Rust CLI tool that processes Obsidian vault notes and outputs Hugo-compatible markdown. Handles frontmatter, tags, images, OG image generation, callout normalization, and more.

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
