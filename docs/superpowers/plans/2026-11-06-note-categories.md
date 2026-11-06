# Note Categories Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every brain note gets one category from the wikilink graph and an author-curated hub list; readers get a category link on each note and a map-of-content index; the stats dashboard gets a topic level.

**Architecture:** A new `obsidian-quartz note-category` Rust subcommand reads `content/*.md` + `data/categories.yaml`, runs seeded label propagation, writes `data/note_category.json`, injects one `categories: [slug]` line per note, and writes an author report (`--propose` writes a proposal instead). Hugo renders the category pages through its native `categories` taxonomy plus two new layouts; the note page reads the JSON like `note_status`. The dashboard repo gets a jq-only script that joins the JSON onto GoatCounter paths.

**Tech Stack:** Rust (regex, serde_yaml, serde_json, already in Cargo.toml), Hugo 0.168 templates, bash + jq, Rill/DuckDB YAML.

**Spec:** `docs/superpowers/specs/2026-11-06-note-categories-design.md`

## Global Constraints

- No new crate dependencies. Conventions of `utils/obsidian-quartz/src/note_status.rs` (consts for paths, `load_json`/`save_json_pretty` from `enrich_with_blog`, report at repo root never under `data/`).
- Deterministic: same `content/` + `categories.yaml` → byte-identical JSON and front matter. Node iteration in sorted stem order; ties by curated order.
- `note-category` runs after the export and **before** `utils/revert-lastmod-only.sh` in `make prepare`.
- Injection is a single-line insert/replace at the sorted key position. Never re-serialise `content/` front matter.
- `other` is a real last category ("Other", order = N+1), injected like any slug. `other` is rejected as a slug in `categories.yaml`. Slugs must be lowercase `[a-z0-9-]+`.
- JSON `path` = `/brain/<brain_slug(stem)>` with **no** trailing slash.
- The vault (`$secondbrain`) is never written. Only `content/`, `data/`, `layouts/`, `assets/`, `utils/`, `Makefile`, `.gitignore`, and the dashboard repo change.
- Templates: `layouts/_default/taxonomy.html` and `layouts/tags/term.html` stay untouched.
- Work on branch `brain-categories-add`; commit per task; nothing lands on `hugo` without the author's review.

## Review Focus

1. A note whose only front-matter change is the `categories:` line must not count as updated: `revert-lastmod-only.sh` restores `lastmod` (Task 7 test).
2. Running `note-category` twice in a row must leave `content/` byte-identical (Task 4 idempotence test on a copy of three real notes).
3. A wikilink to a note that does not exist, to `_index`, or to itself must not create an edge (Task 1 tests).
4. `categories.yaml` missing → exit 0, no files written, no injection, Hugo renders the site as today (Task 2/9 tests: missing-file path + template guards).
5. A hub listed twice or missing must warn, not abort the build (Task 3 test).

---

### Task 1: Link graph from `content/`

**Files:**
- Create: `utils/obsidian-quartz/src/note_category.rs` (module `note_category`, registered in `main.rs` in Task 7)
- Test: unit tests in the same file (`#[cfg(test)] mod tests`)

**Interfaces:**
- Produces:
  ```rust
  pub struct Graph { pub stems: Vec<String> /* sorted */, pub adj: BTreeMap<String, BTreeMap<String, u32>> /* undirected weights */ }
  pub fn parse_links(body: &str) -> Vec<String>          // raw targets, lowercased stem (alias/heading/.md/embed stripped, path prefix dropped)
  pub fn build_graph(notes: &BTreeMap<String, String>) -> Graph   // stem -> file text (front matter included)
  impl Graph { pub fn degree(&self, stem: &str) -> u32 }
  ```
- Loader: `fn read_content(dir: &Path) -> Result<BTreeMap<String, String>>` top-level `.md` only, skips `_index`, stem = filename without `.md` (already lowercase on disk).

- [ ] **Step 1: Write failing tests**

