# Mobile Burger Navigation — Design

**Date:** 2026-08-10
**Status:** Approved, ready for implementation plan

## Goal

Make the second brain's mobile header (ssp.sh/brain) identical to the blog's
(sspaeti-hugo-blog). Today the two sites share the same desktop header, nav bar,
and CSS, so visitors can't tell them apart — **except on mobile**: the blog shows
a hamburger (burger) that opens a dropdown menu, while the second brain shows a
horizontally-scrollable row of icons. This design replaces that icon row with a
burger + dropdown panel that matches the blog.

## Background — how each side works today

**Second brain (current mobile, ≤768px):** `assets/styles/darkmode.scss` hides
`.nav-text` and shows the SVG icons of `.icon-links` (Blogs, Brain, DE Vault,
Book, About) as a scrollable icon row. The search chip (`#search-icon`, opens a
**modal**) and the dark-mode toggle (`.darkmode`) sit beside it in the header bar.
Mobile header reorder rules live in `assets/styles/base.scss` (~line 656).

**Blog (uBlogger / LoveIt theme):** `themes/uBlogger/.../_header.scss` +
`theme.js`. A `.menu-toggle` of three `<span>` bars (1.5rem × 2px, radius 3px)
animates into an X when `.active` (`rotate(45deg)` / `rotate(-45deg)` translate).
Tapping it toggles `.active` on both the burger and a full-width `.menu` panel
(centered text, top border, drop shadow, items `display:block`). The blog tucks
**search + theme-switch inside** the panel. Toggle JS is trivial:
`click → classList.toggle('active')` on both elements, plus a click-outside mask.

## Decisions (from brainstorming)

- **Panel contents:** Full blog match. On mobile the header bar shows only
  logo + burger; search **and** the dark-mode toggle move into the panel.
- **Search in panel:** Reuse the existing modal. The panel's "Search" row is a
  button that triggers the current search modal (no rebuild, behavior stays
  consistent site-wide). It is a row, not an inline input box.
- **Implementation:** Self-contained burger that *proxies* to existing controls —
  not a literal uBlogger theme port, and not runtime DOM relocation.

## Approach — self-contained burger, proxy the controls

Add burger + panel markup. On mobile (≤768px) hide the icon-links row, the bar's
search chip, and the dark toggle; show the burger. The panel has its own stacked
nav-link anchors plus a "Search" row and a dark-toggle row that **proxy** to the
existing `#search-icon` and `.darkmode` controls (dispatch a click). ~40 lines of
SCSS lifted from the blog's bar/X animation, a ~15-line toggle. No search rebuild,
no theme-toggle duplication, no uBlogger machinery.

Rejected alternatives:
- **Literal theme port** — drags in `$header-height` fixed-positioning,
  Algolia search wiring, and `data-header-mobile` scroll behavior that don't
  exist here; lots of adaptation for no visible gain.
- **Runtime DOM relocation** — moving the real search/dark nodes into the panel
  is fragile under the SPA `render()` (nodes are re-created on nav) and on resize.

## Components

### 1. Markup — `layouts/partials/blog-nav.html`

Keep the existing `.icon-links` block untouched (it remains the desktop/tablet
nav). Append two siblings:

- **`.menu-toggle`** button — three `<span>` bars, `aria-label="Menu"`,
  `aria-expanded="false"`, `aria-controls` pointing at the panel. Hidden ≥769px.
- **`.mobile-menu`** panel — in order:
  1. A "Search" row (button/anchor) that opens the modal.
  2. The five nav anchors, reusing the same hrefs and active-state logic already
     in the partial (Blogs, Brain, DE Vault, Book, About).
  3. A dark-toggle row.
  Hidden ≥769px and hidden until `.active`.

### 2. SCSS — new block in `assets/styles/darkmode.scss` (next to existing nav styles)

- `.menu-toggle` / `.mobile-menu`: `display:none` by default; shown only under
  `@media (max-width:768px)`.
- Burger bars + `.active` → X animation, copied from the blog
  (`_header.scss` ~lines 248–290): bars 1.5rem × 2px, `rotate(45deg)` /
  `rotate(-45deg)` translate, second bar `opacity:0`.
- Panel: full-width dropdown below the bar, `text-align:center`, top border,
  drop shadow, `.menu-item { display:block; line-height:2.5rem }`,
  `.active { display:block }` — matching the blog.
- Under the same `@media`, hide `.icon-links`, `#search-icon`, `.header-divider`,
  and `.darkmode` in the bar. This replaces the current icon-row mobile rules in
  `base.scss` (~line 656) with "show burger instead".
- Dark-mode variants via the existing `[saved-theme="dark"]` selectors, matching
  the blog's `[theme=dark]` bar/border/background treatment.

### 3. Toggle JS — small `<script>`, invoked from `render()` in `layouts/partials/head.html`

Runs inside `render()` so it survives / re-binds after SPA navigation. Prefer
**event delegation on `document`** (bind once, survives DOM swaps):

- Click on `.menu-toggle` → toggle `.active` on burger + panel; update
  `aria-expanded`.
- Panel "Search" row → `document.getElementById('search-icon').click()`, then
  close the panel.
- Panel dark row → click the existing `.darkmode` control.
- Close (remove `.active`, reset `aria-expanded`) on: click outside the panel,
  tap of any nav link inside the panel, or SPA navigation.

## Data flow / behavior

Burger and panel are pure client-side toggles over a single `.active` class.
Active-link highlighting reuses the existing `render()` block that already sets
`.active` on `.icon-links` links; extend its selector so the panel's matching
link is marked too (Blogs / Brain / DE Vault by pathname).

## Edge cases

- **Resize mobile → desktop while open:** CSS `display:none` at ≥769px hides the
  panel automatically; JS also clears `.active` on nav.
- **SPA navigation mid-open:** panel closes because `render()` clears `.active`.
- **No-JS mobile:** burger does nothing. Acceptable — but confirm during
  implementation whether a no-JS fallback matters (the desktop icon-links still
  render for wider viewports regardless).

## Testing

Manual via `make serve` at ≤768px:

- Burger animates into an X when tapped.
- Panel drops with Search row + 5 links + dark toggle.
- Search row opens the existing modal.
- Dark row flips the theme.
- Tapping a nav link navigates and closes the panel.
- Outside-tap closes the panel.
- Verify in both light and dark themes.
- On a `/brain/data-engineering/` page, the DE Vault link shows the active state.

## Files touched

- `layouts/partials/blog-nav.html` — add burger + panel markup.
- `assets/styles/darkmode.scss` — burger/panel styles + mobile bar-hiding rules.
- `assets/styles/base.scss` — retire/adjust the current mobile icon-row reorder
  rules (~line 656) superseded by the burger.
- `layouts/partials/head.html` — toggle JS inside `render()`; extend active-link
  selector to cover the panel.
