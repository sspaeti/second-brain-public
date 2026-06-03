# Popover on Blog + Book Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the brain popover-v2 to the blog (`www.ssp.sh`, Hugo) and the book (`dedp.online`, mdBook), with scoped CORS on the brain server, single canonical source of popover JS/CSS in brain, and build-time copy into the consumer repos.

**Architecture:** Brain owns canonical `popover-v2.js`, `popover-v2.css`, and FloatingUI vendor files. Blog and book each have a `sync-popover` Makefile target that copies these files from the brain repo before their site builds. Popover script gains a `{selector}` option so blog/book can target `a.brain-link[href]` while brain keeps `a.internal-link[href]`. Apache `.htaccess` on the brain echoes a scoped `Access-Control-Allow-Origin` and `Vary: Origin` (load-bearing for Bunny CDN per-origin caching).

**Tech Stack:** Hugo (brain, blog), mdBook (book), Apache `.htaccess` (CORS), Bunny CDN, Make.

**Branches (already set up):**
- Brain: `popover-v2` (current — spec already committed here)
- Blog: `popover-brain` (empty placeholder, matches `master`)
- Book: `popover-brain` (empty placeholder, matches `main`)

**Repo paths:**
- Brain: `/home/sspaeti/git/sspaeti.com/second-brain-public`
- Blog: `/home/sspaeti/git/sspaeti.com/sspaeti-hugo-blog`
- Book: `/home/sspaeti/git/book/dedp`

**No automated JS/template test suite exists.** Each task ends with a concrete `hugo --quiet && grep` / `curl` / browser check.

**Deploy steps (Phase 4) require manual user confirmation** — they push to production.

---

## File Structure

**Brain (canonical source) — create/modify:**
- `assets/css/popover-v2.css` — **CREATE**: plain CSS, flattened from current SCSS
- `assets/js/popover-v2.js` — **MODIFY**: accept `{selector}` option
- `assets/styles/popover-v2.scss` — **DELETE**: replaced by plain CSS
- `assets/styles/custom.scss` — **MODIFY**: remove `@import "popover-v2";` line
- `layouts/partials/head.html` — **MODIFY**: load popover-v2.css via `<link>`
- `static/.htaccess` — **MODIFY**: add CORS block

**Blog — create/modify:**
- `assets/css/popover-v2.css` — **CREATE via sync**: copied from brain
- `assets/js/popover-v2.js` — **CREATE via sync**: copied from brain
- `assets/js/floating-ui.core.umd.min.js` — **CREATE via sync**
- `assets/js/floating-ui.dom.umd.min.js` — **CREATE via sync**
- `Makefile` — **MODIFY**: add `sync-popover` target + make `hugo-generate`/`run` depend on it
- `layouts/partials/process-wikilinks.html` — **MODIFY**: emit `class="brain-link"` in both `<a>` outputs
- `layouts/partials/assets.html` — **MODIFY**: load FloatingUI + popover JS + CSS + init call

**Book — create/modify:**
- `assets/css/popover-v2.css` — **CREATE via sync**: copied from brain
- `assets/js/popover-v2.js` — **CREATE via sync**: copied from brain
- `assets/js/floating-ui.core.umd.min.js` — **CREATE via sync**
- `assets/js/floating-ui.dom.umd.min.js` — **CREATE via sync**
- `assets/js/popover-v2-init.js` — **CREATE manually** (idempotent init call)
- `Makefile` — **MODIFY**: add `sync-popover` target + make `serve`/`brain` depend on it
- `book.toml` — **MODIFY**: register the 4 popover assets in `additional-css` / `additional-js`

---

## Phase 1: Brain prep (single source of truth)

### Task 1: Convert popover-v2.scss → popover-v2.css

**Files:**
- Create: `second-brain-public/assets/css/popover-v2.css`
- Delete: `second-brain-public/assets/styles/popover-v2.scss`
- Modify: `second-brain-public/assets/styles/custom.scss` (remove the `@import "popover-v2";` line)

The new file is plain CSS (no SCSS nesting, no `&` parent refs) so blog and book can use it without an SCSS compiler.

- [ ] **Step 1: Create the plain-CSS file**

Create `second-brain-public/assets/css/popover-v2.css`:

