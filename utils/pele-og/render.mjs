#!/usr/bin/env node
// Render a Mermaid diagram to a self-contained SVG with pele (https://pele.run), for the
// OG images obsidian-quartz builds from `ogimage: mermaid` (src/svg_generator.rs).
//
//   node utils/pele-og/render.mjs <in.mmd> <out.svg> [width]
//
// Pele colours its SVG through CSS custom properties (--pele-*), which librsvg does not
// resolve, so every var() is substituted with the site's dark palette below (the values
// assets/styles/pele.scss resolves to in dark mode) before the file is written. The result
// is plain SVG that rsvg-convert / ImageMagick rasterize without a browser.
//
// pele itself is vendored in ./vendor (dist/pele.js of the npm package, MIT; version in
// vendor/VERSION, refresh with `make update-pele-og`), so nothing has to be installed.
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const { render } = await import(join(here, 'vendor', 'pele.js'));

const [, , input, output, widthArg] = process.argv;
if (!input || !output) {
  console.error('usage: render.mjs <in.mmd> <out.svg> [width]');
  process.exit(2);
}
const width = Math.max(280, parseInt(widthArg || '1100', 10) || 1100);

// Dark palette: mirror of [saved-theme="dark"] in assets/styles/pele.scss (card #1F1F28,
// fujiWhite text, oldWhite lines, blog table surfaces, Kanagawa series).
const PALETTE = {
  'pele-bg': '#1F1F28',
  'pele-fg': '#DCD7BA',
  'pele-muted': '#C8C093',
  'pele-line': '#DCD7BA',
  'pele-surface': '#2A2A37',
  'pele-surface-alt': '#363646',
  'pele-border': '#C8C093',
  'pele-accent': '#E46876',
  'pele-critical': '#C34043',
  'pele-error': '#C34043',
  'pele-radius': '4px',
  'pele-font': "'Atkinson Hyperlegible', sans-serif",
  'pele-font-mono': "'Iosevka', monospace",
  'pele-title-weight': 'bold',
  'pele-heading-weight': 'bold',
  'pele-bold-weight': 'bold',
  'pele-series-1': '#E46876',
  'pele-series-2': '#7093A3',
  'pele-series-3': '#76946A',
  'pele-series-4': '#E6C384',
  'pele-series-5': '#D27E99',
  'pele-series-6': '#938AA9',
  'pele-series-7': '#7E9CD8',
  'pele-series-8': '#C34043',
};

const source = readFileSync(input, 'utf8');
const { svg } = render(source, {
  maxWidth: width,
  responsive: false,
  fontFamily: 'Atkinson Hyperlegible',
  // gantt charts are drawn at a fixed width (800) and only capped to maxWidth: ask for the
  // full width so the chart fills the image like the page does (see diagrams-pele.js)
  config: { gantt: { useWidth: width } },
});

writeFileSync(output, inlineVariables(svg));

// Replace every `var(--name[, fallback])` with a concrete value. Pele declares private
// aliases on the root (`--_fg:var(--pele-fg,#1f1f1f)`), so those are collected first and
// resolved through the palette; anything unknown keeps its fallback.
function inlineVariables(markup) {
  const values = { ...PALETTE };
  const declaration = /--(_\w+)\s*:\s*var\(--([\w-]+)(?:\s*,\s*([^)]*))?\)/g;
  for (const [, alias, name, fallback] of markup.matchAll(declaration)) {
    values[alias] = values[name] ?? (fallback ?? '').trim();
  }
  // drop the alias declarations themselves, then resolve the remaining var() uses
  let out = markup.replace(/--_\w+\s*:\s*var\([^)]*\);?/g, '');
  const use = /var\(--([\w-]+)(?:\s*,\s*([^)]*))?\)/g;
  for (let pass = 0; pass < 3 && use.test(out); pass++) {
    out = out.replace(use, (_, name, fallback) => values[name] ?? (fallback ?? '').trim());
  }
  return out;
}
