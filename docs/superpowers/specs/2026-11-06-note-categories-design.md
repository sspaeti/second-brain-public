# Note categories: generated from the link graph, curated once by the author

**Date:** 2026-11-06 (v2, rewritten after review and a throwaway graph spike)
**Status:** Approved by the author 2026-11-06, in implementation on branch `brain-categories-add`

## Problem

The public brain has 714 notes and no topic structure a reader or a chart can use. A reader who
likes one Obsidian note has no "other Obsidian notes" to go to; a newcomer has no map of what
is here. The stats dashboard (`my-personal-social-stats-dashboard`, GoatCounter data) stops at
the section split (`/brain` vs `/blog`); below that is a long tail of single pages that says
nothing about *what* people come for.

The author will not tag notes or maintain categories per note. Only 24 of 714 notes carry a tag,
and vault taxonomies have never stayed up to date. Everything must come out of `make prepare`.

## What already exists (and is reused)

- **The author's category list.** `content/_index.md` (a vault note) already has a hand-written
  topic grid with eight cards: Data Engineering, Personal Knowledge Management, Deep Life,
  Writing & Reading, Productivity, Neovim, Digital Garden, Programming. The blog's about page
  names the same topics. Each card links 1-12 "start here" notes. That grid *is* the curated
  category list and the hubs; the provisional `categories.yaml` below is generated from it.
- **MOC notes.** The vault has `Map of Content (MOC)`, `Data Engineering` (the "Data
  Engineering Vault", `hideTitle`, 92 outgoing links), `Data Engineering Concepts` (108 out,
  7 in), `coffee-beans-base` (37 out). High out-degree / low in-degree notes are maps, and the
  proposal command surfaces them as hub candidates.
- **The link graph.** 2,964 undirected wikilink edges between 714 notes, 17 isolated notes.
  Raw label propagation gives 90 communities (37 of size >= 5) whose hubs name obvious topics
  (duckdb/table formats, data modeling, orchestrators, coffee beans, obsidian, AI writing,
  digital garden, bluesky/open web, vibe coding, OLAP, semantic layer, vim/terminal).
- **Precedent.** Note status (`docs/superpowers/specs/2026-10-01-note-status-design.md`):
  computed from data the notes already produce, explainable per note, overridable, zero per-note
  work. `data/note_status.json` keyed by stem, `hugo.Data.note_status` in templates,
  `note_status_report.md` for the author, `#garden/<level>` override in `file_utils.rs`.

## Key insight

Seeded label propagation from the author's hubs assigns almost every note to a sensible
category in a handful of rounds. Spike on the real notes (pure stdlib, same rules as below):

| curated list | assigned | `other` | rounds | largest |
|---|---|---|---|---|
| the 8 home-page cards as-is | 655 | 59 (36 of them coffee) | 5 | Data Engineering 298 |
| 13 categories (cards + DE split + coffee + AI) | 693 | 21 | 5 | Data engineering 89 |

The 21 leftovers are real leftovers (recipes, macOS tips, `xattr`, `sign pdf on linux`). So:

1. **Generate** a proposal from the graph and the home-page grid (on demand, re-runnable).
2. **Curate** the list once by hand: which categories exist, names, order, hubs.
3. **Assign** every note on every build from the curated hubs, deterministically.

## Vocabulary

- **Category**: one of ~10-15 author-curated topics. `slug` for URLs and data
  (`data-lakes`), `name` for display (`Data lakes & table formats`).
- **Group** (optional): a coarser bucket a category belongs to, e.g. the five data-engineering
  categories under `Data engineering`. Display grouping and one dashboard column, nothing else.
- **Hub**: a note whose link neighbourhood defines a category. Hubs are ordinary notes and
  double as the category's "start here" list, in the order written.
- **Reason**: a per-note sentence explaining the assignment, like note-status `reason`.

## Goals

