# Popover Previews on Blog + Book — Design

**Status:** Approved (2026-06-03)
**Implements:** Extend brain popover-v2 to `sspaeti-hugo-blog` (Hugo, `www.ssp.sh`) and `book/dedp` (mdBook, `dedp.online`).

---

## Goal

When a user hovers a link pointing at the brain (`https://www.ssp.sh/brain/<slug>/`) from the blog or the book, render the same scrollable, content-only popover preview already shipping in the brain.

## Scope

**In:**
- Blog: load popover JS/CSS, mark wikilink output with a class the script can target.
- Book: load popover JS/CSS via `additional-js` / `additional-css`; book already emits `class="brain-link"` on wikilinks.
- Brain: add scoped CORS headers to allow `https://dedp.online` (and `https://(www\.)?ssp\.sh` for completeness).
- Build-time copy of the canonical popover artifacts from brain into blog and book.
- Selector parametrization in the popover script so brain/blog/book can each pass their own selector.

**Out:**
- Image preview support in blog/book wikilinks (no `![[img.ext]]` rewriting on those sites — that is brain-only).
- Newsletter (`list.ssp.sh` / Listmonk) — deferred.
- Removing brain's v1 popover code path.
- Hover preview for non-brain links (external links, in-blog links, etc.).
- Math (KaTeX) rendering inside popovers — known gap from the brain implementation, tracked separately.

## Architecture

User hovers `<a class="brain-link" href="https://www.ssp.sh/brain/foo/">…</a>` on blog or book.

1. Popover script (`popover-v2.js`) attaches `mouseenter` to all `a.brain-link[href]`.
2. On hover: `fetch("https://www.ssp.sh/brain/foo/", { credentials: "same-origin" })`.
3. Browser sends `Origin: https://www.ssp.sh` (blog) or `Origin: https://dedp.online` (book).
4. Bunny CDN serves the cached brain page. Apache `.htaccess` (origin) sets `Access-Control-Allow-Origin: <origin>` + `Vary: Origin` so Bunny caches per-origin.
5. Browser accepts response. Script parses HTML, extracts all elements with class `popover-hint` (already marked by brain: title, meta, content body), prefixes their IDs, rewrites relative URLs to absolute against brain, mounts a `position: fixed` overlay on `<body>`.
6. Anchor scroll: if the link had `#heading`, popover inner scrolls to `#popover-internal-<heading>`.

CDN/CORS detail: `Vary: Origin` is load-bearing. Without it, Bunny may return the cached response from one origin to a different origin, breaking CORS. After deploying the `.htaccess` change, the brain pull-zone cache must be purged so pre-CORS entries are evicted.

## Canonical source

Brain owns the truth. Source files in `second-brain-public/`:

- `assets/js/popover-v2.js` — popover script (already exists from prior work).
- `assets/js/floating-ui.core.umd.min.js` — FloatingUI core (already vendored).
- `assets/js/floating-ui.dom.umd.min.js` — FloatingUI DOM (already vendored).
- `assets/css/popover-v2.css` — **new file**; plain CSS (flattened from existing `assets/styles/popover-v2.scss`).

Reason for plain CSS: book uses mdBook, which expects plain CSS in `additional-css`; blog also accepts plain CSS via Hugo `resources.Get`. A single plain-CSS file is consumed by all three sites with no per-site compilation step.

Brain transition: brain currently `@import "popover-v2";` inside `assets/styles/custom.scss`. That import is removed; `popover-v2.css` is loaded as a standalone stylesheet via `<link>` in brain's `head.html`. The existing `.scss` file is deleted.

## Selector parametrization

`popover-v2.js` exposes `window.initPopoverV2(opts)` with one option:

```js
window.initPopoverV2({ selector: "a.brain-link[href]" })
```

Default (when `opts` omitted or `opts.selector` is absent): `"a.internal-link[href]"`. Brain keeps calling `initPopoverV2()` with no argument. Blog and book call with `{ selector: "a.brain-link[href]" }`.

No other behavior parametrized.

## Build-time copy mechanism

Each consumer site owns a `sync-popover` Makefile target that copies the canonical files from brain into the consumer's repo. The target runs before the site's own build target.

Required files (4 total):
- `popover-v2.js`
- `popover-v2.css`
- `floating-ui.core.umd.min.js`
- `floating-ui.dom.umd.min.js`