```css
@keyframes popover-v2-dropin {
  0%   { opacity: 0; visibility: hidden; }
  1%   { opacity: 0; }
  100% { opacity: 1; visibility: visible; }
}

.popover-v2 {
  z-index: 999;
  position: fixed;
  overflow: visible;
  padding: 1rem;
  left: 0;
  top: 0;
  will-change: transform;
  visibility: hidden;
  opacity: 0;
  transition:
    opacity 0.3s ease,
    visibility 0.3s ease;
}

.popover-v2 > .popover-v2-inner {
  position: relative;
  width: 30rem;
  max-height: 20rem;
  padding: 1rem;
  font-weight: initial;
  font-style: initial;
  line-height: normal;
  font-size: 0.9rem;
  font-family: var(--bodyFont, inherit);
  border: 1px solid var(--lightgray, #ddd);
  background-color: var(--light, #fff);
  color: var(--dark, #222);
  border-radius: 5px;
  box-shadow: 6px 6px 36px 0 rgba(0, 0, 0, 0.25);
  overflow: auto;
  overscroll-behavior: contain;
  white-space: normal;
  cursor: default;
}

.popover-v2 > .popover-v2-inner[data-content-type*="pdf"],
.popover-v2 > .popover-v2-inner[data-content-type*="image"] {
  padding: 0;
  max-height: 100%;
}

.popover-v2 > .popover-v2-inner[data-content-type*="image"] img {
  margin: 0;
  border-radius: 0;
  display: block;
  max-width: 100%;
}

.popover-v2 > .popover-v2-inner[data-content-type*="pdf"] iframe {
  width: 100%;
  min-height: 20rem;
  border: 0;
}

.popover-v2 h1,
.popover-v2 h2,
.popover-v2 h3 { font-size: 1rem; margin-top: 0.4rem; }
.popover-v2 p { margin: 0.4rem 0; }
.popover-v2 img { max-width: 100%; }

.popover-v2 .meta {
  opacity: 0.7;
  font-style: italic;
  font-size: 0.85rem;
  color: var(--gray, #4e4e4e);
}
.popover-v2 .meta .meta-label-full  { display: none; }
.popover-v2 .meta .meta-label-short { display: inline; }

.popover-v2 .e-content a[href^="http"]:not(.hanchor) {
  color: var(--gray);
  text-decoration: underline;
  text-decoration-color: var(--secondary);
  text-underline-offset: 3px;
  text-decoration-thickness: 2px;
}

.popover-v2 .e-content a.internal-link:not(.broken),
.popover-v2 .e-content a:not(.internal-link):not(.hanchor):not([href^="http"]) {
  color: var(--gray);
  background-color: transparent;
  text-decoration: underline;
  text-decoration-color: var(--tertiary);
  text-underline-offset: 3px;
  text-decoration-thickness: 2px;
}

.popover-v2 .e-content a.internal-link.broken {
  color: #000;
  background-color: transparent;
  text-decoration: underline dashed;
  text-decoration-color: #555;
  text-underline-offset: 3px;
  text-decoration-thickness: 1px;
  cursor: not-allowed;
}

[saved-theme="dark"] .popover-v2 .e-content a[href^="http"]:not(.hanchor) {
  text-decoration-color: var(--primary);
}
[saved-theme="dark"] .popover-v2 .e-content a.internal-link:not(.broken),
[saved-theme="dark"] .popover-v2 .e-content a:not(.internal-link):not(.hanchor):not([href^="http"]) {
  text-decoration-color: var(--secondary);
}
[saved-theme="dark"] .popover-v2 .e-content a.internal-link.broken {
  color: #8BA4B0;
  opacity: 0.7;
  text-decoration-color: #8BA4B0;
}

@media (max-width: 768px) {
  .popover-v2 { display: none !important; }
}

.popover-v2.active-popover {
  animation: popover-v2-dropin 0.3s ease;
  animation-fill-mode: forwards;
  animation-delay: 0.15s;
}

.popover-v2:hover {
  visibility: visible;
  opacity: 1;
}
```

- [ ] **Step 2: Delete the SCSS source file**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
rm assets/styles/popover-v2.scss
```

- [ ] **Step 3: Remove the @import line from custom.scss**

Edit `second-brain-public/assets/styles/custom.scss`. Find at top of file:

```scss
// Add your own CSS here!

@import "popover-v2";