- Every public note gets exactly one category. Notes the graph gives no signal for land in
  `other`, which is a real category ("Other", last in order, its own page), so no note is
  without one (author decision 2026-11-06).
- The author curates the list once, then revisits only when the build report shows drift.
- A reader on a note sees which category it belongs to and can jump to the category.
- A newcomer gets one page, `/brain/categories/`, that works as the map of content: categories
  in curated order, each with a description, "start here" notes and a count.
- The stats dashboard gets a `topic` level between section and page.

## Non-goals

- No per-note tagging and no vault edits. `$secondbrain` stays untouched; only `content/`
  (generated, a git submodule with its own history) and this repo change.
- No LLM classification. Assignment is deterministic and explainable from links alone.
- No multi-category notes.
- Not a replacement for Hugo tags; the few existing tags stay.
- Not a replacement for the home-page topic grid in v1. The grid stays as it is (vault-owned);
  the category index links to it and vice versa. Folding counts/links into the grid is a
  later vault edit by the author.

## Anti-goals

- Categories that flip between builds for no reason. A note moves only when its links or the
  curated list change, and the report says which notes moved.
- A list that needs grooming every week.
- `other` becoming a big bucket. It is public, so it must stay the short tail (today: coffee
  beans and a few tips), and the report flags growth.
- A category-only change showing up as "updated today" on the note or in RSS.

## How it works

### Curated list: `data/categories.yaml` (committed, author-owned)

Ordered by importance. Order drives the index page, the dashboard sort, and tie-breaks.

```yaml
- slug: data-engineering
  name: Data engineering
  group: Data engineering                  # optional
  description: Concepts, lifecycle, the vault's entry points.
  hubs: [data engineering, data engineering concepts, data engineering whitepapers,
         data engineering lifecycle, data engineering toolkit, apache kafka,
         change data capture (cdc)]
  blog_categories: [data engineering]      # optional, dashboard only
- slug: pkm
  name: Personal knowledge management
  description: Second brain, Obsidian, Zettelkasten, note-taking.
  hubs: [personal knowledge management (pkm), second brain, obsidian, zettelkasten, ...]
  blog_categories: [productivity]
```

Rules:

- `hubs` are note filename stems as they appear in `content/` (lowercase, spaces kept), so the
  author can copy them from the proposal or from `ls content`. A hub that does not exist is a
  build warning. A hub listed under two categories belongs to the first (curated order) with a
  warning; the home-page grid lists `deep work`, `markdown`, `reading books for a happy life`
  and `public second brain with quartz` under two cards, so this must not be an error.
- `description` is one line, shown on the index page. `group` and `blog_categories` are
  optional. Unknown keys are ignored.
- File absent: `note-category` prints a warning, exits 0, writes nothing, injects nothing. The
  site is then exactly as today. Hugo templates guard on `hugo.Data.categories`.

### Provisional first version (generated by `--propose`, committed, then edited)

Fourteen categories, settled with the author on 2026-11-06: the home-page cards, data
engineering split in five, Personal knowledge management and Digital garden merged (same
thing), Productivity on its own, Deep life and Philosophy merged (they overlap), AI added,
Books added, and Food & coffee (the coffee notes come from an Obsidian Base, one note per
bean, plus the recipes). Counts are from the spike
and will shift slightly with the real implementation.

