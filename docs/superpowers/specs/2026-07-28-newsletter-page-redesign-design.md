# Newsletter landing page redesign — Design

**Date:** 2026-07-28
**File under design:** `content/newsletter.md` (rendered at `/newsletter`, the canonical subscribe page)

## Goal

Turn the newsletter note into a proper subscribe **landing page** that people actually
convert on. It replaces `subscribe.ssp.sh` as the place to sign up; `newsletter.ssp.sh`
forwards here. Two concrete problems to fix:

1. Today the page only offers a plain link-button that bounces visitors to
   `subscribe.ssp.sh`. Replace it with the **working inline Listmonk form** so people
   subscribe without leaving the page.
2. Make the "what this is / why subscribe" clearer and more visually appealing, in the
   site's light/dark theme, **without losing the author's writing**.

Chosen visual direction: **split hero** (pitch + form left, "You'll get" checklist card
right) with the author's existing prose kept below it. The hero also carries small,
supporting trust elements — the `ssp.sh` logo + wordmark above the headline, and a small
circular photo of the author as a byline under the form (Option 1 from the mockups; kept
small so the form stays the focus).

## Constraints

- It's a Quartz/Hugo note. Do not touch site navigation or footer.
- Keep bulk HTML out of `newsletter.md`. Heavy markup goes in a shortcode; visuals go in
  the SCSS file. Prose stays as markdown so it reads as the author's writing.
- Must work in both light and dark mode using the existing theme tokens in
  `assets/styles/custom.scss` (Kanagawa-style remap under `[saved-theme="dark"]`).
- The form must post to Listmonk exactly like the existing `newsletter-footer.html`
  partial (same list, same captcha).

## Files changed

### 1. New shortcode: `layouts/shortcodes/newsletter-hero.html`

Holds all the heavy markup: the split hero, the inline Listmonk form, and the checklist
card. Rendered from `newsletter.md` via `{{< newsletter-hero >}}`.

Structure:

```
<div class="nl-hero">
  <div class="nl-hero-pitch">
    <div class="nl-hero-brand">
      <!-- logo via Hugo asset pipeline, same pattern as partials/newsletter.html -->
      {{ $logo := resources.Get "svg/sspaeti-color.svg" }}
      <img class="nl-hero-logo" src="{{ $logo.RelPermalink }}" alt="ssp.sh logo" width="30" height="30" />
      <span class="nl-hero-wordmark">ssp.sh</span>
    </div>
    <h1>The ssp.sh Newsletter</h1>
    <p class="nl-hero-oneliner">Data engineering, writing & a public Second Brain
       — written by hand, to you.</p>

    <!-- Listmonk form: fields copied verbatim from newsletter-footer.html -->
    <form method="post" action="https://list.ssp.sh/subscription/form"
          class="listmonk-form nl-hero-form" target="_top">
      <input type="hidden" name="nonce" />
      <div class="nl-hero-row">
        <input type="email" name="email" required placeholder="your@email.com"
               class="nl-hero-email" />
        <input type="submit" value="Subscribe free" class="nl-hero-btn" />
      </div>
      <input type="hidden" name="l" checked
             value="4625c037-fda8-45f7-9a66-47d765c6562b" />
      <div class="captcha nl-hero-captcha">
        <altcha-widget challengeurl="https://list.ssp.sh/api/public/captcha/altcha"></altcha-widget>
        <script type="module" src="https://list.ssp.sh/public/static/altcha.umd.js" async defer></script>
      </div>
    </form>

    <p class="nl-hero-micro">Free · no schedule · only when I have something worth sharing.</p>
    <p class="nl-hero-micro">You'll get a confirmation email — click the link to finish.</p>

    <div class="nl-hero-byline">
      <img src="/images/me_alps.jpg" alt="Simon Späti" width="38" height="38" />
      <div class="nl-hero-byline-who">
        <b>Simon Späti</b><br>
        <span>data engineer, author &amp; technical writer</span>
      </div>
    </div>
  </div>

  <aside class="nl-hero-card">
    <div class="nl-hero-card-k">You'll get</div>
    <ul>
      <li>Data engineering deep dives</li>
      <li>Second Brain note digests</li>
      <li>Book & writing progress</li>
      <li>Books I'm reading</li>
    </ul>
  </aside>
</div>

<p class="nl-hero-cred">Written by Simon Späti — 20+ years in data engineering, author of
   <em>Patterns of Data Engineering</em>, featured in Towards Data Science & freeCodeCamp.</p>
```

Notes:
- **Captcha placement:** the altcha widget sits *below* the email+submit row (inside the
  form, after the hidden list input) so the hero stays compact and the widget does not
  push the button around. It should render small and unobtrusive; if altcha renders large,
  scale/reduce its footprint via `.nl-hero-captcha` in SCSS.
- **Confirm-inbox cue** is the second `.nl-hero-micro` line (handles Listmonk double
  opt-in so people finish signing up).
- **Credibility line** is the muted `.nl-hero-cred` line directly under the hero box.
- **Logo** is loaded through Hugo's asset pipeline from `assets/svg/sspaeti-color.svg`
  (same `resources.Get` approach as `partials/newsletter.html`), rendered ~30px next to the
  `ssp.sh` wordmark. The wordmark is a **single color** (`var(--gray)`) — no two-tone.
- **Byline photo** is a small (~38px) circular image of the author placed under the form
  (Option 1). Source: `static/images/me_alps.jpg` (see asset step below), referenced as
  `/images/me_alps.jpg`.
- Content strings are hardcoded in the shortcode (single-use page). No parameters needed.

### Asset: copy the author photo into the brain repo