:root {
```

Change to:

```scss
// Add your own CSS here!

:root {
```

(Two lines deleted: the `@import` and the blank line above it.)

- [ ] **Step 4: Verify Hugo build still passes**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
hugo --quiet 2>&1 | head -20
```

Expected: no errors. Build completes.

- [ ] **Step 5: Commit**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
git add assets/css/popover-v2.css assets/styles/popover-v2.scss assets/styles/custom.scss
git commit -m "$(cat <<'EOF'
refactor(popover): convert popover-v2 from SCSS to plain CSS

Plain CSS lets blog + book consume the same file without an SCSS
compiler. Nesting flattened; behavior identical.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Load popover-v2.css standalone in brain head.html

The previous task removed the SCSS `@import`. The popover styles are now a standalone stylesheet that brain's `head.html` must load via `<link>`. Use the same Hugo `resources.Get` + `Fingerprint` + `Minify` pattern as other site assets.

**Files:**
- Modify: `second-brain-public/layouts/partials/head.html` (around line 174 — same area where popover.js is loaded)

- [ ] **Step 1: Add the standalone stylesheet link**

Open `second-brain-public/layouts/partials/head.html`. Find the existing block (around line 174):

```html
  {{ if $data.enableLinkPreviewV2 | default $.Site.Data.config.enableLinkPreviewV2 }}
    {{ $popoverV2 := resources.Get "js/popover-v2.js" | resources.Fingerprint "md5" | resources.Minify }}
    <script src="{{$popoverV2.Permalink}}"></script>
  {{ else }}
    {{ $popover := resources.Get "js/popover.js" | resources.Fingerprint "md5" | resources.Minify }}
    <script src="{{$popover.Permalink}}"></script>
  {{ end }}
```

Insert this stylesheet load right BEFORE that block:

```html
  {{ $popoverV2Css := resources.Get "css/popover-v2.css" | resources.Fingerprint "md5" | resources.Minify }}
  <link rel="stylesheet" href="{{ $popoverV2Css.Permalink }}">

  {{ if $data.enableLinkPreviewV2 | default $.Site.Data.config.enableLinkPreviewV2 }}
    {{ $popoverV2 := resources.Get "js/popover-v2.js" | resources.Fingerprint "md5" | resources.Minify }}
    <script src="{{$popoverV2.Permalink}}"></script>
  {{ else }}
    {{ $popover := resources.Get "js/popover.js" | resources.Fingerprint "md5" | resources.Minify }}
    <script src="{{$popover.Permalink}}"></script>
  {{ end }}
```

- [ ] **Step 2: Verify the CSS is referenced**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
hugo --quiet && grep -o '/brain/css/popover-v2\.[a-z0-9]*\.min\.css' public/index.html | head -1
```

Expected: one match like `/brain/css/popover-v2.<hash>.min.css`.

- [ ] **Step 3: Verify the popover styles still exist in compiled CSS**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
ls public/css/ 2>/dev/null && grep -o "\.popover-v2[a-z-]*" public/css/popover-v2.*.min.css | sort -u | head -5
```

Expected: at least `.popover-v2`, `.popover-v2-dropin`, `.popover-v2-inner`.

- [ ] **Step 4: Commit**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
git add layouts/partials/head.html
git commit -m "$(cat <<'EOF'
feat(popover): load popover-v2.css as standalone stylesheet in brain

Prepares for sharing the same plain-CSS file with blog and book.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Parametrize selector in popover-v2.js

`window.initPopoverV2` becomes `window.initPopoverV2(opts)` where `opts.selector` overrides the default. Default is the existing brain selector so no other caller has to change.

**Files:**
- Modify: `second-brain-public/assets/js/popover-v2.js` (end of file, the existing `initPopoverV2` definition)

- [ ] **Step 1: Update the init function signature**

Open `second-brain-public/assets/js/popover-v2.js`. Find the existing block at the end:

```js
  window.initPopoverV2 = function initPopoverV2() {
    const links = document.querySelectorAll("a.internal-link[href]");
    links.forEach((link) => {
      link.addEventListener("mouseenter", onMouseEnter);
      link.addEventListener("mouseleave", onMouseLeave);
    });
  };
```

Replace with:

```js
  window.initPopoverV2 = function initPopoverV2(opts) {
    const selector = (opts && opts.selector) || "a.internal-link[href]";
    const links = document.querySelectorAll(selector);
    links.forEach((link) => {
      link.addEventListener("mouseenter", onMouseEnter);
      link.addEventListener("mouseleave", onMouseLeave);
    });
  };
```

- [ ] **Step 2: Verify the file still parses**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
node --check assets/js/popover-v2.js && echo "parse OK"
```

Expected: `parse OK`.

- [ ] **Step 3: Verify brain init call still works (no opts passed)**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
grep "initPopoverV2" layouts/partials/head.html
```

Expected output:
```
      if (window.initPopoverV2) { window.initPopoverV2(); }
```

The empty-args call falls through to the default selector `a.internal-link[href]`. No template change needed.

- [ ] **Step 4: Browser regression check (manual, do once before next task)**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
make serve
```

Open any brain note in browser, hover an internal link → v2 popover renders as today. Stop server with Ctrl+C.

- [ ] **Step 5: Commit**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
git add assets/js/popover-v2.js
git commit -m "$(cat <<'EOF'
feat(popover): parametrize selector in initPopoverV2

Lets blog and book reuse the script with their own brain-link selector.
Default unchanged (a.internal-link[href]).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Add scoped CORS to brain .htaccess

Adds an `Access-Control-Allow-Origin` echo for `ssp.sh` and `dedp.online` origins, plus `Vary: Origin` so Bunny CDN caches per-origin.

**Files:**
- Modify: `second-brain-public/static/.htaccess` (top of file, after `RewriteEngine On`)

- [ ] **Step 1: Add the CORS block**

Open `second-brain-public/static/.htaccess`. The file currently starts with:

```apache
RewriteEngine On

# Redirect renamed brain articles to new URL
```

Insert the CORS block between `RewriteEngine On` and the first comment line:

```apache
RewriteEngine On

# Cross-origin allowlist for popover previews (blog, book, brain).
# Vary: Origin is load-bearing for the Bunny CDN — without it Bunny may
# serve one origin's cached response to another, breaking CORS for the
# second origin. After deploying, purge brain pull-zone cache once so
# pre-CORS entries are evicted.
<IfModule mod_headers.c>
  SetEnvIf Origin "^https://(www\.)?(ssp\.sh|dedp\.online)$" CORS_ALLOWED_ORIGIN=$0
  Header set Access-Control-Allow-Origin "%{CORS_ALLOWED_ORIGIN}e" env=CORS_ALLOWED_ORIGIN
  Header merge Vary "Origin"
</IfModule>

# Redirect renamed brain articles to new URL
```

- [ ] **Step 2: Verify the .htaccess copies to public/ on hugo build**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
hugo --quiet && grep -A3 "Access-Control-Allow-Origin" public/.htaccess
```

Expected output:
```
  Header set Access-Control-Allow-Origin "%{CORS_ALLOWED_ORIGIN}e" env=CORS_ALLOWED_ORIGIN
  Header merge Vary "Origin"
</IfModule>
```

- [ ] **Step 3: Commit**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
git add static/.htaccess
git commit -m "$(cat <<'EOF'
feat(brain): scoped CORS allowlist for ssp.sh + dedp.online

Enables fetch-based popover previews from blog (same-origin already, but
included for completeness) and book (cross-origin). Vary: Origin tells
Bunny CDN to cache per-origin so cross-origin requests don't get a
poisoned response.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

**Phase 1 complete.** Brain has plain-CSS popover, parametrized selector, and CORS headers ready in `static/.htaccess`. Live deployment of the .htaccess happens in Phase 4.

---

## Phase 2: Blog integration

### Task 5: Add sync-popover target to blog Makefile

**Files:**
- Modify: `sspaeti-hugo-blog/Makefile`

- [ ] **Step 1: Add target near other prepare-like targets**

Open `sspaeti-hugo-blog/Makefile`. After the existing `prepare:` target (around line ~10), add:

```makefile
BRAIN_REPO ?= /home/sspaeti/git/sspaeti.com/second-brain-public

sync-popover: ## copy canonical popover assets from second-brain-public into blog
	@mkdir -p assets/css assets/js
	cp $(BRAIN_REPO)/assets/css/popover-v2.css                assets/css/popover-v2.css
	cp $(BRAIN_REPO)/assets/js/popover-v2.js                  assets/js/popover-v2.js
	cp $(BRAIN_REPO)/assets/js/floating-ui.core.umd.min.js    assets/js/floating-ui.core.umd.min.js
	cp $(BRAIN_REPO)/assets/js/floating-ui.dom.umd.min.js     assets/js/floating-ui.dom.umd.min.js
	@echo "synced popover assets from $(BRAIN_REPO)"
```

- [ ] **Step 2: Make `prepare` depend on `sync-popover`**

Still in `sspaeti-hugo-blog/Makefile`, find the `prepare:` target:

```makefile
prepare: ## prepare commands
	rm -rf public
	echo "public deleted.."
	mkdir -p static/indices
	hugo-obsidian -input=content/posts -output=static/indices -index=true -root=.
	python3 helper-scripts/enrich-link-index.py
```

Change to:

```makefile
prepare: sync-popover ## prepare commands
	rm -rf public
	echo "public deleted.."
	mkdir -p static/indices
	hugo-obsidian -input=content/posts -output=static/indices -index=true -root=.
	python3 helper-scripts/enrich-link-index.py
```

(Only the `prepare:` line changed — added `sync-popover` as prerequisite.)

- [ ] **Step 3: Run sync target**

```bash
cd /home/sspaeti/git/sspaeti.com/sspaeti-hugo-blog
make sync-popover
```

Expected output:
```
synced popover assets from /home/sspaeti/git/sspaeti.com/second-brain-public
```

- [ ] **Step 4: Verify all 4 files are present**

```bash
cd /home/sspaeti/git/sspaeti.com/sspaeti-hugo-blog
ls -la assets/css/popover-v2.css assets/js/popover-v2.js assets/js/floating-ui.core.umd.min.js assets/js/floating-ui.dom.umd.min.js
```

Expected: 4 files listed, all > 0 bytes.

- [ ] **Step 5: Commit**

```bash
cd /home/sspaeti/git/sspaeti.com/sspaeti-hugo-blog
git add Makefile assets/css/popover-v2.css assets/js/popover-v2.js assets/js/floating-ui.core.umd.min.js assets/js/floating-ui.dom.umd.min.js
git commit -m "$(cat <<'EOF'
feat(popover): add sync-popover target and import canonical assets

Pulls popover-v2.{js,css} and FloatingUI vendor files from second-brain
repo. `make prepare` runs sync first so assets are always current.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: Add brain-link class to blog wikilinks

`process-wikilinks.html` currently emits `<a href="...">text</a>` for both aliased and plain wikilinks. We add `class="brain-link"` to both so the popover script can find them with `a.brain-link[href]`.

**Files:**
- Modify: `sspaeti-hugo-blog/layouts/partials/process-wikilinks.html` (two `<a>` template strings)

- [ ] **Step 1: Update both anchor templates**

Open `sspaeti-hugo-blog/layouts/partials/process-wikilinks.html`. Find the aliased-link replacement (around line 18):

```go-html-template
    {{- $replacement := printf `<a href="%s">%s</a>` $fullURL $displayText -}}
```

Change to:

```go-html-template
    {{- $replacement := printf `<a href="%s" class="brain-link">%s</a>` $fullURL $displayText -}}
```

Find the plain-link replacement (around line 27):

```go-html-template
    {{- $replacement := printf `<a href="%s">%s</a>` $fullURL $linkText -}}
```

Change to:

```go-html-template
    {{- $replacement := printf `<a href="%s" class="brain-link">%s</a>` $fullURL $linkText -}}
```

- [ ] **Step 2: Rebuild a sample post and verify the class is emitted**

```bash
cd /home/sspaeti/git/sspaeti.com/sspaeti-hugo-blog
hugo --quiet 2>&1 | head -10
grep -orE '<a href="https://ssp\.sh/brain/[^"]+" class="brain-link"' public/posts/ 2>/dev/null | head -5
```

Expected: at least one match showing `class="brain-link"` paired with a brain URL. If your `content/posts/` has no `[[wikilinks]]` to brain yet, pick a post that does have them and verify.

If unsure which posts have wikilinks, run:
```bash
grep -lE "\[\[" /home/sspaeti/git/sspaeti.com/sspaeti-hugo-blog/content/posts/*.md 2>/dev/null | head -3
```

- [ ] **Step 3: Commit**

```bash
cd /home/sspaeti/git/sspaeti.com/sspaeti-hugo-blog
git add layouts/partials/process-wikilinks.html
git commit -m "$(cat <<'EOF'
feat(wikilinks): emit class=brain-link so popover script can target them

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 7: Load popover assets + init in blog assets.html

**Files:**
- Modify: `sspaeti-hugo-blog/layouts/partials/assets.html` (end of file, near the existing `external-links.js` block)

- [ ] **Step 1: Add the popover loading block**

Open `sspaeti-hugo-blog/layouts/partials/assets.html`. Find the last visible block before `{{- partial "plugin/analytics.html" . -}}`:

```html
{{- /* External Links Script */ -}}
{{ $extLinks := resources.Get "js/external-links.js" | fingerprint }}
<script src="{{ $extLinks.RelPermalink }}" integrity="{{ $extLinks.Data.Integrity }}"></script>