| slug | name | group | hubs (first = entry note) | notes |
|---|---|---|---|---|
| data-engineering | Data engineering | Data engineering | data engineering, data engineering concepts, data engineering whitepapers, data engineering lifecycle, data engineering toolkit, apache kafka, change data capture (cdc) | 86 |
| data-lakes | Data lakes & table formats | Data engineering | duckdb, data lake table format, data lakehouse, ducklake, delta lake, apache iceberg, datafusion, apache arrow, data lake, open data stack | 72 |
| data-modeling | Data modeling & warehousing | Data engineering | data modeling, data warehouse, dimensional modeling, data modeling languages, data modeling tools, relational model, data warehouse automation | 62 |
| orchestration | Orchestration & data stacks | Data engineering | data orchestrators, dagster, apache airflow, modern data stack, data contracts, declarative, declarative data stack, dag | 50 |
| analytics | Analytics, OLAP & semantic layer | Data engineering | semantic layer, metrics layer, cube, olap, modern olap systems, cloud data warehouses, rill developer, business intelligence, bi tools | 37 |
| programming | Programming | Data engineering | programming, python, rust, sql, functional data engineering, extending sql for analytics, great open-source tools in rust, docker, kubernetes, functional programming | 26 |
| second-brain | Second brain & digital garden | | second brain, obsidian, personal knowledge management (pkm), zettelkasten, digital garden, public second brain with quartz, managing my business with obsidian, permanent notes, how to build a second brain, smart note taking, map of content (moc), open-source obsidian publish alternatives, other public second brains, static site generators (ssg), future of blogging, bluesky, at protocol, why have your website, gohugo | 123 |
| writing | Writing & reading | | writing, reading, writing is thinking, markdown, creative writing, distract-free typewriter, markdown vs rich text, plain text files, ai writing, writing manually, will ai replace humans | 63 |
| ai | AI & coding with agents | | vibe coding, claude code, neomd, using obsidian with ai, generative ai and its addiction, having taste (with ai) | 25 |
| neovim | Neovim & terminal | | neovim, vim, neovim setup, vim language (and motions), query databases in vim (vim-dadbod), vim for obsidian, tuis, dotfiles, my terminal workflow with macos and linux, tmux, omarchy | 48 |
| productivity | Productivity | | productivity, getting things done (gtd), my gigs, no meetings (async), music that keeps you focused, creativity vs productivity, hell yeah or no, ultradian rhythm, todos, weekly todo lists, deep work | 33 |
| deep-life | Deep life & philosophy | | deep life, digital minimalism, what makes a happy life, thoughts on journal, minimalism, pathless path, dumb phone, journaling, slow technology, mini retirements, stoic, kintsugi, deep happiness, shallow happiness, slow living, default path, connecting the dots | 31 |
| books | Books | | reading books for a happy life, how to read books, books of data engineering, audiobooks, taking notes for physical books, the data engineering cookbook | 8 |
| food | Food & coffee | | coffee-beans-base, adrianos espresso, kafoj espresso, espresso dark roast, pintli coffee, home made ramen, ramen - soy sauce eggs, miso-glazed celeriac steaks with warm asian slaw, matcha | 42 |

Spike result for this list: 696 assigned, 18 `other`, 5 rounds.

### Command 1: `obsidian-quartz note-category --propose` (author-only, on demand)

Reads `content/*.md`, builds the undirected weighted wikilink graph: resolve `[[target]]`,
`[[target|alias]]`, `[[target#heading]]`, `[[target.md]]`, `![[embed]]` to a lowercase stem;
drop self-links, links to notes that do not exist, `_index`; weight = number of links between
the pair in either direction. Writes `note_category_proposal.md` at the repo root (gitignored):

- **Home-page grid**: the topic cards parsed from `content/_index.md` (`<div class="topic-card
  <slug>">`, `<h3>` name, hrefs mapped back to stems through `brain_slug`), as a ready-to-paste
  `categories.yaml` block. This is the starting point.
- **Communities**: deterministic label propagation (sorted node order, lowest label on ties, no
  randomness, 30 rounds). One block per community of size >= 5, ordered by size: size, top 5
  hubs by degree, 10 sample members, and, if `categories.yaml` exists, which curated category
  most members currently land in. This shows what the grid does not cover (coffee, AI, bluesky).
- **Map-like notes**: notes with out-degree >= 20 and out-degree >= 3x in-degree, as hub
  candidates (today: data engineering concepts, coffee-beans-base, data modeling languages,
  my terminal workflow with macos and linux).
- **Isolated notes** (no links either way).