```rust
#[test] fn parse_links_strips_alias_heading_embed_and_path() {
    let got = parse_links("[[DuckDB|the duck]] ![[Delta Lake#History]] [[a/b/Apache Arrow.md]] [[x]]");
    assert_eq!(got, vec!["duckdb", "delta lake", "apache arrow", "x"]);
}
#[test] fn build_graph_drops_missing_self_and_index_counts_both_directions() {
    let mut n = BTreeMap::new();
    n.insert("a".into(), "[[b]] [[b]] [[a]] [[zzz]] [[_index]]".into());
    n.insert("b".into(), "[[a]]".into());
    n.insert("c".into(), "".into());
    let g = build_graph(&n);
    assert_eq!(g.adj["a"]["b"], 3); assert_eq!(g.adj["b"]["a"], 3);
    assert!(g.adj.get("c").map_or(true, |m| m.is_empty()));
    assert_eq!(g.degree("a"), 3); assert_eq!(g.stems, vec!["a", "b", "c"]);
}
```

- [ ] **Step 2: Run `cd utils/obsidian-quartz && cargo test note_category` → FAIL (module missing)**
- [ ] **Step 3: Implement `parse_links`, `build_graph`, `read_content`** with one regex `!?\[\[([^\]|#]+)(?:#[^\]|]*)?(?:\|[^\]]*)?\]\]`; take the last `/` segment, strip `.md`, trim, lowercase.
- [ ] **Step 4: `cargo test note_category` → PASS**
- [ ] **Step 5: Commit** `feat(note-category): wikilink graph from content/`

### Task 2: Curated list loader

**Files:** Modify `note_category.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct Category { pub slug: String, pub name: String, pub group: Option<String>, pub description: Option<String>, pub hubs: Vec<String>, pub blog_categories: Vec<String> }
  pub const OTHER_SLUG: &str = "other"; pub const OTHER_NAME: &str = "Other";
  pub fn parse_categories(yaml: &str) -> Result<Vec<Category>, String>   // errors: bad slug, duplicate slug, `other` used, empty hubs
  const CATEGORIES_YAML: &str = "data/categories.yaml";
  ```
- Consumes: nothing.

- [ ] **Step 1: Failing tests**: valid YAML round-trips (optional keys default), `slug: Data-Eng` → `Err` containing "lowercase", `slug: other` → `Err`, duplicate slug → `Err`, unknown key ignored.
- [ ] **Step 2: Run → FAIL**
- [ ] **Step 3: Implement with `serde_yaml` + `#[serde(default)]` and a validation pass.**
- [ ] **Step 4: Run → PASS. Commit** `feat(note-category): parse data/categories.yaml`

### Task 3: Seeded label propagation

**Files:** Modify `note_category.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct Assignment { pub stem: String, pub slug: String, pub source: Source /* Hub | Links | Manual | Other */, pub score: u32, pub reason: String }
  pub fn assign(graph: &Graph, cats: &[Category], manual: &BTreeMap<String, String>) -> (Vec<Assignment>, Vec<String> /* warnings */)
  ```
- Semantics (spec "Command 2"): hubs fixed; duplicate hub → first category + warning `hub 'x' listed under 'a' and 'b'; using 'a'`; missing hub → warning; in-place updates, sorted order, max 50 rounds, tie → lower curated index; unlabelled → `other`, score 0; `manual` entries (stem → slug) are fixed like hubs with `Source::Manual` (unknown slug → warning, treated as computed).
- Reason strings: hub → `Hub of <Name>`; links → `Links with <top>, <second> and N more notes in <Name>` (neighbours in that category sorted by weight desc then stem; "and N more" only if N>0; two names max); manual → `Set by author (#category/<slug>)`; other → `No links to a categorised note`.

