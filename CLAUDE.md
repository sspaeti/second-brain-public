# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

This is a public second brain website built with Hugo and the Quartz theme (v3). It automatically publishes Obsidian notes tagged with `#publish` from a private vault to a public website hosted at ssp.sh/brain.

## Architecture

- **Hugo Static Site**: Uses Hugo with Quartz theme for generating the public website
- **Content Processing**: Two utility systems process Obsidian notes:
  - Python scripts (`utils/find-publish-notes.py`) - legacy approach
  - Rust utility (`utils/obsidian-quartz/`) - current preferred approach
- **Link Generation**: Uses `hugo-obsidian` (forked version) to generate backlinks and graph connections
- **Margin sidenotes**: `assets/styles/sidenotes.scss` + `assets/js/sidenotes.js` move callouts and footnotes into the right gutter and the TOC/backlinks into the left gutter at ≥1420px (see README changelog 2026-09-09). The JS re-runs on `million:navigate`; opt out per note with `sidenotes: false`.
- **Deployment**: Static files generated to `public/` and uploaded via rsync

## Core Commands

### Development and Building
```bash
# Start development server (full build + serve)
make serve

# Run Hugo development server only
make run

# Generate static site without serving
make hugo-generate

# Full deployment (build + upload)
make deploy
```

### Content Processing
```bash
# Process notes using Rust utility (preferred)
make prepare

# Process notes using Python scripts (legacy)
make prepare-python
```

### Deployment
```bash
# Upload to server
make upload

# Quick upload without full rebuild
make upload-only
```

## Required Environment Variables

The content processing requires these environment variables:
- `secondbrain`: Path to private Obsidian vault
- `public_secondbrain`: Path to public content folder

## Key Dependencies

- **Hugo**: Static site generator (`/usr/bin/hugo`)
- **hugo-obsidian**: Backlink generator (`/home/sspaeti/.go/bin/hugo-obsidian`)
- **obsidian-quartz**: Rust utility for content processing (built from `utils/obsidian-quartz/`)
- **Python dependencies**: For legacy scripts (pandoc, frontmatter)

## Content Processing Workflow

1. Notes tagged with `#publish` are identified in the private Obsidian vault
2. Content is processed to:
   - Convert first header to frontmatter
   - Generate social media preview images
   - Copy referenced images
   - Convert filenames to lowercase
3. Hugo generates static site with backlinks and graph visualization
4. Site is deployed via rsync to `sspaeti@sspaeti.com:~/www/ssp/brain`

## Important Notes

- The `hugo-obsidian` tool is a custom fork: https://github.com/sspaeti/hugo-obsidian
- Configuration prevents aliases for multi-word titles to avoid conflicts
- Link indexes are converted to lowercase for proper Hugo compatibility
- Git info is enabled for last modified dates