The author pastes, renames, merges, orders, and commits `categories.yaml`. Expected once, and
again only when the build report shows drift.

### Command 2: `obsidian-quartz note-category` (every build)

Runs in `make prepare` **right after the `obsidian-quartz` export and before
`utils/revert-lastmod-only.sh`** (not after note-status as v1 said). The revert script diffs
`content/` against the submodule HEAD; once `categories:` is committed there, an injector that
runs after the script would make every note look changed on every build.

Seeded label propagation:

1. Label each hub with its category. Hubs are fixed and never relabelled. Duplicate hub → first
   category wins, warning. Missing hub → warning.
2. Repeat until no label changes (cap 50 rounds): every non-hub note, in sorted stem order,
   takes the label with the highest summed edge weight among its already-labelled neighbours.
   Updates are in place (a note relabelled earlier in the same round already votes with its new
   label; a note may change label again in a later round). Ties break by curated order.
   Unlabelled neighbours do not vote. Same inputs → byte-identical output. The spike numbers
   above come from exactly this procedure; a synchronous-round or label-once variant converges
   differently.
3. Still unlabelled after convergence → `other`: the built-in last category, name "Other",
   `order` = number of curated categories + 1, injected like any other slug, so
   `/brain/categories/other/` exists and every note has exactly one category. `other` may not
   be used as a slug in `categories.yaml`.
4. Override: a vault tag `#category/<slug>` wins over the computation, handled like
   `#garden/<level>` in `file_utils.rs` (a `split_category_tag` next to `split_garden_tag`;
   stripped from `tags`, written as `categories: [<slug>]` and `category_source: manual` by the
   export, in both the empty-front-matter branch and the merge branch, with `categories` added
   to the flow-style list arm next to `tags`/`aliases`). The build command reads
   `category_source: manual` and leaves such notes alone. The author is not expected to ever
   use this.

Outputs:

- `data/note_category.json` (gitignored like `note_status.json`), keyed by stem:
  ```json
  "duckdb": {"category": "data-lakes", "name": "Data lakes & table formats",
             "group": "Data engineering", "order": 2, "path": "/brain/duckdb/",
             "source": "hub" | "links" | "manual", "score": 7, "degree": 96,
             "reason": "Hub of Data lakes & table formats" |
                       "Links with duckdb, delta lake and 3 more notes in Data lakes & table formats"}
  ```
  `path` is `/brain/` + `brain_slug(stem)` **without a trailing slash**, because that is the
  `page` key GoatCounter paths are normalised to in the dashboard (`regexp_replace(path,
  '/+$', '')`), so the join needs no slugging and no further normalisation;
  `degree` is the note's total link weight, used to rank notes on the category page;
  `order` is the 1-based curated position. `score` is the raw summed weight (left to the
  builder in v1; raw is fine and explainable).
- `categories: [<slug>]` injected into the front matter of every note in `content/`
  (`other` included), as a **single-line insert/replace at the key's sorted position** (keys in the
  exported front matter are alphabetical, so the line goes after `aliases:` and before
  `createddate:`). The file is otherwise written back byte-for-byte. The export's own front
  matter writer is *not* re-run over `content/`: it serialises values built from the vault
  parse (`title` pre-quoted, `enableToc:` empty, a blank line after every non-string value),
  and re-parsing a content file through it would rewrite `title:`/`enableToc:` on all 714
  notes, which the revert script would then treat as real edits. Idempotence (second run is a
  no-op) and "only this one line differs" are both tested against a copy of the real
  `content/`. Notes with `category_source: manual` are left untouched.
- `note_category_report.md` at the repo root (gitignored): count per category and per group;
  every `other` note with its degree; notes whose winning score is 1; hubs not found or
  duplicated; **notes that moved since the previous run** (diff against the old
  `note_category.json` before overwriting it); and the 15 highest-degree notes in `other` or
  with score 1, as hub candidates. No community detection in the build; that stays in
  `--propose`, which the report tells the author to run when `other` grows past 5% of notes.