{{- partial "plugin/analytics.html" . -}}
```

Insert the popover block between `external-links` and analytics:

```html
{{- /* External Links Script */ -}}
{{ $extLinks := resources.Get "js/external-links.js" | fingerprint }}
<script src="{{ $extLinks.RelPermalink }}" integrity="{{ $extLinks.Data.Integrity }}"></script>

{{- /* Brain popover previews (canonical source: second-brain-public) */ -}}
{{ $popoverCss := resources.Get "css/popover-v2.css" | resources.Fingerprint "md5" | resources.Minify }}
<link rel="stylesheet" href="{{ $popoverCss.RelPermalink }}">
{{ $fuiCore := resources.Get "js/floating-ui.core.umd.min.js" | resources.Fingerprint "md5" }}
{{ $fuiDom  := resources.Get "js/floating-ui.dom.umd.min.js"  | resources.Fingerprint "md5" }}
{{ $popoverJs := resources.Get "js/popover-v2.js" | resources.Fingerprint "md5" | resources.Minify }}
<script src="{{ $fuiCore.RelPermalink }}"></script>
<script src="{{ $fuiDom.RelPermalink }}"></script>
<script src="{{ $popoverJs.RelPermalink }}"></script>
<script>
  document.addEventListener("DOMContentLoaded", function () {
    if (window.initPopoverV2) {
      window.initPopoverV2({ selector: "a.brain-link[href]" });
    }
  });