- [ ] **Step 1: Failing tests** on a hand-built graph: (a) hub stays put even if all its neighbours are another category; (b) tie 1:1 between cat index 0 and 1 → index 0; (c) chain a(hub A)-b-c-d: all get A, scores 1; (d) isolated → other/score 0; (e) duplicate hub warning text; (f) manual override wins and reason text; (g) determinism: run twice, equal.
- [ ] **Step 2: Run → FAIL. Step 3: Implement. Step 4: Run → PASS. Commit** `feat(note-category): seeded label propagation`

### Task 4: JSON output and front-matter injection

**Files:** Modify `note_category.rs`

**Interfaces:**
- Produces:
  ```rust
  pub fn to_json(assigns: &[Assignment], cats: &[Category], graph: &Graph) -> serde_json::Value   // keyed by stem: category,name,group,order,path,source,score,degree,reason
  pub fn inject_category_line(text: &str, slug: &str) -> Option<String>   // None when unchanged
  pub fn manual_from_frontmatter(text: &str) -> Option<String>            // slug when `category_source: manual` and `categories: [x]`
  const OUT_JSON: &str = "data/note_category.json";
  ```
- `path` = `format!("/brain/{}", crate::slug::brain_slug(stem))`; `order` = 1-based curated index, `other` = cats.len()+1; `group`/`name` for other = null/"Other".
- Injection: front matter is lines between the first `---` and the next `---`. Replace an existing `categories:` line in place; otherwise insert before the first key that sorts after `categories` (byte order, as the export does), or before the closing `---` when none. Everything else byte-identical.

- [ ] **Step 1: Failing tests**: insert between `aliases:` and `createddate:`; insert as last key; replace existing with different slug; identical slug → `None`; no front matter → `None`; `to_json` entry for a hub has `order`, `path` without trailing slash (`"don't stop"` → `/brain/dont-stop`), `degree`.
- [ ] **Step 2: Run → FAIL. Step 3: Implement. Step 4: Run → PASS. Commit** `feat(note-category): json + idempotent front-matter line`

### Task 5: Report and `run()`

**Files:** Modify `note_category.rs`

**Interfaces:**
- Produces: `pub fn run(args: &[String]) -> Result<(), Box<dyn Error>>` (`args` after the subcommand; `--propose` handled in Task 6), `fn render_report(prev: Option<&Value>, assigns, cats, graph, warnings) -> String`, `const OUT_REPORT: &str = "note_category_report.md"`.
- `run()` without `--propose`: if `CATEGORIES_YAML` missing → `eprintln!("note-category: data/categories.yaml not found; categories disabled")`, return Ok. Else: read content, graph, manual map from front matter, assign, load previous JSON (optional), write JSON, inject into every note (count changed files), write report, print one summary line `note-category: 714 notes -> 14 categories, 18 other, 696 lines changed`.
- Report sections: counts per category and group; `## other` list with degree; `## weak (score 1)`; `## warnings`; `## moved since last run` (`stem: old -> new`); `## hub candidates` (15 highest-degree notes in other or score 1); a reminder line about `make purge-cdn` when moved is non-empty and about `--propose` when other > 5%.

- [ ] **Step 1: Failing test** for `render_report` with a previous JSON that differs for one stem → contains `- b: a -> other` under "moved"; empty previous → no moved section.
- [ ] **Step 2–4: implement, PASS. Commit** `feat(note-category): run + author report`

### Task 6: `--propose`

**Files:** Modify `note_category.rs`

**Interfaces:**
- Produces: `fn propose(graph, notes, cats: Option<&[Category]>) -> String`, `const OUT_PROPOSAL: &str = "note_category_proposal.md"`, `pub fn parse_home_grid(index_md: &str, stems: &BTreeSet<String>) -> Vec<(String /*slug*/, String /*name*/, Vec<String> /*hub stems*/)>`, `pub fn communities(graph: &Graph) -> Vec<Vec<String>>` (deterministic synchronous LPA: init label=stem, 30 rounds, sorted nodes, lowest label on ties; return groups size desc).
- Home grid: `<div class="topic-card (slug)">` … `</div>`; name = text of `<h3>` with tags stripped; hrefs mapped to stems by matching `brain_slug(stem)`; duplicates across cards dropped (first card wins).
- Output order: grid as YAML block, communities ≥5 (size, top-5 by degree, 10 samples, dominant curated category if `cats`), map-like notes (out ≥ 20 and out ≥ 3×in: needs directed counts, so keep `out_deg`/`in_deg` maps on `Graph`), isolated notes.