Source path (absolute, on the dev machine): `$(BRAIN_REPO)/assets/{js,css}/<file>` where `BRAIN_REPO=/home/sspaeti/git/sspaeti.com/second-brain-public`. Default value lives in each consumer's Makefile, overridable via environment variable for portability.

The copy is destructive (overwrites existing files in the destination). Files are then committed to each consumer's git repo — they are part of the consumer's source tree, not an ephemeral build artifact. This means popover updates flow as: edit in brain → `make sync-popover` in blog → commit + deploy blog; same for book.

## Brain-side changes

### CORS in `.htaccess`

File: `second-brain-public/static/.htaccess`. Add this block near the top, after `RewriteEngine On`:

```apache
<IfModule mod_headers.c>
  SetEnvIf Origin "^https://(www\.)?(ssp\.sh|dedp\.online)$" CORS_ALLOWED_ORIGIN=$0
  Header set Access-Control-Allow-Origin "%{CORS_ALLOWED_ORIGIN}e" env=CORS_ALLOWED_ORIGIN
  Header merge Vary "Origin"
</IfModule>
```

Behavior: requests with `Origin` matching the allowlist get an echo-back CORS header and `Vary: Origin`. Other origins get neither — CORS fails for them as today. `Vary: Origin` instructs Bunny to cache per-origin.

After deploy: purge brain pull-zone cache in Bunny dashboard once to evict pre-CORS entries.

Verification:
```bash
curl -H "Origin: https://dedp.online" -I https://www.ssp.sh/brain/zettelkasten/
# expect: Access-Control-Allow-Origin: https://dedp.online
# expect: Vary: Origin (or includes Origin among other Vary values)

curl -H "Origin: https://evil.example.com" -I https://www.ssp.sh/brain/zettelkasten/
# expect: NO Access-Control-Allow-Origin header
```

### Plain-CSS conversion

`assets/styles/popover-v2.scss` → `assets/css/popover-v2.css` (plain CSS, nesting flattened). `custom.scss` import line removed. `head.html` loads the CSS standalone via `resources.Get "css/popover-v2.css" | resources.Fingerprint`.

## Blog-side changes (`sspaeti-hugo-blog`)

### Wikilink class

File: `layouts/partials/process-wikilinks.html`. Both branches (aliased and plain) emit `<a href="...">text</a>`. Change to `<a href="..." class="brain-link">text</a>`.

### Makefile sync target

File: `Makefile`. New target:
```makefile
BRAIN_REPO ?= /home/sspaeti/git/sspaeti.com/second-brain-public

sync-popover:
	cp $(BRAIN_REPO)/assets/js/popover-v2.js               assets/js/popover-v2.js
	cp $(BRAIN_REPO)/assets/js/floating-ui.core.umd.min.js assets/js/floating-ui.core.umd.min.js
	cp $(BRAIN_REPO)/assets/js/floating-ui.dom.umd.min.js  assets/js/floating-ui.dom.umd.min.js
	cp $(BRAIN_REPO)/assets/css/popover-v2.css             assets/css/popover-v2.css
```

The existing site build/deploy target gets `sync-popover` added as a prerequisite so the files are always current at build time.

### Head loading

