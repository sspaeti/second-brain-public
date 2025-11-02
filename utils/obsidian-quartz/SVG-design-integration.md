# OG Image Design Integration

## Summary

Successfully integrated **Design 4 (Landscape Minimal Split-Screen)** into the Rust codebase for automatic OG image generation.

## Design Specifications

- **Dimensions:** 1200 × 630px (standard OG format, ~2:1 aspect ratio)
- **Color palette:** #FF5D62 (accent), #181820/#1F1F28 (dark backgrounds), #DCD7BA (text)
- **Layout:** Split-screen with brain icon (left), vertical divider at x=320, title area (right)
- **Typography:** Inter/Helvetica 600 weight
- **Dynamic font sizing:** 52-76px based on title length (1-4 lines)
- **Line breaking:** Max 20 chars/line, 4 lines maximum

## Key Components

### 1. SVG Template (`src/og_template.svg`)
- Brain icon: translate(80, 220), scale(8)
- Vertical divider at x=320
- Title placeholder: {{TITLE_PLACEHOLDER}}
- sspaeti logo: bottom right (1040, 550), scale(0.0025)
- Tagline: "A Digital Vault of Knowledge"
- Domain: ssp.sh/brain

### 2. Code (`src/svg_generator.rs`)
- `split_title()`: Word-based line breaking (max 20 chars/line, 4 lines)
- `create_svg()`: Template loading + dynamic title insertion
- Title positioning: x=360, y=230, line_height=70px
- XML character escaping for SVG safety

## Usage

```bash
cd utils/obsidian-quartz
cargo run
# Or via Makefile
make prepare
```

Each note tagged with `#publish` automatically gets:
- Unique OG image generated from template
- Title dynamically positioned and sized
- WebP conversion via ImageMagick
- Frontmatter updated with ogimage path

## Example Output

For "Automatic Grammar/Spellchecking on Markdown files (DevOps)":
- Lines: ["Automatic", "Grammar/Spellchecking", "on Markdown files", "(DevOps)"]
- Font size: 52px (4 lines)
- Output: `content/_img/feature/gen/automatic-grammar-spellchecking-on-markdown-files-devops.webp`