- [ ] **Step 1: Failing tests**: `parse_home_grid` on a two-card snippet with a duplicate href; `communities` on two triangles joined by nothing → two groups of 3.
- [ ] **Step 2–4: implement, PASS. Commit** `feat(note-category): --propose`

### Task 7: Wire into the build

**Files:**
- Modify: `utils/obsidian-quartz/src/main.rs` (dispatch `note-category` → `note_category::run(&args[2..])`), `Makefile` (line after `obsidian-quartz #copy all notes`), `.gitignore`, `utils/revert-lastmod-only.sh`, `utils/test_revert_lastmod_only.py`
- Makefile line: `obsidian-quartz note-category #one category per note from the link graph + data/categories.yaml -> data/note_category.json, categories: front matter (+ note_category_report.md for the author)`
- Revert script: regex `^[+-](lastmod|status|status_source|categories|category_source):` in both greps.

- [ ] **Step 1: Failing Python test** `test_categories_line_is_kept_but_lastmod_is_restored` (category line added + lastmod bumped → lastmod restored, `categories: [neovim]` kept) and `test_category_change_is_kept_but_lastmod_is_restored` (committed `[a]` → working `[b]`).
- [ ] **Step 2: `python -m unittest utils/test_revert_lastmod_only.py` → FAIL. Step 3: edit script. Step 4: PASS.**
- [ ] **Step 5: main.rs + Makefile + .gitignore; `cargo build --release` and `make -C utils/obsidian-quartz` to install. Commit** `build: run note-category in make prepare before revert-lastmod-only`

### Task 8: `#category/<slug>` vault override in the export

**Files:** Modify `utils/obsidian-quartz/src/file_utils.rs` (tag handling ~L334-355, branch A ~L567-581, branch B ~L720-733, flow-style arm ~L807), `note_status.rs` (add `split_category_tag` next to `split_garden_tag`)

**Interfaces:** `pub fn split_category_tag(tags: Vec<String>) -> (Vec<String>, Option<String>)` (first `category/<slug>` wins, lowercase, stripped from tags). Export writes `categories: [<slug>]` + `category_source: manual` when present; removes both keys first in branch B (like status). Flow-style arm: `key == "tags" || key == "aliases" || key == "categories"`.

- [ ] **Step 1: Failing unit test** for `split_category_tag(["publish","category/Neovim","x"])` → `(["publish","x"], Some("neovim"))`.
- [ ] **Step 2–4: implement, PASS (`cargo test`). Commit** `feat(export): #category/<slug> vault override`

### Task 9: Hugo templates, styles, fixtures

**Files:**
- Modify: `layouts/_default/single.html` (meta item before status; "More in" block inside the backlinks container after `backlinks.html`), `layouts/index.html` (a `<p class="browse-categories">` link `/brain/categories/` before `recent.html`)
- Create: `layouts/categories/taxonomy.html`, `layouts/categories/term.html`, `assets/styles/categories.scss` (auto-bundled: `styles/[!_]*.scss`)
- Fixtures: `utils/obsidian-quartz/tests/hugo-render/data/categories.yaml`, `.../data/note_category.json`; add `categories: [data-lakes]` to `status note.md`, `categories: [other]` to `manual note.md`; test in `tests/hugo_render.rs`
- Template rules (spec "Reader-facing"): meta item `<a class="note-category" href="{{ "categories/<slug>/" | relURL }}">Name</a>` rendered outside the `hideLastUpdate` guard when that is set (own line), unless `.Params.hideCategory`; index iterates `hugo.Data.categories` (then Other), count from `.Site.Taxonomies.categories` (`index … slug`), skip zero; hubs → "Start with" links via `.Site.GetPage (printf "/%s" (brain_slug))` — stems are not slugs, so the JSON is the lookup: `hugo.Data.note_category` entry `.path`; term page: hubs first, then remaining pages sorted by `degree` desc via `sort` on a built slice; "More in": top 5 by degree in same slug, excluding self, skipped for `other`/`hideBacklinks`.