### Reader-facing

- **Note page** (`layouts/_default/single.html`): the category name joins the meta line as a
  link to the category page, right before the status word:
  `Last updated Oct 1, 2026 · Created Oct 1, 2026 · 1 min read · Data lakes & table formats · growing`.
  Read from `hugo.Data.note_category` by `.File.BaseFileName`, same as note status. `other`
  notes show "Other". Notes with `hideLastUpdate: true` (`data engineering`, `data engineering
  toolkit`, `cv`) still get the category item as a meta line of its own, because the Data
  Engineering Vault page is the entry note of the biggest group and should say where it sits;
  a `hideCategory: true` front-matter key turns it off per note.
- **Category index** `/brain/categories/` — the map of content. `layouts/categories/
  taxonomy.html` iterates `hugo.Data.categories` (so the curated order and descriptions win over
  Hugo's alphabetical taxonomy order), optionally grouped by `group`, each entry with name,
  description, note count from `.Site.Taxonomies.categories`, and the hubs as "Start with"
  links in curated order (first three, "and N more" opens the category page). Categories whose
  term has zero pages are skipped. "Other" is listed last with its count and no hubs. A one-line intro links to `Map of Content (MOC)` and the
  home-page grid.
- **Category page** `/brain/categories/<slug>/`: `layouts/categories/term.html`. Heading = name,
  description, then "Start here" (the hubs, curated order), then all other notes via
  `page-list.html` sorted by `degree` descending (most-linked first; a map, not a changelog),
  with the existing date + status word per row. No pagination; the biggest category is ~90 rows.
- **Home-page link**: in `layouts/index.html` next to the recent-notes partial (a "Browse by
  category" line), never in the vault-owned `_index.md`.
- **Related notes on the note page** (cheap, v1): under the backlinks, "More in <category>":
  up to 5 highest-degree notes of the same category, excluding the note itself. This is the
  "find the other Obsidian notes" moment the problem statement opens with. Skipped for `other`
  (no meaningful "more") and, like the backlinks, for `hideBacklinks: true` notes (those are curated landing pages
  with their own lists).
- Templates: new `layouts/categories/{taxonomy,term}.html` only. `layouts/_default/
  taxonomy.html` and `layouts/tags/term.html` are tags-specific and stay untouched. Note that
  `/brain/categories/` already exists today: with no terms it is rendered by
  `_default/taxonomy.html` as the tags listing titled "All posts" (`public/categories/
  index.html`), so the new template replaces that page rather than adding a URL. Hugo's
  `categories` taxonomy is on by default; term pages are kind `term`, so they are not in
  `.Site.RegularPages` and do not touch the home RSS feed, the recent-notes list or the
  graph/search indices (hugo-obsidian reads `content/` only). Each term page gets its own
  `/brain/categories/<slug>/index.xml` from `rss.xml` (`.Pages`), exactly as tag pages do;
  that is a feature (subscribe to one topic), not a regression.
- `utils/revert-lastmod-only.sh`: treat `categories` and `category_source` like `status` /
  `status_source` (a category-only diff keeps the working tree but restores HEAD's `lastmod`).
  `utils/recent_updates.py` already excludes front-matter lines from word counts, so a
  category-only commit produces no badge and no session. Add a case to
  `utils/test_revert_lastmod_only.py`.
- `.gitignore`: `data/note_category.json`, `note_category_report.md`,
  `note_category_proposal.md`.
- **Rollout and CDN.** `utils/purge_cdn.py` purges only notes whose `lastmod` moved since the
  last purge. The first build changes every note's HTML (meta line, "More in") without touching
  `lastmod`, and so does every later edit of `categories.yaml`. The first deploy is therefore
  `make deploy-clean` (full purge), and the report prints a reminder to run `make purge-cdn`
  whenever the moved-notes list is non-empty. After the first `make prepare` the author
  commits the content submodule once; until then the ~700 touched notes sit in the working
  tree, which `recent_updates.py` treats as uncommitted edits (harmless, local serve only).

### Stats dashboard (`my-personal-social-stats-dashboard`, Rill + DuckDB)

- New `scripts/page_topics.sh`, run next to `scripts/blog_posts.sh` in `make stats-fetch` /
  `make update`. Reads the sibling checkout `second-brain-public/data/note_category.json`
  (exists only after `make prepare` on the same machine; missing file → write an empty
  `data/page_topics.ndjson` and warn, dashboard shows everything as Other) and
  `data/categories.yaml` (through the JSON: every entry carries name/group/order, so bash needs
  only `jq`; the YAML is read only for `blog_categories`, via a `yq`-free `grep`/`awk` of the
  two keys, or the Rust command also writes `data/categories.json` — builder's choice, prefer
  the latter). Blog rows come from the blog's `categories:` front matter (`content/posts/*/
  index.en.md`, 114 posts): both the inline `categories: [a, b]` form and the block form
  (`categories:` followed by `  - a` lines, 15 posts) must be parsed; strip `#...` comments
  (48 posts carry `#only use one`, a few the typo `#only use on`) and whitespace, lowercase.
  Blog front matter today: `data engineering` (~73), `productivity` (~16), `reflections` (4),
  `data warehouse automation` (4), `show your workflow` (2), `writing` (2). A post with several
  categories takes the first. Mapped through `blog_categories`; page = the `url:` front matter
  with the trailing slash stripped (`/blog/<slug>`), not the folder name. Output
  `data/page_topics.ndjson`: `{page, topic_slug, topic_name, topic_group, topic_order}`.
  Mapping (settled with the author): `data engineering` → data-engineering,
  `data warehouse automation` → data-modeling, `productivity` → productivity,
  `reflections` → deep-life, `writing` → writing; `show your workflow` → neovim (my call).
- Model `page_topics`; `goatcounter_paths` LEFT JOINs it on `page` (full rebuild, so history
  is relabelled on every refresh, intended). New columns `topic`, `topic_label` (`"NN Name"`,
  zero-padded curated order, same convention as `section_label`) and `topic_group`. Unmatched
  `/brain/*` and `/blog/*` pages → `"99 Other"`; `/brain`, `/brain/categories*`, `/blog`, `/`
  → `"00 Navigation"` (the brain home is the top page and must not inflate Other); other
  sections keep their section name so the chart still sums to the site.
- Metrics view `website_paths`: dimensions `topic_label` ("Topic") and `topic_group` ("Topic
  group") between Section and Page.
- Dashboard `21_s_website_reach`: after the daily-by-section chart, a row "What do readers come
  for": stacked bar of visitors per day by `topic_label` (`tr=P7D&grain=day`) and a table
  topic / visitors / pages / visitors per page for the same window.
- KPI "pages per visitor" (independent of categories, can be built first): sum of per-page
  uniques over the site's daily uniques (`goatcounter_daily`), via a `site_visitors` column on
  `goatcounter_paths`.
- The dashboard repo is clean on `main` as of this review; the "uncommitted tenure buckets"
  note from the earlier session no longer applies.

## Constraints

- Rust, inside `utils/obsidian-quartz`, `note_status.rs` conventions. No new crate
  dependencies; `regex`, `serde_yaml`, `serde_json` are already there. No thresholds in
  `config.toml` for v1 (the list is data; `other` has no threshold).
- Deterministic: same `content/` + `categories.yaml` → byte-identical JSON and front matter.
- Build time: 714 nodes, well under a second.
- `make test` covers: link parsing (alias, heading, `.md` suffix, embed, missing target,
  self-link), tie-break by curated order, hubs stay fixed, duplicate hub → first wins,
  `other` fallback (injected, last order), the `#category/` override, front-matter injection idempotence (running twice
  changes nothing), the revert script ignoring `categories:`, and a Hugo render fixture in
  `utils/obsidian-quartz/tests/hugo-render/` (fixture `data/categories.yaml` **and**
  `note_category.json`, both required: the fixture `data/` mount sits before the real one, so
  a missing fixture file lets the real gitignored `note_category.json` leak in; asserts the
  meta-line link, the index order, the term page and that the home RSS feed has no category
  items).
- Hugo: no change to existing note URLs or the home RSS feed; `/brain/categories/` changes
  from an empty tags listing to the index. `disablePathToLower = true` is set, so slugs in
  `categories.yaml` must already be lowercase; the command rejects others.

## Decided by the author

- Generated first, curated by hand; the author has the final say on names, which exist, order.
- No tags, no per-note work, no vault edits. Everything in `make prepare`.
- Automated and explainable, in the style of note-status.
- Three readers: the person on a note (which category am I in), the newcomer (a map of content
  to start from), and the author's analytics (which topics are read).

## My calls (change if wrong)

- The home-page topic grid is the source of the curated list, not the raw clusters; the raw
  clusters only add what the grid misses (coffee, AI, bluesky/open web) and split data
  engineering so the analytics are useful. Fourteen categories in the provisional file.
- Optional `group` key so the five data-engineering categories still roll up to one number.
- Hubs are the "start here" notes; the first hub is the entry note. No separate `start:` key.
- Category page sorted by link degree, not by last updated; hubs first.
- "More in <category>" block on each note page is in v1; it is five template lines once the
  JSON carries `degree`.
- `note-category` runs before `revert-lastmod-only.sh`; the script learns two more line names.
- Report diffs against the previous run and skips community detection; `--propose` keeps it.
- JSON carries `path`, `order`, `group`, `degree` so neither the dashboard nor the templates
  re-derive anything.
- `"00 Navigation"` topic label for home/index pages in the dashboard.
- Hub lists per category are mine (copied from the home-page cards plus the strongest cluster
  hubs); the author edits YAML.

- Front-matter injection is a sorted-position line insert, never a re-serialisation of
  `content/` (see Outputs); the export writes `categories:` only for the manual override.
- Category item shows even on `hideLastUpdate` notes; "More in" follows `hideBacklinks`.
- `path` in the JSON has no trailing slash, matching the dashboard's `page` key.
- First deploy is `make deploy-clean`; `categories.yaml` edits need a CDN purge.
- Provisional `blog_categories` mapping as listed in the dashboard section.

## Left to the builder

- Exact markup/CSS of the index and term pages; reuse `page-list.html` and the home-page
  `topic-grid` look (`assets/styles/`), nothing new in the bundle if avoidable.
- `--propose` community detection: deterministic synchronous label propagation is enough;
  no Louvain, no dependency.
- Whether `categories.json` is emitted for the dashboard or the bash script greps the YAML.

## Changes from v1 (2026-11-06 review)

- Curated list seeded from the existing home-page topic grid and the blog about page, not from
  cluster hub names; provisional file has 13 categories with real counts from a spike.
- Makefile order fixed: inject before `revert-lastmod-only.sh`, not after `note-status`.
- Duplicate hubs are a warning (first wins), not an error; the grid has four of them.
- Report: moved-notes diff and hub candidates instead of a community-detection run per build.
- Index page shows curated hubs as "Start with", category page sorts by degree; "More in
  <category>" block added to the note page.
- JSON gains `path`, `order`, `group`, `degree`; dashboard gains `topic_group` and a
  Navigation bucket; blog category parsing must strip the `#only use one` comments.
- Dashboard-repo "uncommitted changes" note dropped; the repo is clean.
- After the builder check: injection pinned to a line insert (re-serialising would rewrite
  `title:`/`enableToc:` on every note), in-place propagation semantics spelled out, `path`
  without trailing slash, block-style blog category lists, the existing empty
  `/brain/categories/` page, CDN purge on rollout, both fixture data files, and category
  display on `hideLastUpdate` notes.