File: `layouts/partials/head.html` (or wherever site-wide scripts/styles load). Add (mirror brain's pattern):
```html
{{ $popoverCSS := resources.Get "css/popover-v2.css" | resources.Fingerprint "md5" | resources.Minify }}
<link rel="stylesheet" href="{{ $popoverCSS.RelPermalink }}">

{{ $fuiCore := resources.Get "js/floating-ui.core.umd.min.js" | resources.Fingerprint "md5" }}
{{ $fuiDom  := resources.Get "js/floating-ui.dom.umd.min.js"  | resources.Fingerprint "md5" }}
{{ $popoverJS := resources.Get "js/popover-v2.js" | resources.Fingerprint "md5" | resources.Minify }}
<script src="{{ $fuiCore.RelPermalink }}"></script>
<script src="{{ $fuiDom.RelPermalink }}"></script>
<script src="{{ $popoverJS.RelPermalink }}" defer></script>
```

Init call (after DOM ready — script is `defer`-loaded, so init in an inline script that runs after `DOMContentLoaded` or, to match brain's pattern, in the existing `render()` callback if one exists):
```html
<script>
  document.addEventListener("DOMContentLoaded", function () {
    if (window.initPopoverV2) {
      window.initPopoverV2({ selector: "a.brain-link[href]" });
    }
  });
</script>
```

The exact insertion point depends on existing blog scaffolding; the plan will pin the precise file/line.

## Book-side changes (`book/dedp`)

### Makefile sync target

File: `Makefile`. New target:
```makefile
BRAIN_REPO ?= /home/sspaeti/git/sspaeti.com/second-brain-public

sync-popover:
	cp $(BRAIN_REPO)/assets/js/popover-v2.js               assets/js/popover-v2.js
	cp $(BRAIN_REPO)/assets/js/floating-ui.core.umd.min.js assets/js/floating-ui.core.umd.min.js
	cp $(BRAIN_REPO)/assets/js/floating-ui.dom.umd.min.js  assets/js/floating-ui.dom.umd.min.js
	cp $(BRAIN_REPO)/assets/css/popover-v2.css             assets/css/popover-v2.css
```

Add `sync-popover` as a prerequisite to the existing `mdbook build` target (or wrapper).

### `book.toml`

Register the new asset files in the existing `[output.html]` section:
```toml
additional-css = [
  # ...existing entries...,
  "./assets/css/popover-v2.css",
]
additional-js = [
  # ...existing entries...,
  "./assets/js/floating-ui.core.umd.min.js",
  "./assets/js/floating-ui.dom.umd.min.js",
  "./assets/js/popover-v2.js",
  "./assets/js/popover-v2-init.js",
]
```

The init call lives in a tiny new file `assets/js/popover-v2-init.js` (mdBook doesn't have a head-template mechanism, so an extra JS file is the idiomatic way):
```js
document.addEventListener("DOMContentLoaded", function () {
  if (window.initPopoverV2) {
    window.initPopoverV2({ selector: "a.brain-link[href]" });
  }
});
```

Init runs after `popover-v2.js` has defined `window.initPopoverV2`. Order in `additional-js` matters: FloatingUI first, then popover-v2, then the init file.

## Bunny CDN considerations

- `Vary: Origin` is critical. Bunny caches per-Origin when this header is present in the response.
- After deploying the `.htaccess` change, **manually purge the brain pull-zone cache** (Bunny dashboard or API) once. Pre-CORS cached entries don't have the headers and will keep blocking until evicted.
- Verification step (post-deploy) hits the live URL with explicit `Origin` headers and confirms the response varies correctly. See "CORS in `.htaccess`" above for `curl` commands.
- If Bunny's Vary handling proves finicky (rare, but possible), fallback is to configure CORS in Bunny Edge Rules instead of `.htaccess`. No code change in repos; tracked as a known fallback, not implemented.

## Testing / verification

This project has no automated JS test runner. Verification is done via local `curl` and browser hover. Each implementation task ends with a concrete `curl`/`grep`/browser check.

**Per-site smoke tests:**
- **Brain (regression):** hover internal link → v2 popover renders identically to today.
- **Blog (new):** hover a wikilink rendered from `[[Zettelkasten]]` → popover renders brain's Zettelkasten content. Check `class="brain-link"` is present in rendered HTML; check `popover-v2.js` loaded; check `fetch` to brain succeeds (browser network tab).
- **Book (new):** open any chapter that links to brain, hover the link → popover renders. Confirm `curl -I -H "Origin: https://dedp.online"` against the brain URL shows `Access-Control-Allow-Origin: https://dedp.online` AND `Vary: Origin`.

**Negative tests:**
- `curl -H "Origin: https://attacker.example" -I` against brain returns NO `Access-Control-Allow-Origin` header.
- Non-brain links on blog/book don't get popovers (selector is `a.brain-link[href]`, not all `<a>`).

**Bunny cache verification:**
After purge, first request from `dedp.online` populates one cache entry; first from `ssp.sh` populates a separate entry. Confirm by hitting the same URL with two different `Origin` curl calls and inspecting `cdn-cache` headers (`x-cache: HIT` after second call per origin).

## Implementation order (high-level)

1. Convert brain's popover-v2.scss → popover-v2.css; rewire brain head.html; verify brain regression.
2. Parametrize selector in popover-v2.js; brain regression.
3. Add CORS .htaccess block; deploy brain; purge Bunny cache; verify with curl.
4. Blog: wikilink class change, Makefile sync target, head.html wiring, smoke test.
5. Book: Makefile sync target, init file, book.toml registration, smoke test.

Each step is verifiable independently. The implementation plan (handed to writing-plans next) decomposes these into bite-sized TDD-equivalent tasks.