</script>

{{- partial "plugin/analytics.html" . -}}
```

- [ ] **Step 2: Build and verify assets are linked**

```bash
cd /home/sspaeti/git/sspaeti.com/sspaeti-hugo-blog
hugo --quiet 2>&1 | head -10
echo "--- popover css link:"
grep -o "/css/popover-v2\.[a-z0-9]*\.min\.css" public/index.html | head -1
echo "--- popover js script:"
grep -o "/js/popover-v2\.[a-z0-9]*\.min\.js" public/index.html | head -1
echo "--- init call:"
grep -c "initPopoverV2" public/index.html
echo "--- FloatingUI loaded:"
grep -o "/js/floating-ui\.[a-z0-9.]*\.js" public/index.html | sort -u
```

Expected: 1 css link, 1 popover-v2 js script, 1 init call, 2 FloatingUI files (core + dom).

- [ ] **Step 3: Local smoke test (manual)**

```bash
cd /home/sspaeti/git/sspaeti.com/sspaeti-hugo-blog
make run
```

Open a blog post with `[[wikilinks]]` in browser at `http://localhost:1312/posts/<slug>/`. Hover a brain link. Expected: popover renders fetching the brain content.

Open browser devtools Network tab — confirm a `fetch` to `https://www.ssp.sh/brain/<slug>/` returns 200 with HTML.

Stop server with Ctrl+C.

- [ ] **Step 4: Commit**

```bash
cd /home/sspaeti/git/sspaeti.com/sspaeti-hugo-blog
git add layouts/partials/assets.html
git commit -m "$(cat <<'EOF'
feat(popover): load v2 popover assets and init for brain links in blog

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

**Phase 2 complete.** Blog has popover wired locally. Production deploy in Phase 4.

---

## Phase 3: Book integration

### Task 8: Add sync-popover target to book Makefile

**Files:**
- Modify: `book/dedp/Makefile`

- [ ] **Step 1: Add the target**

Open `book/dedp/Makefile`. Find the `brain:` target:

```makefile
brain:
	make -C ~/git/sspaeti.com/second-brain-public/
