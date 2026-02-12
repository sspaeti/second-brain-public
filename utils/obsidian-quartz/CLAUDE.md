# CLAUDE.md — obsidian-quartz

Rust CLI tool that processes Obsidian markdown notes for publishing via Hugo + Quartz.

## What It Does

Reads markdown files from a private Obsidian vault, filters for `#publish`-tagged notes, and outputs processed files to the public content folder. Two modes:

1. **Default mode** — process notes: extract title/tags/frontmatter, copy images, generate OG images (SVG→WebP via ImageMagick), write cleaned files with merged frontmatter
2. **`convert_to_lower_case` arg** — lowercase `assets/indices/linkIndex.json` for Hugo compatibility

## Project Structure

```
src/
  main.rs            — CLI entry, directory walking, image map building
  file_utils.rs      — Core note processing: frontmatter parsing/merging, tag extraction, image copying, file writing
  svg_generator.rs   — OG image generation (SVG templating + ImageMagick conversion)
  handle_link_index.rs — Lowercases linkIndex.json
  og_template.svg    — SVG template for OG images (uses {{TITLE_PLACEHOLDER}})
```

## Build & Run

```bash
make              # cargo build --release + install to ~/.local/bin/
make run-debug    # cargo run (debug mode)
```

## Required Environment Variables

- `secondbrain` — path to private Obsidian vault
- `public_secondbrain` — path to public Hugo content folder

## Dependencies

- `regex`, `glob`, `chrono`, `serde_yaml`, `serde`, `pandoc`
- External: `magick` (ImageMagick) for SVG→WebP conversion

## Key Behaviors

- Skips `Book/` and `Blog/` directories (symlinked folders)
- Filters out emoji tags defined in `EXCLUDED_TAG_EMOJIS`
- Frontmatter is sorted alphabetically by key
- Tags formatted as YAML inline arrays: `tags: [tag1, tag2]`
- OG images only generated if `ogimage` key missing from frontmatter
- Output filenames are lowercased
- Created date extracted from `Created [[YYYY-MM-DD]]` pattern in note body
- Callout blocks get a blank `>` line inserted between header and content for Goldmark
- Files with special names (`_index.md`, `data engineering.md`, etc.) get `enableToc: false`
