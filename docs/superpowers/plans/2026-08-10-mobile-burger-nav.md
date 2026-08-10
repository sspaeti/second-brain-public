# Mobile Burger Navigation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the second brain's mobile icon-row nav with a hamburger + dropdown panel that visually matches the blog (sspaeti-hugo-blog), so the two sites are indistinguishable on mobile too.

**Architecture:** A self-contained burger. New markup in `blog-nav.html` (burger button + dropdown panel); new component styles in `darkmode.scss` (bar/X animation + panel, copied from the blog); mobile bar-visibility rules in `base.scss`; a small toggle script inside the existing SPA `render()` in `head.html`. The panel's Search and dark-mode rows **proxy** to the existing `#search-icon` and `#darkmode-toggle` controls — no search rebuild, no toggle duplication.

**Tech Stack:** Hugo templates (Go templates), SCSS (Dart Sass via Hugo Pipes), vanilla JS. No unit-test harness exists for templates/SCSS; verification is a clean `hugo` build plus manual browser checks in a mobile viewport.

---

## Testing note

This is a static-site UI change with no automated test framework. The verification gate for each task is:

1. **Build check:** `hugo --quiet` from the repo root completes with **no template or SCSS errors** (exit code 0). Run from `/home/sspaeti/git/sspaeti.com/second-brain-public`.
2. **Manual check (browser):** `make run` serves the dev site; open it, set the viewport to ≤768px wide (DevTools device toolbar), and confirm the behavior described in the task.

Where a task says "Verify build", run step 1. Where it says "Verify in browser", run step 2.

---

## File Structure

- `layouts/partials/blog-nav.html` — **Modify.** Keep the existing `.icon-links` block (desktop/tablet nav) and append a `.menu-toggle` button + a `.mobile-menu` panel. Responsible for all nav markup.
- `assets/styles/darkmode.scss` — **Modify.** Append the burger + panel component styles (appearance, X animation, dark-theme variants). Responsible for nav-component look.
- `assets/styles/base.scss` — **Modify.** Replace the current mobile icon-row reorder block (~lines 654–675) with burger-era bar rules: hide `.icon-links`/search/divider/darkmode/newsletter in the bar, show the burger, make the header a positioning context for the panel. Responsible for header bar layout.
- `layouts/partials/head.html` — **Modify.** Add the toggle script inside `render()` (~after line 307, alongside the existing active-link block) and extend the active-link selector to also mark the panel's matching link. Responsible for behavior.

---

## Task 1: Burger + panel markup

**Files:**
- Modify: `layouts/partials/blog-nav.html`

The current file is the `.icon-links` div (lines 1–38). Leave it exactly as-is and append the burger button and panel **after** the closing `</div>` on line 38.

- [ ] **Step 1: Append the burger button and panel markup**

Add this to the end of `layouts/partials/blog-nav.html` (after the existing `</div>` that closes `.icon-links`):

```html
<button class='menu-toggle' id='mobile-menu-toggle' type='button' aria-label='Menu' aria-controls='mobile-menu' aria-expanded='false'>
    <span></span><span></span><span></span>
</button>
<div class='mobile-menu' id='mobile-menu'>
    <button class='mobile-menu-item mobile-search' type='button'>
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 19.9 19.7" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M18.5 18.3l-5.4-5.4"/><circle cx="8" cy="8" r="7"/></svg>
        <span>Search</span>
    </button>
    <a class='mobile-menu-item' href="/posts/">Blogs</a>
    <a class='mobile-menu-item mobile-brain-link' href="/brain/">Brain</a>
    <a class='mobile-menu-item mobile-vault-link' href="/brain/data-engineering/">DE Vault</a>
    <a class='mobile-menu-item' href="https://www.ssp.sh/book/">Book</a>
    <a class='mobile-menu-item' href="https://www.ssp.sh/about/">About</a>
    <button class='mobile-menu-item mobile-darktoggle' type='button'>Toggle theme</button>
</div>
```

Note: the `.mobile-search` and `.mobile-darktoggle` are buttons that will proxy to the existing controls via JS (Task 4). The nav hrefs mirror the existing `.icon-links` entries so behavior stays identical.

- [ ] **Step 2: Verify build**

