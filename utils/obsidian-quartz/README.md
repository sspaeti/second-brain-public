# Obsidian Quartz Public Brain

A Rust CLI tool that processes Obsidian vault notes tagged with `#publish` and outputs Hugo-compatible markdown files for the [Quartz](https://github.com/jackyzha0/quartz) theme (v3).

## Features

- **Selective Publishing**: Only publishes notes tagged with `#publish` (in frontmatter or inline)
- **Frontmatter Management**: Parses, merges, and generates YAML frontmatter with sorted keys
- **Title Extraction**: Converts first `# heading` into frontmatter `title` and removes it from body
- **Tag Processing**: Extracts tags from `Tags:` lines, filters out emoji tags and `#publish`
- **Date Extraction**: Extracts creation dates from `Created [[YYYY-MM-DD]]` pattern
- **Description Extraction**: Automatically extracts clean descriptions from first paragraph:
  - Removes wikilinks (`[[Link]]` → `Link`), markdown links (`[Text](URL)` → `Text`)
  - Strips formatting (bold, italic, code, strikethrough)
  - Removes list markers (`-`, `*`, `>`, `1.`), blockquote lines
  - Smart sentence truncation (completes sentences when possible, max 180 chars)
  - Stores in frontmatter as quoted `description: "..."` for proper YAML syntax
  - Manual override: use `desc:` frontmatter field
- **Image Handling**: Detects `![[image.png]]` references and copies images to public folder
- **OpenGraph Image Generation**: Creates SVG/WebP social media preview images (1200x630) with:
  - Title text (44-60px font, max 4 lines, 25 chars/line)
  - Description overlay (24px font, Kanagawa oldWhite color #C8C093, max 3 lines, 60 chars/line)
  - Dark gradient background with brain icon and branding
  - Converted SVG → WebP via ImageMagick
- **Callout Normalization**: Inserts blank blockquote lines between callout headers and content so Hugo/Goldmark renders title and body as separate `<p>` elements
- **Link Index**: Converts `linkIndex.json` keys to lowercase for Hugo compatibility
- **Blog Cross-Site Enrichment**: Imports blog↔brain edges from `sspaeti-hugo-blog/static/indices/linkIndex.json` so blog posts appear as nodes in the brain's interactive graph. Reads each blog post's frontmatter `url:` to resolve the canonical `/blog/<slug>` (handles posts whose folder name doesn't match their URL), filters to cross-site edges only (skips blog↔blog), drops edges with unpublished brain targets, normalizes any pre-existing `/blog/foo/#anchor` references to canonical form, and appends idempotently (re-runs are no-ops). Blog node entries get `type: "blog"` for downstream filtering (e.g., excluding them from full-text search).
- **Filename Lowercasing**: All output filenames are lowercased
- **BASE File Support**: Publishes Obsidian Database Folder plugin `.base` files as standalone Hugo pages with database views:
  - Detects BASE files with `publish: true` property
  - Copies source files from vault to temporary staging folder
  - Supports recursive subdirectory scanning
  - Applies folder filters (`file.path.contains("...")`)
  - Supports exclusion patterns (`!file.path.contains("...")` to skip folders)
  - Generates HTML tables with wikilinks that resolve to published content
  - Includes description/introductory content before tables
  - Disables backlinks and graph for BASE pages
  - Cleans up temporary staging folder after processing
  - Example: Coffee beans database with 46+ entries, Books database with 180+ entries

## Project Structure

```
src/
  main.rs               # Entry point, directory traversal, image map building
  file_utils.rs         # Core note processing: frontmatter, tags, images, dates, callouts, BASE files
  svg_generator.rs      # OG image generation (SVG -> WebP via ImageMagick)
  handle_link_index.rs  # Lowercases Hugo linkIndex.json for compatibility
  enrich_with_blog.rs   # Imports blog↔brain cross-edges from the sister blog repo into brain indices
  base_parser.rs        # Parse BASE YAML files into Rust structs
  base_query.rs         # Query and filter notes based on BASE filter expressions
  base_renderer.rs      # Render BASE notes as HTML tables/cards/lists
  og_template.svg       # SVG template for social preview images
```

## Usage

The tool requires two environment variables:

```bash
export secondbrain="/path/to/your/private/obsidian/vault"
export public_secondbrain="/path/to/your/public/folder"

# Process notes from vault to public folder
obsidian-quartz

# Lowercase linkIndex.json for Hugo compatibility
obsidian-quartz convert_to_lower_case

# Merge blog↔brain cross-site edges into brain indices (reads ../sspaeti-hugo-blog/static/indices/*.json)
obsidian-quartz enrich-with-blog
```

The brain `Makefile` chains `make -C ../sspaeti-hugo-blog prepare` so the blog's indices are refreshed before `enrich-with-blog` runs. If the blog repo is missing or its build fails, the enrich step warns and exits cleanly — the brain-only graph still works.

## How It Works

1. Scans the Obsidian vault for `.md` and `.base` files (skipping `Book` and `Blog` directories)
2. For each `.base` file with `publish: true`:
   - Parses BASE YAML structure (filters, views, properties, formulas)
   - Extracts folder filters and exclusion patterns from filter expressions
   - Creates temporary staging folder (`/content/BASES/<base-name>/`)
   - Copies all matching source files from vault to staging folder (respecting exclusions)
   - Recursively scans subdirectories in source folder
   - Queries notes from staging folder to build table
   - Renders HTML table with wikilinks (`[[Name]]` format)
   - Includes description/intro content before table if specified
   - Writes standalone markdown page to `/content/<base-name>.md`
   - Cleans up temporary staging folder
3. For each `.md` file with a `#publish` tag:
   - Parses existing YAML frontmatter
   - Extracts title, tags, and created date
   - Extracts and cleans description from first paragraph (or uses manual `desc:` field)
   - Generates OG preview image with title + description overlay if `ogimage` not already in frontmatter
   - Copies referenced images to public folder
   - Normalizes callout blocks (inserts blank `>` lines for proper rendering)
   - Writes processed file with merged frontmatter to public folder
4. Output frontmatter keys are sorted alphabetically, tags formatted as inline YAML arrays
5. Descriptions are properly quoted and escaped for YAML syntax

### BASE File Workflow

The BASE file processing uses a staging approach to avoid scanning the private vault during query operations:

```
Private Vault                    Git Repo (Public)
-------------                    -----------------
/vault/folder/
  ├── Coffee.base    ──────┐     /content/
  ├── Item1.md          ├──────> ├── coffee-base.md (generated table)
  ├── Subfolder/        │        ├── item1.md (if #publish)
  │   └── Item2.md  ────┤        └── item2.md (if #publish)
  └── Study/            │
      └── Item3.md  ────┼──X (excluded by !file.path.contains)
                        │
                 Temp Staging:
                 /content/BASES/Coffee/ (cleaned up after)
```

This ensures:
- No direct vault scanning during BASE queries
- Exclusion patterns work correctly
- Wikilinks resolve to published content in `/content/`
- Clean separation between private vault and public repo

## Installation

```bash
cd utils/obsidian-quartz
make
```

This builds the release binary and installs it to `~/.local/bin/obsidian-quartz`.
