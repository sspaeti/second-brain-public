# Obsidian Quartz Public Brain

A utility for processing and publishing Obsidian notes to a Quartz-powered website.

## Features

- **Selective Publishing**: Only publishes notes tagged with `#publish`
- **Image Handling**: Automatically copies referenced images to your public folder
- **OpenGraph Image Generation**: Creates SVG/WebP social media preview images. See `svg_generator.rs`
- **Date Extraction**: Extracts creation dates from `Created [[YYYY-MM-DD]]` pattern and adds to frontmatter
- **Frontmatter Management**: Preserves and enhances YAML frontmatter
- **Link Handling**: Converts link indexes to lowercase for better compatibility

## Usage

The tool requires several environment variables:

```bash
secondbrain="/path/to/your/private/obsidian/vault"
public_secondbrain="/path/to/your/public/folder"
export secondbrain public_secondbrain

# Process files
obsidian-quartz

# Convert link indexes for Hugo
obsidian-quartz convert_to_lower_case
```

## How It Works

1. Scans your Obsidian vault for notes with `#publish` tag
2. Processes each file:
   - Generates social media preview images
   - Updates frontmatter with metadata
   - Copies referenced images to the public folder
   - Writes processed files to the public destination
3. Preserves your existing frontmatter while ensuring required fields

## Installation

```bash
cd utils/obsidian-quartz
make
```

This builds and installs the utility to `/usr/local/bin/obsidian-quartz`.
