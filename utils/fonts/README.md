# Iosevka source files

`iosevka-src/iosevka-{regular,bold,italic,bolditalic}.woff2` are the Iosevka
20.0.0 webfonts the sites have shipped since 2026-07 (975 codepoints, all
OpenType features, ~146 KB each). `make fonts-subset` derives the deployed
files in `static/fonts/woff2/` from them:

- `iosevka-<face>.woff2` — every codepoint, stylistic-set / character-variant
  glyphs (cvXX, ssXX, language ligation sets) dropped because no stylesheet
  enables them, but keeping `NWID`/`WWID` (Chrome renders ~200 arrow /
  box-drawing glyphs differently without them). ~67 KB.
- `iosevka-<face>-latin.woff2` — Latin, punctuation, currency, arrows, fi/fl;
  what a normal page needs. Preloaded (regular). ~46 KB.

Do **not** regenerate from `static/fonts/woff2/iosevka-*-full.woff2`: that is a
different Iosevka build (29,597 glyphs, other default variants). Latin is the
same, but ~200 glyphs differ — vertical/double arrows, box drawing, block
elements, Powerline icons — so code-block diagrams would change.

The blog (`../sspaeti-hugo-blog`, `make sync-fonts`) copies the eight generated
files from here, so both sites are byte-identical.