- [ ] **Step 1: Failing render test** `note_category_link_index_and_term_page`: `status-note` meta contains `href="/brain/categories/data-lakes/"` before `nc-label`; `/categories/index.html` lists "Data lakes" before "Other" and no "All posts"; `/categories/data-lakes/index.html` contains "Status Note"; home `index.xml` has no `/categories/` item; `manual-note` shows "Other" and no "More in".
- [ ] **Step 2: `cargo test --test hugo_render` → FAIL. Step 3: templates + scss. Step 4: PASS. Commit** `feat(hugo): category meta link, index, term pages`

### Task 10: Provisional `data/categories.yaml` and first run

**Files:** Create `data/categories.yaml` (the 14 rows of the spec table with `name`, `group`, one-line `description`, `hubs`, `blog_categories`).

- [ ] **Step 1: `obsidian-quartz note-category --propose`** → read `note_category_proposal.md`, confirm the grid block parses and all spec hubs exist.
- [ ] **Step 2: Write `data/categories.yaml`; run `obsidian-quartz note-category`**; expected summary ≈ `714 notes -> 14 categories, ~18 other`; check report: no missing hubs.
- [ ] **Step 3: Run it again → `0 lines changed`. `git -C content diff --stat | tail -1` ≈ 714 files, one insertion each.**
- [ ] **Step 4: `bash utils/revert-lastmod-only.sh`** → no "reverted" lines for notes that only gained a category (lastmod unchanged anyway).
- [ ] **Step 5: `hugo --quiet -d /tmp/.../brain-cat`**; open `/categories/index.html`, one term page, one note. **Step 6: `make test`. Commit** `data: provisional categories.yaml` (content submodule is NOT committed by me; the author reviews it).

### Task 11: Dashboard (sibling repo, branch `brain-topics`)

**Files:** `my-personal-social-stats-dashboard/scripts/page_topics.sh`, `models/page_topics.yaml`, `models/goatcounter_paths.yaml`, `metrics/website_paths.yaml`, `dashboards/21_s_website_reach.yaml`, `Makefile` (`stats-fetch`), README line.

- Script: `BRAIN_DIR` (default `$HOME/git/sspaeti.com/second-brain-public`), `BLOG_DIR` as in `blog_posts.sh`. jq over `note_category.json` → `{page: .path, topic_slug: .category, topic_name: .name, topic_group: .group, topic_order: .order}`. Blog: awk over front matter for `categories:` inline or block, strip `#…`, trim, lowercase, first wins; map via a small inline table built with `grep -A` from `categories.yaml` (`slug:` + `blog_categories:` lines); unmapped → `other`; page = `url:` without trailing slash. Missing JSON → empty file + warning.
- Model/metrics/dashboard per spec; labels `printf('%02d %s', topic_order, topic_name)`, `99 Other`, `00 Navigation` for `/`, `/brain`, `/brain/categories%`, `/blog`.
- [ ] **Step 1: run script, `head data/page_topics.ndjson`, `jq -s length`** ≈ 714 + 114. **Step 2: `rill project refresh --local --model goatcounter_paths --full`** if Rill running; else `duckdb` ad-hoc check of the join hit rate (> 90 % of `/brain/%` visitors matched). **Step 3: commit on `brain-topics`.**