`me_alps.jpg` currently only lives in the blog repo
(`sspaeti-hugo-blog/static/images/me_alps.jpg`). Copy it into this repo at
`static/images/me_alps.jpg` so the newsletter page is self-contained and works under
`make serve` without depending on the blog deploy. Referenced as `/images/me_alps.jpg`.

### 2. Styles: `assets/styles/custom.scss`

Add one new block, mirroring the existing `.nl-footer-*` convention (prefix `.nl-hero-`),
placed near the end with the other newsletter CSS. Requirements:

- `.nl-hero`: CSS grid, `grid-template-columns: 1.2fr .8fr`, gap ~22px, align center.
  Pink-only gradient background (no blue):
  `linear-gradient(135deg, rgba(255,93,98,.12), rgba(210,126,153,.12))`, `border-radius:12px`,
  padding ~28px.
- `@media (max-width: 640px)`: collapse to a single column (`grid-template-columns: 1fr`)
  so the pitch+form appears first and the checklist card stacks below it. Confirmed good on
  mobile.
- `.nl-hero-brand`: inline-flex, center-aligned, gap ~9px, small bottom margin. Holds logo + wordmark.
- `.nl-hero-logo`: ~30×30px, block. `.nl-hero-wordmark`: bold (~800), `font-size:1rem`,
  **single color** `var(--gray)` (whole `ssp.sh` one color, no accent on `.sh`).
- `.nl-hero-oneliner`: ~1.02rem, `var(--gray)`.
- `.nl-hero-byline`: flex row, center-aligned, gap ~10px, top margin ~14px.
  `.nl-hero-byline img`: 38×38px, `border-radius:50%`, `object-fit:cover`, subtle border
  (`var(--light)`) + soft shadow. `.nl-hero-byline-who`: ~0.85rem — name in `var(--gray)`
  (bold), role line in `var(--global-font-secondary-color)`.
- `.nl-hero-form` / `.nl-hero-row`: flex row, wraps; email input flexes (`min-width:180px`),
  `var(--outlinegray)` border, background `var(--global-background-color)`, text `var(--gray)`.
- `.nl-hero-btn`: background `var(--secondary)`, white text in light mode; in
  `[saved-theme="dark"]` set text color `#181820` (same pattern as `.nl-footer-btn`, since
  dark `--secondary` is the muted teal `#658594`).
- `.nl-hero-micro`: ~0.82rem, `var(--global-font-secondary-color)`.
- `.nl-hero-card`: background `var(--light)`, `1px solid var(--outlinegray)`,
  `border-radius:10px`, padding ~18px. `.nl-hero-card-k` = small uppercase label in
  `var(--secondary)`. `ul` list-style none; each `li` uses a `✓` marker in `var(--secondary)`.
- `.nl-hero-cred`: ~0.88rem, muted (`var(--global-font-secondary-color)`), small top margin.
- `.nl-curl`: de-blued note box — `background: var(--lightgray)`,
  `1px solid var(--outlinegray)`, `border-radius:8px`, padding ~12px 16px, ~0.9rem. **No**
  colored left border, **no** gradient, **no** box-shadow. Inline `code` gets a subtle
  `var(--global-background-color)` background + `var(--outlinegray)` border.
- Dark-mode input styling: reuse the same treatment as `.nl-footer-email`
  (`background:#2A2A37; border-color:#54546D; color:#DCD7BA`).

### 3. Page: `content/newsletter.md`

Keep frontmatter as-is. New body order:

1. `{{< newsletter-hero >}}`
2. Author's two paragraphs (real markdown prose):
   > Every now and then — a new article, notes I've been working on in my Second Brain, or
   > just thoughts I want to send your way. Each edition is a handy overview, so you don't
   > have to keep checking my brain or socials yourself.
   >
   > Inside you'll find deep dives into data engineering, a digest of recent notes, progress
   > on my book *Patterns of Data Engineering*, and books I'm reading.
3. Curl note — a single-line div (tiny, not bulky):
   `<div class="nl-curl">💡 You can also subscribe via <code>curl sub.ssp.sh/your@email.com</code> (or <code>curl sub.ssp.sh/why</code> to see why to follow).</div>`
4. Short past-editions line (markdown):
   > Most past editions are email-only — see the backlinks below for a few, and subscribe to
   > catch the next one.
5. `Origin: [subscribe.ssp.sh](https://subscribe.ssp.sh)` line kept.
6. **All original page text preserved** at the bottom inside an HTML comment
   (`<!-- ... -->`), including the old `## What Makes This Newsletter Different?` section, the
   original bullet list, the old admonition, and the old `<a>` subscribe button — so wording
   can be reworked later without re-deriving it.

## Out of scope

- Unsubscribe-anytime microcopy and a sample-edition link (considered, declined).
- Any change to the `subscribe.ssp.sh` / `newsletter.ssp.sh` forwarding config itself
  (DNS/redirect lives outside this repo).
- Subscriber-count social proof (no data source).
- Changes to the footer CTA partial (`newsletter-footer.html`) — left untouched; this new
  hero reuses its Listmonk field values only.

## Verification

- `make run` / `make serve`, open `/newsletter`, toggle the site's light/dark switch:
  hero, logo + wordmark, byline photo, form, checklist, curl note, and credibility line all
  readable in both modes. Logo reads well on the pink gradient in both themes.
- The `ssp.sh` wordmark is a single color (not two-tone).
- The byline photo (`/images/me_alps.jpg`) loads locally under `make serve` (i.e. the copy
  in this repo's `static/images/` is present, not depending on the blog deploy).
- Narrow the window: hero collapses to one column, form before checklist.
- Submit a test email: lands on Listmonk's confirmation flow on the same list ID as the
  footer form; confirmation email arrives.
