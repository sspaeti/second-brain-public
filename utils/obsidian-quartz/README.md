# Obsidian Quartz Public Brain

A Rust CLI tool that processes Obsidian vault notes tagged with `#publish` and outputs Hugo-compatible markdown files for the [Quartz](https://github.com/jackyzha0/quartz) theme (v3).

## Features

- **Selective Publishing**: Only publishes notes tagged with `#publish` (in frontmatter or inline)
- **Frontmatter Management**: Parses, merges, and generates YAML frontmatter with sorted keys
- **Title Extraction**: Converts first `# heading` into frontmatter `title` and removes it from body
- **Tag Processing**: Extracts tags from `Tags:` lines, filters out emoji tags and `#publish`
- **Date Extraction**: Extracts creation dates from `Created [[YYYY-MM-DD]]` pattern
- **Image Handling**: Detects `![[image.png]]` references and copies images to public folder
- **OpenGraph Image Generation**: Creates SVG/WebP social media preview images (1200x630) via ImageMagick
- **Callout Normalization**: Inserts blank blockquote lines between callout headers and content so Hugo/Goldmark renders title and body as separate `<p>` elements
- **Link Index**: Converts `linkIndex.json` keys to lowercase for Hugo compatibility
- **Filename Lowercasing**: All output filenames are lowercased

## Project Structure

```
src/
  main.rs               # Entry point, directory traversal, image map building
  file_utils.rs          # Core note processing: frontmatter, tags, images, dates, callouts
  svg_generator.rs       # OG image generation (SVG -> WebP via ImageMagick)
  handle_link_index.rs   # Lowercases Hugo linkIndex.json for compatibility
  og_template.svg        # SVG template for social preview images
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
```

## How It Works

1. Scans the Obsidian vault for `.md` files (skipping `Book` and `Blog` directories)
2. For each file with a `#publish` tag:
   - Parses existing YAML frontmatter
   - Extracts title, tags, and created date
   - Generates OG preview image if `ogimage` not already in frontmatter
   - Copies referenced images to public folder
   - Normalizes callout blocks (inserts blank `>` lines for proper rendering)
   - Writes processed file with merged frontmatter to public folder
3. Output frontmatter keys are sorted alphabetically, tags formatted as inline YAML arrays

## Installation

```bash
cd utils/obsidian-quartz
make
```

This builds the release binary and installs it to `~/.local/bin/obsidian-quartz`.
