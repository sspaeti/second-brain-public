# OG Image Design Integration

## Summary

Successfully integrated **Design 8b (Landscape Bold - Monochrome)** into the Rust codebase for automatic OG image generation.

## What Changed

### 1. New SVG Template (`src/og_template.svg`)
- Created a separate template file containing the full design
- Uses placeholder `{{TITLE_PLACEHOLDER}}` for dynamic title insertion
- Design features:
  - **Dimensions:** 1350 x 1080px (5:4 landscape aspect ratio)
  - **Color palette:** Your website colors (#FF5D62, #181820, #1F1F28, #DCD7BA)
  - **Brain icon** in framed box (top left)
  - **sspaeti logo** in monochrome (bottom left, properly oriented)
  - **Bold typography** for titles
  - **"SECOND BRAIN"** header in Inter/Helvetica 600 weight
  - **"A Digital Vault of Knowledge"** tagline
  - **ssp.sh/brain** domain

### 2. Updated `src/svg_generator.rs`

## How It Works

1. **Title Processing:**
   ```rust
   split_title(title) // Groups words into lines (max 25 chars per line)
   ```

2. **SVG Generation:**
   ```rust
   create_svg(words, width, height)
   // 1. Loads template from src/og_template.svg
   // 2. Generates title text with proper escaping
   // 3. Replaces {{TITLE_PLACEHOLDER}} in template
   ```

3. **Image Conversion:**
   - SVG saved to `content/_img/feature/gen/{note-name}.svg`
   - Converted to WebP using ImageMagick
   - SVG file removed after conversion

## Testing

Build successfully completes:
```bash
cargo build --release
# Output: Finished `release` profile [optimized]
```

## Usage

The system automatically generates OG images when processing notes:

```bash
cd utils/obsidian-quartz
cargo run
# Or via Makefile
make prepare
```

Each published note with `#publish` tag will get:
- A unique SVG generated from template
- Title dynamically inserted (1-4 lines based on length)
- WebP conversion for optimal file size
- Frontmatter updated with ogimage path

## Example Output

For a note titled "Data Engineering Best Practices":
- Lines: ["Data Engineering", "Best Practices"]
- Font size: 88px (2 words)
- Position: x=100, y=600 and y=720
- File: `content/_img/feature/gen/data-engineering-best-practices.webp`

