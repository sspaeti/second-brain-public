# Note categories: generated from the link graph, curated by the author

**Date:** 2026-11-06
**Status:** Design reviewed (builder check 2026-11-06), ready for planning

## Problem

The public brain has ~714 notes and no topic structure. Readers who like one Obsidian note have
no way to find "the other Obsidian notes"; the only navigation is wikilinks, backlinks and search.
On the stats side (`my-personal-social-stats-dashboard`, GoatCounter data) the same gap shows up:
below the section split (`/brain` vs `/blog`) the next level is single pages, a long tail of
pages with five or fewer visits that says nothing about *what* people come for (Obsidian? data
engineering? coffee?).

The author does not want to tag notes or maintain categories per note. Only 24 of 714 notes carry
any tag, and the vault's existing taxonomy is rarely kept up to date. The precedent is note
status (`docs/superpowers/specs/2026-10-01-note-status-design.md`): computed from data the notes
already produce, explainable per note, overridable when wrong, zero per-note work.

## Key insight

The wikilink graph already encodes topics. A throwaway label-propagation run over the 714 notes
(2,964 undirected link edges) produced coherent clusters whose most-linked notes name the topic:

| size | hub notes | obvious category |
|---|---|---|
| 145 | obsidian, markdown, neovim, second brain | PKM & tools |
| 76 | data engineering, sql, dbt | Data engineering |
| 60 | duckdb, data lake table format, delta lake | Data lakes & formats |
| 36 | amaro gayo, asimba, bandeira | Coffee |
| 32 | will ai replace humans, vibe coding, ai writing | AI & writing |
| 31 | data modeling, dimensional modeling | Data modeling |
| 29 | writing, deep work, productivity | Writing & productivity |
| 21 | dagster, data orchestrators, airflow | Orchestration |
| 20 | semantic layer, metrics layer, cube | Semantic layer |
| 18 | olap, cloud data warehouses | OLAP & warehouses |
| … | 65 smaller communities, 17 isolated notes | |

Raw communities are not usable as-is: there are 75 of them, membership shifts between runs, and
the hub name is sometimes a poor label ("amaro gayo" for coffee). So the design splits the work:

1. **Generate** a proposal from the graph (automated, re-runnable).
2. **Curate** the category list once by hand: which exist, their names, their order of importance,
   and the hub notes that anchor each (the author's "final say").
3. **Assign** every note to one category automatically from the curated hubs, on every build.

## Vocabulary

- **Category**: one of ~8-12 author-curated topics. Lowercase slug for URLs and data
  (`data-engineering`), human name for display (`Data engineering`).
- **Hub**: a note whose link neighbourhood defines a category. Each category has one or more hubs.
  Hubs are ordinary notes; being a hub changes nothing about how they render.
- **Reason**: a per-note, human-readable sentence explaining the assignment, like the note-status
  `reason` field.

## Goals

- Every public note gets exactly one category, or `other` when the graph gives no signal.
- The author curates the category list once, then only revisits it when the generated report
  suggests a change (a big community with no hub, many notes landing in `other`).
- Readers can browse a category: an index of categories in the curated order, and a page per
  category listing its notes.
- The stats dashboard gets a `topic` level between section and page, so "what do readers come
  for this week" is one chart.

## Non-goals

- No per-note tagging in the vault. The private vault (`$secondbrain`) stays untouched.
- No LLM classification. The assignment must be deterministic and explainable from links alone.
- No multi-category notes. One category per note keeps the reader list and the stats clean.
- Not a replacement for Hugo tags; the few existing tags stay as they are.

## Anti-goals (what would make this a failure)

- Categories that flip between builds. A note must stay put unless its links or the curated list
  change.
- A category list that needs grooming every week.
- "Other" becoming the biggest bucket.

## How it works

### Curated list: `data/categories.yaml` (committed)

Author-owned, ordered by importance (first = most important; order is used for display, for
sorting in the dashboard, and as the tie-break in assignment):

```yaml
- slug: data-engineering
  name: Data engineering
  description: Concepts, patterns, tooling.      # one line, shown on the index page
  hubs: [data engineering, sql, dbt, duckdb, data lake table format]
  blog_categories: [data engineering]           # optional: Hugo blog categories mapped to this
- slug: pkm
  name: Obsidian & second brain
  hubs: [obsidian, second brain, markdown, neovim]
  blog_categories: [productivity]
- slug: ai-writing
  name: AI & writing
  hubs: [will ai replace humans, writing, deep work]
  blog_categories: [writing, reflections]
- slug: coffee
  name: Coffee
  hubs: [coffee]
# ...
```

`hubs` are note filename stems (lowercase, as they appear in `content/`), not slugs, so the
author can copy them from the proposal report. A hub that no longer exists is a build warning, not
an error. `blog_categories` is only read by the dashboard export (see below). The example above is
illustrative; the real first version comes out of the proposal (next section).

**First version and the missing-file case.** The build ships a provisional `data/categories.yaml`
generated from the proposal (names taken from the top hub, one category per community of
size >= 15, ordered by size), committed as the starting point the author then edits. That is the
"generated list, then final say" flow, and it lets the whole chain (assignment, Hugo pages,
dashboard) be built and tested in one go. When the file is absent, `note-category` exits 0 with a
warning, writes no JSON, injects nothing, and Hugo renders no category pages; the site is then
exactly as today. The author reviews the provisional list before the next `make deploy`, since a
deploy publishes the names in every note's meta line.

### Command 1: `obsidian-quartz note-category --propose`

Author-only, run on demand. Reads `content/*.md`, builds the undirected weighted wikilink graph
(resolve `[[target]]`, `[[target|alias]]`, `[[target#heading]]` to a stem; ignore links to
non-existent notes and self-links; weight = number of links between the pair). Runs community
detection with a fixed seed. Writes `note_category_proposal.md` at the repo root (gitignored, next
to `note_status_report.md`; never under `data/`):

- one block per community of size >= 5, ordered by size: size, top 5 hubs by in-degree, 10
  sample members, and, if `data/categories.yaml` exists, which curated category most of its
  members currently land in (so a re-run after curation shows what is still uncovered);
- a short list of isolated notes (no links in or out);
- a suggested starting `categories.yaml` block the author can paste and edit.

The author takes the proposal, renames, merges, drops and orders, and writes `categories.yaml`.
This step is expected to happen once, and again only when the report shows drift.

### Command 2: `obsidian-quartz note-category` (every build)

Runs in `make prepare` right after `note-status`. Seeded label propagation:

1. Label each hub with its category. Hubs are fixed and never relabelled. A note listed as hub
   in two categories is a build error.
2. Repeat until no label changes (cap at 50 rounds): every non-hub note takes the label with the
   highest summed edge weight among its already-labelled neighbours. Ties break by curated order
   (earlier category wins). Unlabelled neighbours do not vote. Process notes in a fixed order
   (sorted stem) so the result is deterministic.
3. Notes still unlabelled after convergence get `other` in the JSON and the report only. `other`
   is never injected as a Hugo term, so there is no public `/brain/categories/other/` junk drawer;
   those notes stay reachable through links and search as today, and the report lists them.
4. Optional author override: a vault tag `#category/<slug>` on a note wins over the computation,
   handled the same way `#garden/<level>` is in `file_utils.rs` (stripped from tags, never a Hugo
   taxonomy term). The design does not require the author to ever use this.

Outputs:

- `data/note_category.json`, keyed by stem like `note_status.json`:
  `{"category": "data-engineering", "name": "Data engineering", "source": "links" | "tag",
  "reason": "Links to data engineering, dbt and 3 more notes in this category", "score": 7}`.
- `categories: [<slug>]` written into the generated front matter of each note in `content/`
  (`content/` is generated and may be changed; the vault is not), so Hugo's built-in
  `categories` taxonomy renders the reader pages with no custom list logic.
- `note_category_report.md` at the repo root (gitignored): counts per category, every note in
  `other`, notes whose winning score is 1 (weak), hubs not found, and the five biggest
  communities from a fresh detection run that have no hub in any category. This is the author's
  signal to revisit `categories.yaml`.

### Reader-facing

- **Note page** (`layouts/_default/single.html`): the category name joins the meta line as a link
  to the category page, right before the status word:
  `Last updated Oct 1, 2026 · Created Oct 1, 2026 · 1 min read · Data engineering · growing`.
  Notes in `other` show no category item.
- **Category index** at `/brain/categories/`: the curated list in curated order, each with name,
  one-line description, note count and the three most-linked notes. Rendered by the existing
  `layouts/_default/taxonomy.html`, extended to read `data/categories.yaml` for order and
  descriptions (Hugo's default order is alphabetical; the curated order is the point).
- **Category page** at `/brain/categories/<slug>/`: Hugo's term page with the existing
  `layouts/tags/term.html` style of list (title, date, status word), sorted by last updated.
- Linked from the brain home page through `layouts/index.html` (next to the recent-notes
  partial), not through `content/_index.md`, which is a vault note that `make prepare` re-copies.
- Templates: categories get their own `layouts/categories/{taxonomy,term}.html`; the existing
  `layouts/tags/term.html` and `layouts/_default/taxonomy.html` are tags-specific (`/tags/%s`)
  and stay untouched. Hugo's `categories` taxonomy is already enabled by default
  (`public/categories/` exists, empty).
- `utils/revert-lastmod-only.sh` must treat the injected `categories:` line like `lastmod`
  (ignore it when deciding whether a note changed), or every note would count as changed after
  the first run.

### Stats dashboard side (`my-personal-social-stats-dashboard`)

- New script `scripts/page_topics.sh`: reads `second-brain-public/data/note_category.json` and
  `data/categories.yaml`, plus the blog's Hugo `categories` front matter (extend
  `scripts/blog_posts.sh`, which already parses that front matter; blog URLs come from the `url:`
  front matter, `/blog/<slug>/`, not from the post folder name), and writes
  `data/page_topics.ndjson`: one row per page `{page: "/brain/<slug>", topic_slug, topic_name,
  topic_order}` and `{page: "/blog/<slug>", ...}`. Brain slugs come from the Rust `brain_slug`
  rules (`utils/obsidian-quartz/src/slug.rs`); the script must use the same mapping, so prefer
  having `note_category.json` also carry the slug rather than re-implementing slugging in bash.
  Blog categories map to a curated category through `blog_categories`; a blog category not listed
  anywhere becomes `other`.
- Model `page_topics` from that file; `goatcounter_paths` LEFT JOINs it on `page`. New columns
  `topic` (slug) and `topic_label` (`"NN Name"`, zero-padded curated order, same convention as
  `section_label`; pages without a match get `"99 Other"`; non-brain/blog sections get their
  section name as topic so the chart still sums to the site).
- Metrics view `website_paths`: dimension `topic_label` ("Topic") between Section and Page.
- Dashboard `21_s_website_reach`: a new row after the daily-by-section chart, "What do readers
  come for": stacked bar of unique visitors per day by topic (last 7 days, `tr=P7D&grain=day`),
  and a table topic / visitors / pages / visitors per page for the same window. This answers the
  "today 632 views, but of what?" question without the page long tail.
- Same dashboard, KPI "pages per visitor": sum of per-page uniques divided by the site's daily
  uniques (`goatcounter_daily`), to tell one person reading ten pages from ten people reading
  one. This needs the site total joined into `goatcounter_paths` per day; a `site_visitors`
  column on that model is the simplest route.
- `scripts/page_topics.sh` runs wherever `scripts/blog_posts.sh` runs today (`make stats-fetch`
  / `make update`). Like `BLOG_DIR` there, it reads the sibling checkout
  `second-brain-public/`; `note_category.json` is gitignored and exists only after
  `make prepare` on the same machine.
- `goatcounter_paths` is a full-rebuild model, so the join re-labels history on every refresh,
  which is intended.

## Constraints

- Rust, inside `utils/obsidian-quartz`, following `note_status.rs` conventions (config in
  `config.toml` only for thresholds; the category list is data, so it lives in `data/`).
- Deterministic output: same inputs, byte-identical `note_category.json`.
- `make test` covers: link parsing (alias, heading, non-existent target), tie-break by curated
  order, hub fixed-ness, `other` fallback, the `#category/` override, and front-matter injection
  (fixture in `utils/obsidian-quartz/tests/hugo-render/`).
- Build time: the graph is small (714 nodes); propagation must stay well under a second.
- Hugo: enabling the `categories` taxonomy must not change the RSS feed or existing URLs.

## Decided by the author

- Categories are generated first, then curated by hand; the author has the final say on names,
  which exist, and their order of importance.
- No tags, no manual per-note work, no vault edits.
- Automated and explainable, in the style of the note-status levels.

## My calls (change if wrong)

- One category per note, `other` as fallback.
- Seeded label propagation from author-chosen hubs rather than raw community detection, for
  stability and naming.
- Curated list in `data/categories.yaml`, hubs referenced by note stem.
- Reader pages through Hugo's native `categories` taxonomy via injected front matter.
- Category shown in the note meta line before the status word; `other` shows nothing and is
  not a Hugo term.
- A provisional `categories.yaml` is generated and committed so the chain can be built and
  tested; the author edits it before the first deploy. Missing file = feature off, site unchanged.
- Home-page link lives in `layouts/index.html`, never in the vault-owned `_index.md`.
- Dashboard topic label uses the `"NN Name"` ordering convention already used for sections.
- Pages-per-visitor KPI included because it was asked in the same breath; it is independent
  of categories and can be built first.

## Left to the builder

- Community-detection algorithm for the proposal (label propagation with a fixed seed is enough;
  Louvain if a small dependency is acceptable).
- Exact markup and CSS of the category index; reuse the recent-notes list styles.
- Whether `score` in the JSON is the raw summed weight or normalised.
- How the dashboard script reads YAML (`yq`, or have the Rust command also emit
  `data/categories.json` so bash only needs `jq`).