Run: `hugo --quiet`
Expected: exit code 0, no template errors. (Nothing is visible yet — the burger/panel are unstyled and will show up as raw elements; styling comes in Task 2/3.)

- [ ] **Step 3: Commit**

```bash
git add layouts/partials/blog-nav.html
git commit -m "feat(nav): add mobile burger button and dropdown panel markup"
```

---

## Task 2: Burger + panel component styles

**Files:**
- Modify: `assets/styles/darkmode.scss`

Append a new block at the end of `assets/styles/darkmode.scss`. These styles are copied/adapted from the blog's `themes/uBlogger/assets/css/_partial/_header.scss` (menu-toggle X animation + mobile `.menu` panel).

- [ ] **Step 1: Append burger + panel styles**

Add to the end of `assets/styles/darkmode.scss`:

```scss
// ── Mobile burger nav (matches the blog's hamburger + dropdown) ──────────────
// Hidden on desktop/tablet; the .icon-links text nav is used there instead.
.menu-toggle,
.mobile-menu {
    display: none;
}

@media all and (max-width: 768px) {
    // Burger button: three bars that animate into an X when .active
    .menu-toggle {
        display: inline-flex;
        flex-direction: column;
        justify-content: center;
        gap: 0;
        background: none;
        border: none;
        padding: 6px 4px;
        cursor: pointer;
        line-height: 1;

        span {
            display: block;
            width: 1.5rem;
            height: 2px;
            border-radius: 3px;
            background: var(--dark);
            transition: all 0.3s ease-in-out;
        }
        span:nth-child(1) { margin-bottom: 0.4rem; }
        span:nth-child(3) { margin-top: 0.4rem; }

        &.active {
            span:nth-child(1) { transform: rotate(45deg) translate(0.35rem, 0.42rem); }
            span:nth-child(2) { opacity: 0; }
            span:nth-child(3) { transform: rotate(-45deg) translate(0.35rem, -0.42rem); }
        }
    }

    // Dropdown panel: full-width, drops below the header bar
    .mobile-menu {
        position: absolute;
        top: 100%;
        left: 0;
        width: 100%;
        z-index: 200;
        display: none;
        text-align: center;
        padding-top: 0.5rem;
        background: var(--global-background-secondary-color);
        border-top: 2px solid var(--gray);
        box-shadow: 0 0.125rem 0.25rem rgba(0, 0, 0, 0.1);

        &.active { display: block; }

        .mobile-menu-item {
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 0.4rem;
            width: 100%;
            line-height: 2.5rem;
            padding: 0;
            background: none;
            border: none;
            font-size: 16px;
            font-weight: 500;
            font-family: inherit;
            color: var(--gray);
            text-decoration: none;
            cursor: pointer;

            &:hover { color: #D27E99; }
            &.active { color: var(--dark); font-weight: 700; }
        }
    }
}

[saved-theme="dark"] {
    @media all and (max-width: 768px) {
        .menu-toggle span { background: var(--global-font-secondary-color); }
        .mobile-menu {
            background: var(--global-background-secondary-color);
            border-top-color: var(--global-font-secondary-color);
        }
        .mobile-menu .mobile-menu-item {
            color: var(--global-font-secondary-color);
            &.active { color: #DCD7BA; }
        }
    }
}
```

- [ ] **Step 2: Verify build**

Run: `hugo --quiet`
Expected: exit code 0, no SCSS compile errors.

- [ ] **Step 3: Verify in browser**

Run: `make run`, open the site at a ≤768px viewport.
Expected: the burger (three bars) is now visible in the header. The panel is present in the DOM but hidden (no `.active` yet). The panel won't open on tap yet — JS comes in Task 4. The old icon row may still show; it is hidden in Task 3.

- [ ] **Step 4: Commit**

```bash
git add assets/styles/darkmode.scss
git commit -m "feat(nav): style mobile burger and dropdown panel to match blog"
```

---

## Task 3: Mobile header bar visibility

**Files:**
- Modify: `assets/styles/base.scss:654-675`

The current block (`assets/styles/base.scss` ~654–675) reorders the header cluster (search / icon-links / darkmode) on mobile. Under the burger design, the bar should show only logo + title + burger. Replace the block's body so it hides the icon-links, search chip, divider, newsletter, and dark toggle on mobile, and makes the header a positioning context for the absolute panel.