```

Add this block immediately AFTER the `brain:` target:

```makefile
brain:
	make -C ~/git/sspaeti.com/second-brain-public/

BRAIN_REPO ?= /home/sspaeti/git/sspaeti.com/second-brain-public

sync-popover: ## copy canonical popover assets from second-brain-public into book
	@mkdir -p assets/css assets/js
	cp $(BRAIN_REPO)/assets/css/popover-v2.css                assets/css/popover-v2.css
	cp $(BRAIN_REPO)/assets/js/popover-v2.js                  assets/js/popover-v2.js
	cp $(BRAIN_REPO)/assets/js/floating-ui.core.umd.min.js    assets/js/floating-ui.core.umd.min.js
	cp $(BRAIN_REPO)/assets/js/floating-ui.dom.umd.min.js     assets/js/floating-ui.dom.umd.min.js
	@echo "synced popover assets from $(BRAIN_REPO)"
```

- [ ] **Step 2: Make `serve` depend on `sync-popover`**

In the same Makefile, find:

```makefile
serve: link-index
	rm -rf book/
	mdbook-released serve -p 3333
```

Change to:

```makefile
serve: link-index sync-popover
	rm -rf book/
	mdbook-released serve -p 3333
```

- [ ] **Step 3: Run sync and verify**

```bash
cd /home/sspaeti/git/book/dedp
make sync-popover
ls -la assets/css/popover-v2.css assets/js/popover-v2.js assets/js/floating-ui.core.umd.min.js assets/js/floating-ui.dom.umd.min.js
```

Expected: 4 files, all > 0 bytes.

- [ ] **Step 4: Commit**

```bash
cd /home/sspaeti/git/book/dedp
git add Makefile assets/css/popover-v2.css assets/js/popover-v2.js assets/js/floating-ui.core.umd.min.js assets/js/floating-ui.dom.umd.min.js
git commit -m "$(cat <<'EOF'
feat(popover): add sync-popover target and import canonical assets

Pulls popover-v2.{js,css} and FloatingUI vendor files from second-brain
repo. `make serve` runs sync first so assets are always current.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 9: Create popover-v2-init.js for the book

mdBook has no head-template mechanism — to initialize the popover, we ship an extra tiny JS file that mdBook loads alongside the popover script.

**Files:**
- Create: `book/dedp/assets/js/popover-v2-init.js`

- [ ] **Step 1: Create the init file**

Create `book/dedp/assets/js/popover-v2-init.js` with content:

```js
// Initialize the brain popover for the book. Selector matches the
// brain-link class emitted by mdbook-sspaeti for wikilinks pointing at
// ssp.sh/brain. Loaded AFTER popover-v2.js so window.initPopoverV2 exists.
document.addEventListener("DOMContentLoaded", function () {
  if (window.initPopoverV2) {
    window.initPopoverV2({ selector: "a.brain-link[href]" });
  }
});
```

- [ ] **Step 2: Verify JS parses**

```bash
cd /home/sspaeti/git/book/dedp
node --check assets/js/popover-v2-init.js && echo "parse OK"
```

Expected: `parse OK`.

- [ ] **Step 3: Commit**

```bash
cd /home/sspaeti/git/book/dedp
git add assets/js/popover-v2-init.js
git commit -m "$(cat <<'EOF'
feat(popover): add init script for book

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 10: Register popover assets in book.toml

**Files:**
- Modify: `book/dedp/book.toml`

- [ ] **Step 1: Add CSS and JS to additional-css / additional-js**

Open `book/dedp/book.toml`. Find the `[output.html]` section:

```toml
[output.html]
additional-css = ["./assets/css/mdbook-admonish.css", "./theme/catppuccin.css", "./theme/catppuccin-highlight.css", "./assets/css/style-pagedoc.css", "./assets/css/customize.css", "./assets/css/graph-view.css"]
# theme = "kanagawa"
default-theme = "light"
preferred-dark-theme = "pinkrose"
additional-js = ["./assets/js/sidebar.js", "mermaid.min.js", "mermaid-init.js", "d3.v7.min.js", "linkIndex.js", "./assets/js/graph-view.js", "./assets/js/graph-view-full.js", "./assets/js/external-links.js"]
```

Append popover-v2.css to `additional-css`:

```toml
additional-css = ["./assets/css/mdbook-admonish.css", "./theme/catppuccin.css", "./theme/catppuccin-highlight.css", "./assets/css/style-pagedoc.css", "./assets/css/customize.css", "./assets/css/graph-view.css", "./assets/css/popover-v2.css"]
```

Append FloatingUI + popover-v2 + init to `additional-js` IN THIS ORDER (FloatingUI must load before popover-v2.js, init must load last):

```toml
additional-js = ["./assets/js/sidebar.js", "mermaid.min.js", "mermaid-init.js", "d3.v7.min.js", "linkIndex.js", "./assets/js/graph-view.js", "./assets/js/graph-view-full.js", "./assets/js/external-links.js", "./assets/js/floating-ui.core.umd.min.js", "./assets/js/floating-ui.dom.umd.min.js", "./assets/js/popover-v2.js", "./assets/js/popover-v2-init.js"]
```

- [ ] **Step 2: Build the book and verify**

```bash
cd /home/sspaeti/git/book/dedp
make sync-popover
mdbook-released build 2>&1 | tail -5 || mdbook build 2>&1 | tail -5
```

Expected: build succeeds.

- [ ] **Step 3: Verify the assets are referenced in book output**

```bash
cd /home/sspaeti/git/book/dedp
grep -o "popover-v2\.[a-z]*" book/index.html 2>/dev/null | sort -u | head -5
grep -o "floating-ui\.[a-z.]*\.js" book/index.html 2>/dev/null | sort -u
```

Expected: at least `popover-v2.css`, `popover-v2.js`, `popover-v2-init.js`, plus the two FloatingUI files.

- [ ] **Step 4: Verify a chapter has brain-link anchors**

```bash
cd /home/sspaeti/git/book/dedp
grep -ol "class=\"brain-link\"" book/ -r 2>/dev/null | head -3
```

Expected: at least one chapter HTML file contains `class="brain-link"`. If none, the book test will require checking after content with brain wikilinks is published.

- [ ] **Step 5: Local smoke test (manual)**

```bash
cd /home/sspaeti/git/book/dedp
make serve-book-brain
```

Wait for both servers to come up. Open a book chapter with brain wikilinks in browser (`http://localhost:3333/...`). Hover a brain-link. Expected: popover renders.

Note: book.toml currently has `brain-base-url = "http://localhost:1313/brain"`, so during local dev the popover fetches from the locally-running brain (same machine). For production builds, the user flips this to the live brain URL — that triggers cross-origin CORS, which works after Phase 4 (.htaccess deployed).

Stop both servers with Ctrl+C.

- [ ] **Step 6: Commit**

```bash
cd /home/sspaeti/git/book/dedp
git add book.toml
git commit -m "$(cat <<'EOF'
feat(popover): register popover-v2 assets in book.toml

Order matters: FloatingUI core + dom load before popover-v2.js;
popover-v2-init.js loads last so window.initPopoverV2 exists.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

**Phase 3 complete.** Book has popover wired locally.

---

## Phase 4: Production deploy + verification

**STOP — do not execute Phase 4 tasks without explicit user confirmation.** These push to production servers.

### Task 11: Deploy brain to push CORS .htaccess live

**Files:** none (executes deploy)

- [ ] **Step 1: Confirm with user that brain is ready to deploy**

The brain branch `popover-v2` currently has the .htaccess CORS change committed. Confirm user wants to deploy this to production via `make deploy`.

- [ ] **Step 2: Deploy brain**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
make deploy
```

Wait for completion.

- [ ] **Step 3: Confirm .htaccess on server**

```bash
curl -sI -H "Origin: https://dedp.online" https://www.ssp.sh/brain/zettelkasten/ | grep -i "access-control\|vary"
```

Expected output (after Bunny purge in next task):
```
access-control-allow-origin: https://dedp.online
vary: Origin, Accept-Encoding
```

If the headers are missing AND you see a `cdn-cache: HIT` header, the cached pre-CORS response is being served — Task 12 (purge) will fix it.

---

### Task 12: Purge Bunny brain pull-zone cache

Pre-CORS cached responses must be evicted so subsequent requests pick up the new headers.

**Files:** none

- [ ] **Step 1: Purge via existing Makefile target**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
make purge-cdn
```

(This calls Bunny's purge API for the ssp.sh pull zone, which serves brain.)

Expected output:
```
CDN cache purged for ssp.sh
```

If the env vars are missing, the target prints a skip message — you'll need to set `BUNNY_API_KEY` and `BUNNY_ZONE_SSP` and re-run, or purge manually in Bunny dashboard.

- [ ] **Step 2: Verify CORS headers after purge**

```bash
curl -sI -H "Origin: https://dedp.online" https://www.ssp.sh/brain/zettelkasten/ | grep -i "access-control\|vary\|cache"
```

Expected:
- `access-control-allow-origin: https://dedp.online`
- `vary: Origin` (possibly merged with other Vary values like `Accept-Encoding`)

- [ ] **Step 3: Negative test — disallowed origin gets no CORS header**

```bash
curl -sI -H "Origin: https://attacker.example" https://www.ssp.sh/brain/zettelkasten/ | grep -i "access-control"
```

Expected: NO output (no header echoed for disallowed origin).

- [ ] **Step 4: Per-origin caching test**

```bash
echo "--- ssp.sh:"
curl -sI -H "Origin: https://www.ssp.sh" https://www.ssp.sh/brain/zettelkasten/ | grep -i "access-control"
echo "--- dedp.online:"
curl -sI -H "Origin: https://dedp.online" https://www.ssp.sh/brain/zettelkasten/ | grep -i "access-control"
```

Expected: each returns the matching origin in its `access-control-allow-origin` header. If they return the same origin, Bunny isn't varying — fall back to Bunny Edge Rules.

---

### Task 13: Deploy blog and verify popover in production

**Files:** none (deploys blog)

- [ ] **Step 1: Confirm user is ready to deploy blog**