- [ ] **Step 1: Replace the mobile header reorder block**

Find this block in `assets/styles/base.scss` (starts ~line 654):

```scss
// mobile nav reorder: search, blog/vault/book, dark-toggle (newsletter hidden)
// right cluster sits flush; all slack space goes to the left of search
@media all and (max-width: 768px) {
  header {
    & #search-icon {
      order: 1;
      margin-right: 0.8em;
    }
    & > .icon-links {
      order: 2;
      padding: 0;
      margin-right: 0;
    }
    & .newsletter { display: none; }
    & .darkmode {
      order: 4;
      float: none;
      padding-left: 0.3em;
      padding-right: 0;
    }
  }
}
```

Replace it entirely with:

```scss
// mobile: full blog match — the bar shows only logo + title + burger.
// Search and the dark-mode toggle move into the burger panel (they proxy to
// the still-rendered #search-icon / #darkmode-toggle controls, which are hidden
// in the bar here). The header becomes the positioning context for .mobile-menu.
@media all and (max-width: 768px) {
  header {
    position: relative;

    & > .icon-links,
    & #search-icon,
    & .header-divider,
    & .newsletter,
    & .darkmode {
      display: none;
    }

    & > .menu-toggle {
      order: 5;
      margin-left: auto;
    }
  }
}
```

- [ ] **Step 2: Verify build**

Run: `hugo --quiet`
Expected: exit code 0.

- [ ] **Step 3: Verify in browser**

Run: `make run`, ≤768px viewport.
Expected: the header bar now shows only the logo, "Second Brain" title, and the burger pinned to the right. The old icon row, search chip, and dark toggle are gone from the bar. Panel still doesn't open (JS is Task 4).

- [ ] **Step 4: Commit**

```bash
git add assets/styles/base.scss
git commit -m "feat(nav): hide bar controls on mobile, show burger only"
```

---

## Task 4: Toggle behavior + active-link sync

**Files:**
- Modify: `layouts/partials/head.html` (inside `render()`, ~after line 307)

The `render()` function runs on every SPA navigation. Add the burger wiring there so it re-applies after nav and the panel auto-closes on navigation. The existing active-link block is at ~lines 300–307; add the new script right after it, and extend that block to also mark the panel link.

- [ ] **Step 1: Extend the active-link block to cover the panel**

Find this block in `layouts/partials/head.html` (~300–307):

```javascript
      // keep the current nav item highlighted after SPA navigation (server-side
      // class="active" only reflects the first-loaded page under SPA routing)
      (function () {
        var p = window.location.pathname;
        document.querySelectorAll('.icon-links a.active').forEach(function (a) { a.classList.remove('active'); });
        var sel = p.startsWith('/brain/data-engineering')
          ? '.icon-links .vault-link a'
          : (p.startsWith('/brain') ? '.icon-links .brain-link a' : null);
        if (sel) { var el = document.querySelector(sel); if (el) el.classList.add('active'); }
      })();
```

Replace it with (adds panel selectors alongside the icon-links ones):

```javascript
      // keep the current nav item highlighted after SPA navigation (server-side
      // class="active" only reflects the first-loaded page under SPA routing)
      (function () {
        var p = window.location.pathname;
        document.querySelectorAll('.icon-links a.active, .mobile-menu a.active').forEach(function (a) { a.classList.remove('active'); });
        var sels = p.startsWith('/brain/data-engineering')
          ? ['.icon-links .vault-link a', '.mobile-menu .mobile-vault-link']
          : (p.startsWith('/brain') ? ['.icon-links .brain-link a', '.mobile-menu .mobile-brain-link'] : []);
        sels.forEach(function (sel) { var el = document.querySelector(sel); if (el) el.classList.add('active'); });
      })();
```

- [ ] **Step 2: Add the burger toggle script**

Immediately after the block from Step 1 (still inside `render()`), add:

```javascript
      // Mobile burger toggle. Uses document-level delegation bound once (guarded
      // by a window flag) so it survives SPA DOM swaps; the open panel is closed
      // on every render() call below so navigation auto-closes it.
      (function () {
        var toggle = document.getElementById('mobile-menu-toggle');
        var panel = document.getElementById('mobile-menu');
        // Close any open panel on (re)render / navigation.
        if (toggle) { toggle.classList.remove('active'); toggle.setAttribute('aria-expanded', 'false'); }
        if (panel) { panel.classList.remove('active'); }

        if (window.__burgerBound) return;
        window.__burgerBound = true;

        document.addEventListener('click', function (e) {
          var t = document.getElementById('mobile-menu-toggle');
          var p = document.getElementById('mobile-menu');
          if (!t || !p) return;

          // Proxy rows first.
          if (e.target.closest('.mobile-search')) {
            t.classList.remove('active'); t.setAttribute('aria-expanded', 'false'); p.classList.remove('active');
            var si = document.getElementById('search-icon'); if (si) si.click();
            return;
          }
          if (e.target.closest('.mobile-darktoggle')) {
            var dt = document.getElementById('darkmode-toggle'); if (dt) dt.click();
            return;
          }
          // Nav link tap: let navigation happen, close the panel.
          if (e.target.closest('.mobile-menu a')) {
            t.classList.remove('active'); t.setAttribute('aria-expanded', 'false'); p.classList.remove('active');
            return;
          }
          // Burger tap: toggle open/closed.
          if (e.target.closest('#mobile-menu-toggle')) {
            var open = t.classList.toggle('active');
            p.classList.toggle('active', open);
            t.setAttribute('aria-expanded', open ? 'true' : 'false');
            return;
          }
          // Click outside an open panel: close.
          if (p.classList.contains('active') && !e.target.closest('#mobile-menu')) {
            t.classList.remove('active'); t.setAttribute('aria-expanded', 'false'); p.classList.remove('active');
          }
        });
      })();
```

- [ ] **Step 3: Verify build**

Run: `hugo --quiet`
Expected: exit code 0, no template errors.

- [ ] **Step 4: Verify in browser (full behavior checklist)**

Run: `make run`, ≤768px viewport. Confirm each:

- Tapping the burger animates the three bars into an X and drops the panel (Search row, Blogs, Brain, DE Vault, Book, About, Toggle theme).
- Tapping **Search** closes the panel and opens the search modal.
- Tapping **Toggle theme** flips light/dark (panel may stay open; the theme changes).
- Tapping a nav link (e.g. Brain) navigates and closes the panel.
- Tapping outside an open panel closes it.
- Re-open the panel, then navigate via a link: the panel is closed on the new page.
- On a `/brain/data-engineering/` page, open the panel: **DE Vault** shows the active (bold/bright) style.
- Repeat the open/close checks in **dark theme**: burger bars and panel text/border use the dark-theme colors.

- [ ] **Step 5: Commit**

```bash
git add layouts/partials/head.html
git commit -m "feat(nav): wire mobile burger toggle and panel active-link sync"
```

---

## Self-Review — spec coverage

- **Panel = full blog match (logo + burger in bar; search + dark toggle in panel):** Task 1 (markup), Task 3 (hide bar controls). ✅
- **Search reuses modal:** Task 1 `.mobile-search` row + Task 4 proxy to `#search-icon`. ✅
- **Self-contained burger, proxy controls:** Tasks 1–4; no uBlogger port, no DOM relocation. ✅
- **Burger X animation copied from blog:** Task 2. ✅
- **Panel look (centered, top border, shadow, stacked items):** Task 2. ✅
- **Dark-theme variants:** Task 2 `[saved-theme="dark"]` block; Task 4 checklist verifies. ✅
- **SPA re-bind + auto-close on nav:** Task 4 (render()-scoped close + once-bound delegation). ✅
- **Active-link highlight incl. panel:** Task 4 Step 1. ✅
- **Edge cases (resize, SPA mid-open):** covered by CSS `display:none` ≥769px + render() close. ✅
- **No-JS mobile fallback:** intentionally out of scope per spec (burger is inert without JS; desktop nav still renders at wider widths). Documented, no task. ✅

Class/ID names are consistent across tasks: `menu-toggle` / `#mobile-menu-toggle`, `mobile-menu` / `#mobile-menu`, `mobile-menu-item`, `mobile-search`, `mobile-darktoggle`, `mobile-brain-link`, `mobile-vault-link`. Proxy targets `#search-icon` and `#darkmode-toggle` match the existing partials.