Blog `popover-brain` branch has wikilink class change + sync-popover + assets.html wiring + synced canonical files.

- [ ] **Step 2: Merge popover-brain to master (or whichever branch the user deploys from)**

```bash
cd /home/sspaeti/git/sspaeti.com/sspaeti-hugo-blog
git checkout master
git merge popover-brain
```

(Ask the user before merging — they may want to keep on the feature branch and deploy from there.)

- [ ] **Step 3: Deploy blog**

```bash
cd /home/sspaeti/git/sspaeti.com/sspaeti-hugo-blog
make hugo-generate
# whichever deploy command the user uses (e.g. make deploy, make upload)
```

- [ ] **Step 4: Production smoke test**

In a browser, open any blog post with brain wikilinks (e.g. `https://www.ssp.sh/posts/<slug>/`). Hover a brain link. Expected: popover renders fetching `https://www.ssp.sh/brain/<slug>/`.

Open browser devtools Network tab — confirm `fetch` to brain returns 200 with `text/html` and the CORS header (technically not needed for same-origin, but should be present).

- [ ] **Step 5: Purge blog CDN if needed**

```bash
cd /home/sspaeti/git/sspaeti.com/sspaeti-hugo-blog
make purge-cdn
```

(Same pull zone as brain since both are under `ssp.sh`.)

---

### Task 14: Deploy book and verify popover in production

**Files:**
- Modify: `book/dedp/book.toml` (flip `brain-base-url` from localhost to live)

- [ ] **Step 1: Flip brain-base-url to production**

Open `book/dedp/book.toml`. Find:

```toml
# brain-base-url = "https://ssp.sh/brain" #live
brain-base-url = "http://localhost:1313/brain" # local
```

Swap which line is commented:

```toml
brain-base-url = "https://www.ssp.sh/brain" #live
# brain-base-url = "http://localhost:1313/brain" # local
```

(Note: use `www.ssp.sh` not `ssp.sh` to avoid the 301 redirect adding latency.)

- [ ] **Step 2: Confirm user is ready to deploy book**

Book `popover-brain` branch has sync-popover + init file + book.toml registration.

- [ ] **Step 3: Build and deploy book**

```bash
cd /home/sspaeti/git/book/dedp
make link-index sync-popover
mdbook-released build  # or whichever build command
# whichever upload/deploy command the user uses
```

- [ ] **Step 4: Production smoke test**

Open a book chapter with brain wikilinks in browser at `https://dedp.online/<chapter>/`. Hover a brain link. Expected: popover renders.

Devtools Network tab: confirm `fetch` to `https://www.ssp.sh/brain/<slug>/` returns 200 AND the response has `access-control-allow-origin: https://dedp.online`. If browser console shows a CORS error, Bunny isn't varying — re-run `make purge-cdn` in brain and try again.

- [ ] **Step 5: Purge book CDN if needed**

```bash
cd /home/sspaeti/git/book/dedp
make purge-cdn
```

- [ ] **Step 6: Commit the brain-base-url flip**

```bash
cd /home/sspaeti/git/book/dedp
git add book.toml
git commit -m "$(cat <<'EOF'
chore(book): point brain-base-url at production for deploy

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

**Phase 4 complete.** Brain CORS live, blog popover live, book popover live.

---

## Out of scope (deliberately deferred)

1. **Newsletter (Listmonk at `list.ssp.sh`)** — JS only runs in web archive views, not email clients. If/when adding: also add `list.ssp.sh` to the CORS allowlist regex in brain `.htaccess`, upload popover JS+CSS via Listmonk admin `custom.js`/`custom.css`, verify wikilink rendering in campaigns.

2. **LaTeX rendering inside popover** — known gap from brain implementation; would require including KaTeX in popover-v2.js. Not blocking, deferred.

3. **Image previews on blog/book** — per design, blog/book don't rewrite `[[image.ext]]` wikilinks. The image branch in popover-v2.js still works if someone manually adds an `<a href="image.webp" class="brain-link">` — that comes free.

4. **Removal of brain v1 popover** — kept behind feature flag per prior user request.

---

## Self-review notes

- **Spec coverage:** Each spec section is mapped:
  - Architecture flow → Task 4 (CORS) + Tasks 7/10 (init JS)
  - Canonical source location → Tasks 1, 2, 3, 5, 8
  - Plain-CSS conversion → Task 1
  - Selector parametrization → Task 3
  - Build-time copy → Tasks 5, 8
  - Brain CORS → Task 4 (commit) + Tasks 11, 12 (deploy + purge)
  - Blog wikilink class → Task 6
  - Blog head loading → Task 7
  - Book Makefile + init + book.toml → Tasks 8, 9, 10
  - Bunny CDN considerations → Task 12
  - Verification → embedded in each task + Tasks 12 Step 3-4, 13 Step 4, 14 Step 4
- **Placeholder scan:** no TBDs; every code block is concrete.
- **Type/identifier consistency:** `popover-brain` branch name consistent across blog+book; `sync-popover` target name consistent across blog+book; `initPopoverV2({selector: "a.brain-link[href]"})` call signature consistent; `BRAIN_REPO` env var consistent.
