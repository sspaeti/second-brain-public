//! Note categories: one category per note from the wikilink graph and the
//! author-curated hub list in `data/categories.yaml`.
//! Spec: docs/superpowers/specs/2026-11-06-note-categories-design.md

use std::collections::{BTreeMap, BTreeSet};

use std::error::Error;
use std::fs;
use std::path::Path;

use regex::Regex;

const CONTENT_DIR: &str = "content";

const CATEGORIES_YAML: &str = "data/categories.yaml";
pub const OTHER_SLUG: &str = "other";
pub const OTHER_NAME: &str = "Other";

/// One curated category from `data/categories.yaml`, in file order.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Category {
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub hubs: Vec<String>,
    #[serde(default)]
    pub blog_categories: Vec<String>,
}


/// `#category/<slug>` tags naming a slug that is not in `categories.yaml` become
/// implicit categories appended after the curated ones (sorted; name = the slug
/// in Title Case, no hubs, no description), so the override always wins and the
/// category page exists. The report asks the author to add them to the YAML.
pub fn with_manual_categories(mut cats: Vec<Category>, manual: &BTreeMap<String, String>) -> (Vec<Category>, Vec<String>) {
    let known: BTreeSet<String> = cats.iter().map(|c| c.slug.clone()).collect();
    let mut added: Vec<String> = manual.values().filter(|s| !known.contains(*s) && s.as_str() != OTHER_SLUG).cloned().collect();
    added.sort();
    added.dedup();
    for slug in &added {
        let name = slug
            .split('-')
            .filter(|w| !w.is_empty())
            .map(|w| {
                let mut c = w.chars();
                match c.next() {
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        cats.push(Category { slug: slug.clone(), name, group: None, description: None, hubs: vec![], blog_categories: vec![] });
    }
    (cats, added)
}

/// Parse and validate the curated list. Slugs must be lowercase
/// `[a-z0-9-]+`, unique, never `other`; every category needs at least one hub.
pub fn parse_categories(yaml: &str) -> Result<Vec<Category>, String> {
    let cats: Vec<Category> = serde_yaml::from_str(yaml).map_err(|e| format!("categories.yaml: {e}"))?;
    let mut seen = BTreeSet::new();
    for c in &cats {
        if c.slug.is_empty() || !c.slug.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-') {
            return Err(format!("categories.yaml: slug '{}' must be lowercase [a-z0-9-]", c.slug));
        }
        if c.slug == OTHER_SLUG {
            return Err(format!("categories.yaml: '{}' is the built-in fallback and cannot be curated", OTHER_SLUG));
        }
        if !seen.insert(c.slug.clone()) {
            return Err(format!("categories.yaml: duplicate slug '{}'", c.slug));
        }
        if c.hubs.is_empty() {
            return Err(format!("categories.yaml: category '{}' has no hubs", c.slug));
        }
    }
    Ok(cats)
}

/// Undirected weighted wikilink graph over the published notes.
#[derive(Debug, Default)]
pub struct Graph {
    /// Every note stem, sorted.
    pub stems: Vec<String>,
    /// `adj[a][b]` = number of links between `a` and `b` in either direction.
    pub adj: BTreeMap<String, BTreeMap<String, u32>>,
    /// Outgoing links to existing other notes (duplicates counted), for the proposal.
    pub out_deg: BTreeMap<String, u32>,
    /// Incoming links from other notes (duplicates counted), for the proposal.
    pub in_deg: BTreeMap<String, u32>,
}

impl Graph {
    /// Total link weight of a note.
    pub fn degree(&self, stem: &str) -> u32 {
        self.adj.get(stem).map_or(0, |m| m.values().sum())
    }

    pub fn neighbours(&self, stem: &str) -> Option<&BTreeMap<String, u32>> {
        self.adj.get(stem)
    }
}

fn link_re() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"!?\[\[([^\]|#]+)(?:#[^\]|]*)?(?:\|[^\]]*)?\]\]").unwrap())
}

/// Wikilink targets of a note body as lowercase stems: alias, heading, `.md`
/// suffix, embed `!` and any folder prefix are dropped. Order and duplicates
/// are kept (the caller counts them as weight).
pub fn parse_links(body: &str) -> Vec<String> {
    link_re()
        .captures_iter(body)
        .filter_map(|c| {
            let raw = c.get(1)?.as_str();
            let last = raw.rsplit('/').next().unwrap_or(raw).trim();
            let last = last.strip_suffix(".md").unwrap_or(last).trim();
            if last.is_empty() {
                None
            } else {
                Some(last.to_lowercase())
            }
        })
        .collect()
}

/// Build the graph from `stem -> file text`. Links to missing notes, to the
/// note itself and to `_index` are ignored.
pub fn build_graph(notes: &BTreeMap<String, String>) -> Graph {
    let mut g = Graph::default();
    g.stems = notes.keys().filter(|s| s.as_str() != "_index").cloned().collect();
    let exists: BTreeSet<&String> = g.stems.iter().collect();
    for (stem, text) in notes {
        if stem == "_index" {
            continue;
        }
        g.adj.entry(stem.clone()).or_default();
        for target in parse_links(text) {
            if target == *stem || target == "_index" || !exists.contains(&target) {
                continue;
            }
            *g.adj.entry(stem.clone()).or_default().entry(target.clone()).or_insert(0) += 1;
            *g.adj.entry(target.clone()).or_default().entry(stem.clone()).or_insert(0) += 1;
            *g.out_deg.entry(stem.clone()).or_insert(0) += 1;
            *g.in_deg.entry(target).or_insert(0) += 1;
        }
    }
    g
}


const MAX_ROUNDS: usize = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Hub,
    Links,
    Manual,
    Other,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Hub => "hub",
            Source::Links => "links",
            Source::Manual => "manual",
            Source::Other => "other",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assignment {
    pub stem: String,
    pub slug: String,
    pub source: Source,
    /// Summed edge weight into the winning category (0 for hubs/manual/other).
    pub score: u32,
    pub reason: String,
}

/// Seeded label propagation. Hubs (and `#category/` manual notes) are fixed;
/// every other note, in sorted order and in place, takes the category with
/// the highest summed edge weight among its already-labelled neighbours,
/// ties broken by curated order, until nothing changes (cap `MAX_ROUNDS`).
/// Notes left without a label become `other`.
pub fn assign(graph: &Graph, cats: &[Category], manual: &BTreeMap<String, String>) -> (Vec<Assignment>, Vec<String>) {
    let mut warnings = Vec::new();
    let rank: BTreeMap<&str, usize> = cats.iter().enumerate().map(|(i, c)| (c.slug.as_str(), i)).collect();
    let name_of = |slug: &str| -> String {
        cats.iter().find(|c| c.slug == slug).map(|c| c.name.clone()).unwrap_or_else(|| OTHER_NAME.to_string())
    };
    let exists: BTreeSet<&str> = graph.stems.iter().map(String::as_str).collect();

    // label: stem -> curated index; fixed: hub or manual
    let mut label: BTreeMap<&str, usize> = BTreeMap::new();
    let mut fixed: BTreeMap<&str, Source> = BTreeMap::new();
    for (i, c) in cats.iter().enumerate() {
        for h in &c.hubs {
            let h = h.as_str();
            if !exists.contains(h) {
                warnings.push(format!("hub '{}' of '{}' not found in content/", h, c.slug));
                continue;
            }
            if let Some(&prev) = label.get(h) {
                if prev != i {
                    warnings.push(format!("hub '{}' listed under '{}' and '{}'; using '{}'", h, cats[prev].slug, c.slug, cats[prev].slug));
                }
                continue;
            }
            label.insert(h, i);
            fixed.insert(h, Source::Hub);
        }
    }
    for (stem, slug) in manual {
        if !exists.contains(stem.as_str()) {
            continue;
        }
        match rank.get(slug.as_str()) {
            Some(&i) => {
                label.insert(stem, i);
                fixed.insert(stem, Source::Manual);
            }
            None => warnings.push(format!("note '{}' has #category/{} but no such category; computing instead", stem, slug)),
        }
    }

    for _ in 0..MAX_ROUNDS {
        let mut changed = 0;
        for stem in &graph.stems {
            if fixed.contains_key(stem.as_str()) {
                continue;
            }
            let Some(nb) = graph.neighbours(stem) else { continue };
            let mut votes: BTreeMap<usize, u32> = BTreeMap::new();
            for (m, w) in nb {
                if let Some(&l) = label.get(m.as_str()) {
                    *votes.entry(l).or_insert(0) += w;
                }
            }
            // highest weight, then lowest curated index (BTreeMap iterates ascending)
            let best = votes.iter().fold(None::<(usize, u32)>, |acc, (&l, &w)| match acc {
                Some((_, bw)) if bw >= w => acc,
                _ => Some((l, w)),
            });
            if let Some((l, _)) = best {
                if label.get(stem.as_str()) != Some(&l) {
                    label.insert(stem, l);
                    changed += 1;
                }
            }
        }
        if changed == 0 {
            break;
        }
    }

    let mut out = Vec::with_capacity(graph.stems.len());
    for stem in &graph.stems {
        let entry = match (label.get(stem.as_str()), fixed.get(stem.as_str())) {
            (Some(&l), Some(&Source::Hub)) => Assignment {
                stem: stem.clone(), slug: cats[l].slug.clone(), source: Source::Hub, score: 0,
                reason: format!("Hub of {}", cats[l].name),
            },
            (Some(&l), Some(&Source::Manual)) => Assignment {
                stem: stem.clone(), slug: cats[l].slug.clone(), source: Source::Manual, score: 0,
                reason: format!("Set by author (#category/{})", cats[l].slug),
            },
            (Some(&l), _) => {
                let mut same: Vec<(&String, u32)> = graph
                    .neighbours(stem)
                    .map(|nb| nb.iter().filter(|(m, _)| label.get(m.as_str()) == Some(&l)).map(|(m, &w)| (m, w)).collect())
                    .unwrap_or_default();
                same.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
                let score: u32 = same.iter().map(|(_, w)| w).sum();
                let names: Vec<&str> = same.iter().take(2).map(|(m, _)| m.as_str()).collect();
                let more = same.len().saturating_sub(2);
                let who = match (names.as_slice(), more) {
                    ([a], _) => a.to_string(),
                    ([a, b], 0) => format!("{a} and {b}"),
                    ([a, b], n) => format!("{a}, {b} and {n} more notes"),
                    _ => String::new(),
                };
                Assignment {
                    stem: stem.clone(), slug: cats[l].slug.clone(), source: Source::Links, score,
                    reason: format!("Links with {} in {}", who, name_of(&cats[l].slug)),
                }
            }
            (None, _) => Assignment {
                stem: stem.clone(), slug: OTHER_SLUG.to_string(), source: Source::Other, score: 0,
                reason: "No links to a categorised note".to_string(),
            },
        };
        out.push(entry);
    }
    (out, warnings)
}


const OUT_JSON: &str = "data/note_category.json";
const RELATED_MAX: usize = 5;

/// `data/note_category.json`, keyed by stem like `note_status.json`.
pub fn to_json(assigns: &[Assignment], cats: &[Category], graph: &Graph) -> serde_json::Value {
    let slug_of: BTreeMap<&str, &str> = assigns.iter().map(|a| (a.stem.as_str(), a.slug.as_str())).collect();
    let mut map = serde_json::Map::new();
    for a in assigns {
        // same-category neighbours by link weight desc then stem: the note's own "More" list
        let mut related: Vec<(&String, u32)> = graph
            .neighbours(&a.stem)
            .map(|nb| nb.iter().filter(|(m, _)| slug_of.get(m.as_str()) == Some(&a.slug.as_str())).map(|(m, &w)| (m, w)).collect())
            .unwrap_or_default();
        related.sort_by(|x, y| y.1.cmp(&x.1).then(x.0.cmp(y.0)));
        let related: Vec<&String> = related.iter().take(RELATED_MAX).map(|(m, _)| *m).collect();
        let (name, group, order) = match cats.iter().position(|c| c.slug == a.slug) {
            Some(i) => (cats[i].name.clone(), cats[i].group.clone(), i + 1),
            None => (OTHER_NAME.to_string(), None, cats.len() + 1),
        };
        map.insert(
            a.stem.clone(),
            serde_json::json!({
                "category": a.slug,
                "name": name,
                "group": group,
                "order": order,
                "path": format!("/brain/{}", crate::slug::brain_slug(&a.stem)),
                "source": a.source.as_str(),
                "score": a.score,
                "degree": graph.degree(&a.stem),
                "reason": a.reason,
                "related": related,
            }),
        );
    }
    serde_json::Value::Object(map)
}

/// Byte offsets of the front-matter lines: (start of first key line, start of closing `---` line).
fn frontmatter_span(text: &str) -> Option<(usize, usize)> {
    let rest = text.strip_prefix("---\n")?;
    let start = 4;
    let mut pos = start;
    for line in rest.split_inclusive('\n') {
        if line == "---\n" || line == "---" {
            return Some((start, pos));
        }
        pos += line.len();
    }
    None
}

/// Insert or replace the single `categories: [<slug>]` line at its sorted key
/// position. Everything else stays byte-identical. `None` when the file
/// already has exactly this line or has no front matter.
pub fn inject_category_line(text: &str, slug: &str) -> Option<String> {
    let (start, end) = frontmatter_span(text)?;
    let wanted = format!("categories: [{}]\n", slug);
    let fm = &text[start..end];
    let mut out = String::with_capacity(text.len() + wanted.len());
    out.push_str(&text[..start]);
    let mut done = false;
    for line in fm.split_inclusive('\n') {
        if line.starts_with("categories:") {
            if line == wanted {
                return None;
            }
            out.push_str(&wanted);
            done = true;
            continue;
        }
        if !done {
            let key = line.split(':').next().unwrap_or("");
            // insert before the first top-level key that sorts after "categories"
            if !line.starts_with(' ') && !line.starts_with('-') && key > "categories" {
                out.push_str(&wanted);
                done = true;
            }
        }
        out.push_str(line);
    }
    if !done {
        out.push_str(&wanted);
    }
    out.push_str(&text[end..]);
    Some(out)
}

/// The slug of a `#category/<slug>` vault override the export wrote
/// (`categories: [<slug>]` together with `category_source: manual`).
pub fn manual_from_frontmatter(text: &str) -> Option<String> {
    let (start, end) = frontmatter_span(text)?;
    let fm = &text[start..end];
    let mut slug = None;
    let mut manual = false;
    for line in fm.lines() {
        if let Some(v) = line.strip_prefix("categories:") {
            slug = Some(v.trim().trim_start_matches('[').trim_end_matches(']').trim().to_string());
        } else if let Some(v) = line.strip_prefix("category_source:") {
            manual = v.trim() == "manual";
        }
    }
    slug.filter(|s| !s.is_empty() && manual)
}


const OUT_REPORT: &str = "note_category_report.md"; // repo root: Hugo loads every file under data/

/// Author report: counts, the `other` list, weak notes, warnings, notes that
/// moved since the previous `note_category.json`, and hub candidates.
pub fn render_report(prev: Option<&serde_json::Value>, assigns: &[Assignment], cats: &[Category], graph: &Graph, warnings: &[String]) -> String {
    let mut out = String::from("# Note category report\n\nGenerated by `obsidian-quartz note-category` (see data/categories.yaml).\n");
    let total = assigns.len();
    let count = |slug: &str| assigns.iter().filter(|a| a.slug == slug).count();
    let other = count(OTHER_SLUG);

    out.push_str(&format!("\n## counts ({} notes)\n\n", total));
    for c in cats {
        out.push_str(&format!("- {} ({}): {}\n", c.name, c.slug, count(&c.slug)));
    }
    out.push_str(&format!("- Other (other): {}\n", other));
    let mut groups: BTreeMap<&str, usize> = BTreeMap::new();
    for c in cats {
        if let Some(g) = &c.group {
            *groups.entry(g.as_str()).or_insert(0) += count(&c.slug);
        }
    }
    if !groups.is_empty() {
        out.push_str("\n### groups\n\n");
        for (g, n) in &groups {
            out.push_str(&format!("- {}: {}\n", g, n));
        }
    }
    if total > 0 && other * 20 > total {
        out.push_str(&format!("\n> `other` is {} of {} notes (> 5%): run `obsidian-quartz note-category --propose` and add hubs.\n", other, total));
    }

    let mut others: Vec<&Assignment> = assigns.iter().filter(|a| a.slug == OTHER_SLUG).collect();
    others.sort_by(|a, b| graph.degree(&b.stem).cmp(&graph.degree(&a.stem)).then(a.stem.cmp(&b.stem)));
    out.push_str(&format!("\n## other ({})\n\n", others.len()));
    for a in &others {
        out.push_str(&format!("- {} (degree {})\n", a.stem, graph.degree(&a.stem)));
    }

    let manual: Vec<&Assignment> = assigns.iter().filter(|a| a.source == Source::Manual).collect();
    if !manual.is_empty() {
        out.push_str(&format!("\n## set by #category/ tag ({})\n\n", manual.len()));
        for a in &manual {
            out.push_str(&format!("- {} -> {}\n", a.stem, a.slug));
        }
    }

    let weak: Vec<&Assignment> = assigns.iter().filter(|a| a.source == Source::Links && a.score == 1).collect();
    out.push_str(&format!("\n## weak, score 1 ({})\n\n", weak.len()));
    for a in &weak {
        out.push_str(&format!("- {} -> {} — {}\n", a.stem, a.slug, a.reason));
    }

    if !warnings.is_empty() {
        out.push_str(&format!("\n## warnings ({})\n\n", warnings.len()));
        for w in warnings {
            out.push_str(&format!("- {}\n", w));
        }
    }

    if prev.is_none() {
        out.push_str("\n> First run (no previous data/note_category.json): every note's HTML changes without a lastmod bump, so after `make deploy` run `make purge-cdn` once (all brain pages).\n");
    }
    if let Some(prev) = prev.and_then(|p| p.as_object()) {
        let mut moved = Vec::new();
        for a in assigns {
            if let Some(old) = prev.get(&a.stem).and_then(|e| e.get("category")).and_then(|c| c.as_str()) {
                if old != a.slug {
                    moved.push(format!("- {}: {} -> {}\n", a.stem, old, a.slug));
                }
            }
        }
        out.push_str(&format!("\n## moved since last run ({})\n\n", moved.len()));
        for m in &moved {
            out.push_str(m);
        }
        if !moved.is_empty() {
            out.push_str("\n> Category pages and these notes' footers changed without a lastmod bump: run `make purge-cdn` after `make deploy`.\n");
        }
    }

    let mut cands: Vec<&Assignment> = assigns.iter().filter(|a| a.slug == OTHER_SLUG || (a.source == Source::Links && a.score == 1)).collect();
    cands.sort_by(|a, b| graph.degree(&b.stem).cmp(&graph.degree(&a.stem)).then(a.stem.cmp(&b.stem)));
    out.push_str("\n## hub candidates (highest-degree notes in other or weak)\n\n");
    for a in cands.iter().take(15) {
        out.push_str(&format!("- {} (degree {}, now {})\n", a.stem, graph.degree(&a.stem), a.slug));
    }
    out
}

/// `obsidian-quartz note-category [--propose]`.
pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let notes = read_content(Path::new(CONTENT_DIR))?;
    let graph = build_graph(&notes);
    let cats = match fs::read_to_string(CATEGORIES_YAML) {
        Ok(text) => Some(parse_categories(&text)?),
        Err(_) => None,
    };
    if args.iter().any(|a| a == "--propose") {
        let mut with_index = notes.clone();
        if let Ok(index) = fs::read_to_string(Path::new(CONTENT_DIR).join("_index.md")) {
            with_index.insert("_index".to_string(), index);
        }
        let text = propose(&graph, &with_index, cats.as_deref());
        fs::write(OUT_PROPOSAL, &text)?;
        println!("note-category: proposal written to {}", OUT_PROPOSAL);
        return Ok(());
    }
    let Some(cats) = cats else {
        eprintln!("note-category: {} not found; categories disabled (no JSON, no front matter change)", CATEGORIES_YAML);
        return Ok(());
    };
    let manual: BTreeMap<String, String> = notes
        .iter()
        .filter_map(|(stem, text)| manual_from_frontmatter(text).map(|s| (stem.clone(), s)))
        .collect();
    let (cats, implicit) = with_manual_categories(cats, &manual);
    let (assigns, mut warnings) = assign(&graph, &cats, &manual);
    for slug in &implicit {
        let n = manual.values().filter(|s| *s == slug).count();
        warnings.push(format!("#category/{} on {} note(s) is not in {}: category created with no hubs/description; add it to the YAML for a proper name and order", slug, n, CATEGORIES_YAML));
    }
    for w in &warnings {
        eprintln!("note-category: {}", w);
    }
    let prev = crate::enrich_with_blog::load_json(OUT_JSON).ok();
    crate::enrich_with_blog::save_json_pretty(OUT_JSON, &to_json(&assigns, &cats, &graph))?;
    let mut changed = 0usize;
    for a in &assigns {
        if a.source == Source::Manual {
            continue;
        }
        let text = &notes[&a.stem];
        if let Some(new_text) = inject_category_line(text, &a.slug) {
            fs::write(Path::new(CONTENT_DIR).join(format!("{}.md", a.stem)), new_text)?;
            changed += 1;
        }
    }
    fs::write(OUT_REPORT, render_report(prev.as_ref(), &assigns, &cats, &graph, &warnings))?;
    let other = assigns.iter().filter(|a| a.slug == OTHER_SLUG).count();
    println!(
        "note-category: {} notes -> {} categories, {} other, {} lines changed -> {} + {}",
        assigns.len(), cats.len(), other, changed, OUT_JSON, OUT_REPORT
    );
    Ok(())
}


const OUT_PROPOSAL: &str = "note_category_proposal.md";
const PROPOSE_MIN_COMMUNITY: usize = 5;
const LPA_ROUNDS: usize = 30;

/// The home page's hand-written topic grid (`content/_index.md`):
/// `(card slug, card name, hub stems in href order)`. Hrefs are matched to
/// stems through `brain_slug`; a stem used by an earlier card is dropped.
pub fn parse_home_grid(index_md: &str, stems: &BTreeSet<String>) -> Vec<(String, String, Vec<String>)> {
    let card_re = Regex::new(r#"(?s)<div class="topic-card ([\w-]+)">(.*?)</div>"#).unwrap();
    let h3_re = Regex::new(r"(?s)<h3>(.*?)</h3>").unwrap();
    let tag_re = Regex::new(r"<[^>]+>").unwrap();
    let href_re = Regex::new(r#"href="([^"]+)""#).unwrap();
    let by_slug: BTreeMap<String, &String> = stems.iter().map(|s| (crate::slug::brain_slug(s), s)).collect();
    let mut used: BTreeSet<&String> = BTreeSet::new();
    let mut out = Vec::new();
    for c in card_re.captures_iter(index_md) {
        let slug = c[1].to_string();
        let body = &c[2];
        let name = h3_re
            .captures(body)
            .map(|h| tag_re.replace_all(&h[1], "").replace("&amp;", "&").split_whitespace().collect::<Vec<_>>().join(" "))
            .unwrap_or_default();
        let mut hubs = Vec::new();
        for h in href_re.captures_iter(body) {
            let href = h[1].trim_end_matches('/');
            let last = href.rsplit('/').next().unwrap_or(href);
            if let Some(stem) = by_slug.get(last) {
                if used.insert(stem) {
                    hubs.push((*stem).clone());
                }
            }
        }
        out.push((slug, name, hubs));
    }
    out
}

/// Deterministic label propagation for the proposal: every node starts as its
/// own label; nodes are visited in sorted order and updated in place, each
/// taking the heaviest label among its neighbours (ties: the label whose
/// note has the higher degree, then the lower stem). Groups are returned
/// largest first, members sorted by degree descending then stem.
pub fn communities(graph: &Graph) -> Vec<Vec<String>> {
    let mut label: BTreeMap<&String, &String> = graph.stems.iter().map(|s| (s, s)).collect();
    for _ in 0..LPA_ROUNDS {
        let mut changed = 0;
        for stem in &graph.stems {
            let Some(nb) = graph.neighbours(stem) else { continue };
            let mut votes: BTreeMap<&String, u32> = BTreeMap::new();
            for (m, w) in nb {
                *votes.entry(label[m]).or_insert(0) += w;
            }
            let best = votes.iter().max_by(|(la, wa), (lb, wb)| {
                wa.cmp(wb)
                    .then(graph.degree(la).cmp(&graph.degree(lb)))
                    .then(lb.cmp(la))
            });
            if let Some((&l, _)) = best {
                if label[stem] != l {
                    label.insert(stem, l);
                    changed += 1;
                }
            }
        }
        if changed == 0 {
            break;
        }
    }
    let mut groups: BTreeMap<&String, Vec<String>> = BTreeMap::new();
    for (stem, l) in &label {
        groups.entry(l).or_default().push((*stem).clone());
    }
    let mut out: Vec<Vec<String>> = groups.into_values().collect();
    for g in &mut out {
        g.sort_by(|a, b| graph.degree(b).cmp(&graph.degree(a)).then(a.cmp(b)));
    }
    out.sort_by(|a, b| b.len().cmp(&a.len()).then(a[0].cmp(&b[0])));
    out
}

fn yaml_list(items: &[String]) -> String {
    format!("[{}]", items.join(", "))
}

/// `note_category_proposal.md`: the home-page grid as a `categories.yaml`
/// block, the link communities, map-like notes and isolated notes.
pub fn propose(graph: &Graph, notes: &BTreeMap<String, String>, cats: Option<&[Category]>) -> String {
    let mut out = String::from("# Note category proposal\n\nGenerated by `obsidian-quartz note-category --propose`. Paste, rename, merge and order into data/categories.yaml.\n");
    let stems: BTreeSet<String> = graph.stems.iter().cloned().collect();

    out.push_str("\n## home-page topic grid as categories.yaml\n\n```yaml\n");
    match notes.get("_index") {
        Some(index) => {
            for (slug, name, hubs) in parse_home_grid(index, &stems) {
                out.push_str(&format!("- slug: {}\n  name: {}\n  hubs: {}\n", slug, name, yaml_list(&hubs)));
            }
        }
        None => out.push_str("# content/_index.md not found\n"),
    }
    out.push_str("```\n");

    let assigned: BTreeMap<String, String> = cats
        .map(|c| assign(graph, c, &BTreeMap::new()).0.into_iter().map(|a| (a.stem, a.slug)).collect())
        .unwrap_or_default();
    let comms = communities(graph);
    let big: Vec<&Vec<String>> = comms.iter().filter(|c| c.len() >= PROPOSE_MIN_COMMUNITY).collect();
    out.push_str(&format!("\n## communities (size >= {}, {} of {})\n", PROPOSE_MIN_COMMUNITY, big.len(), comms.len()));
    for c in big {
        let hubs: Vec<String> = c.iter().take(5).cloned().collect();
        let sample: Vec<String> = c.iter().skip(5).take(10).cloned().collect();
        out.push_str(&format!("\n### {} notes: {}\n\n", c.len(), hubs.join(", ")));
        if !sample.is_empty() {
            out.push_str(&format!("- also: {}\n", sample.join(", ")));
        }
        if !assigned.is_empty() {
            let mut tally: BTreeMap<&str, usize> = BTreeMap::new();
            for m in c {
                *tally.entry(assigned.get(m).map(String::as_str).unwrap_or(OTHER_SLUG)).or_insert(0) += 1;
            }
            let mut t: Vec<(&str, usize)> = tally.into_iter().collect();
            t.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
            let desc: Vec<String> = t.iter().map(|(s, n)| format!("{} {}", n, s)).collect();
            out.push_str(&format!("- currently: {}\n", desc.join(", ")));
        }
        out.push_str(&format!("- hubs: {}\n", yaml_list(&hubs)));
    }

    let mut maps: Vec<&String> = graph
        .stems
        .iter()
        .filter(|s| {
            let o = graph.out_deg.get(*s).copied().unwrap_or(0);
            let i = graph.in_deg.get(*s).copied().unwrap_or(0);
            o >= 20 && o >= 3 * i
        })
        .collect();
    maps.sort_by(|a, b| graph.out_deg[*b].cmp(&graph.out_deg[*a]).then(a.cmp(b)));
    out.push_str(&format!("\n## map-like notes (out >= 20 and >= 3x in, {})\n\n", maps.len()));
    for m in maps {
        out.push_str(&format!("- {} ({} out, {} in)\n", m, graph.out_deg[m], graph.in_deg.get(m).copied().unwrap_or(0)));
    }

    let isolated: Vec<&String> = graph.stems.iter().filter(|s| graph.degree(s) == 0).collect();
    out.push_str(&format!("\n## isolated notes ({})\n\n", isolated.len()));
    for s in isolated {
        out.push_str(&format!("- {}\n", s));
    }
    out
}

/// Top-level `content/*.md` as `stem -> text`, skipping `_index`.
pub fn read_content(dir: &Path) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    let mut out = BTreeMap::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) if s != "_index" => s.to_string(),
            _ => continue,
        };
        out.insert(stem, fs::read_to_string(&path)?);
    }
    if out.is_empty() {
        return Err(format!("no notes found under {}", dir.display()).into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notes(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }


    const YAML: &str = "- slug: data-lakes\n  name: Data lakes\n  group: Data engineering\n  description: Lakes.\n  hubs: [duckdb, delta lake]\n  blog_categories: [data engineering]\n  unknown_key: 1\n- slug: pkm\n  name: PKM\n  hubs: [obsidian]\n";

    #[test]
    fn parse_categories_reads_optional_keys_and_ignores_unknown() {
        let cats = parse_categories(YAML).unwrap();
        assert_eq!(cats.len(), 2);
        assert_eq!(cats[0].slug, "data-lakes");
        assert_eq!(cats[0].group.as_deref(), Some("Data engineering"));
        assert_eq!(cats[0].hubs, vec!["duckdb", "delta lake"]);
        assert_eq!(cats[0].blog_categories, vec!["data engineering"]);
        assert_eq!(cats[1].group, None);
        assert_eq!(cats[1].description, None);
        assert!(cats[1].blog_categories.is_empty());
    }

    #[test]
    fn parse_categories_rejects_bad_slugs() {
        let err = parse_categories("- slug: Data-Eng\n  name: X\n  hubs: [a]\n").unwrap_err();
        assert!(err.contains("lowercase"), "{err}");
        let err = parse_categories("- slug: other\n  name: X\n  hubs: [a]\n").unwrap_err();
        assert!(err.contains("other"), "{err}");
        let err = parse_categories("- slug: a\n  name: X\n  hubs: [a]\n- slug: a\n  name: Y\n  hubs: [b]\n").unwrap_err();
        assert!(err.contains("duplicate"), "{err}");
        let err = parse_categories("- slug: a\n  name: X\n  hubs: []\n").unwrap_err();
        assert!(err.contains("hubs"), "{err}");
    }


    fn cat(slug: &str, hubs: &[&str]) -> Category {
        Category { slug: slug.into(), name: slug.to_uppercase(), group: None, description: None,
                   hubs: hubs.iter().map(|h| h.to_string()).collect(), blog_categories: vec![] }
    }
    fn by_stem(a: &[Assignment], s: &str) -> Assignment { a.iter().find(|x| x.stem == s).unwrap().clone() }

    #[test]
    fn assign_hub_stays_fixed_even_when_neighbours_are_elsewhere() {
        // h is hub of A but links three times to b, hub of B
        let n = notes(&[("h", "[[b]] [[b]] [[b]]"), ("b", "")]);
        let g = build_graph(&n);
        let (a, w) = assign(&g, &[cat("a", &["h"]), cat("b", &["b"])], &BTreeMap::new());
        assert!(w.is_empty());
        let h = by_stem(&a, "h");
        assert_eq!((h.slug.as_str(), h.source), ("a", Source::Hub));
        assert_eq!(h.reason, "Hub of A");
    }

    #[test]
    fn assign_tie_breaks_by_curated_order() {
        let n = notes(&[("x", "[[a]] [[b]]"), ("a", ""), ("b", "")]);
        let g = build_graph(&n);
        let (asg, _) = assign(&g, &[cat("second", &["b"]), cat("first", &["a"])], &BTreeMap::new());
        assert_eq!(by_stem(&asg, "x").slug, "second", "earlier category wins the 1:1 tie");
    }

    #[test]
    fn assign_propagates_along_a_chain_with_reason_and_score() {
        let n = notes(&[("a", "[[b]]"), ("b", "[[c]]"), ("c", "[[d]]"), ("d", ""), ("lonely", "")]);
        let g = build_graph(&n);
        let (asg, _) = assign(&g, &[cat("a", &["a"])], &BTreeMap::new());
        // middle notes see two neighbours in A (score 2), the end only one
        for (s, score) in [("b", 2), ("c", 2), ("d", 1)] {
            let e = by_stem(&asg, s);
            assert_eq!((e.slug.as_str(), e.source, e.score), ("a", Source::Links, score), "{s}");
        }
        assert_eq!(by_stem(&asg, "b").reason, "Links with a and c in A");
        assert_eq!(by_stem(&asg, "d").reason, "Links with c in A");
        let l = by_stem(&asg, "lonely");
        assert_eq!((l.slug.as_str(), l.source, l.score), ("other", Source::Other, 0));
        assert_eq!(l.reason, "No links to a categorised note");
    }

    #[test]
    fn assign_reason_lists_top_two_neighbours_and_count() {
        let n = notes(&[("x", "[[p]] [[p]] [[q]] [[r]] [[s]]"), ("p", ""), ("q", ""), ("r", ""), ("s", "")]);
        let g = build_graph(&n);
        let (asg, _) = assign(&g, &[cat("c", &["p", "q", "r", "s"])], &BTreeMap::new());
        let x = by_stem(&asg, "x");
        assert_eq!(x.score, 5);
        assert_eq!(x.reason, "Links with p, q and 2 more notes in C");
    }

    #[test]
    fn assign_warns_on_duplicate_and_missing_hubs() {
        let n = notes(&[("h", ""), ("z", "")]);
        let g = build_graph(&n);
        let (asg, w) = assign(&g, &[cat("a", &["h", "nope"]), cat("b", &["h"])], &BTreeMap::new());
        assert_eq!(by_stem(&asg, "h").slug, "a");
        assert!(w.iter().any(|m| m == "hub 'h' listed under 'a' and 'b'; using 'a'"), "{w:?}");
        assert!(w.iter().any(|m| m.contains("hub 'nope'") && m.contains("not found")), "{w:?}");
    }

    #[test]
    fn assign_manual_override_wins_and_is_deterministic() {
        let n = notes(&[("h", "[[m]]"), ("m", "")]);
        let g = build_graph(&n);
        let mut manual = BTreeMap::new();
        manual.insert("m".to_string(), "b".to_string());
        let cats = [cat("a", &["h"]), cat("b", &["h2"])];
        let (asg1, _) = assign(&g, &cats, &manual);
        let m = by_stem(&asg1, "m");
        assert_eq!((m.slug.as_str(), m.source), ("b", Source::Manual));
        assert_eq!(m.reason, "Set by author (#category/b)");
        let (asg2, _) = assign(&g, &cats, &manual);
        assert_eq!(asg1, asg2);
        // unknown manual slug: warning, computed instead
        manual.insert("m".to_string(), "zzz".to_string());
        let (asg3, w) = assign(&g, &cats, &manual);
        assert_eq!(by_stem(&asg3, "m").slug, "a");
        assert!(w.iter().any(|x| x.contains("zzz")), "{w:?}");
    }


    const NOTE: &str = "---\naliases: [x]\ncreateddate: 2021-01-01\nlastmod: 2026-01-01 10:00:00\ntitle: \"T\"\n---\nBody [[a]]\n";

    #[test]
    fn inject_inserts_at_sorted_position() {
        let got = inject_category_line(NOTE, "neovim").unwrap();
        assert_eq!(got, "---\naliases: [x]\ncategories: [neovim]\ncreateddate: 2021-01-01\nlastmod: 2026-01-01 10:00:00\ntitle: \"T\"\n---\nBody [[a]]\n");
    }

    #[test]
    fn inject_appends_as_last_key_when_all_sort_before() {
        let got = inject_category_line("---\naliases: [x]\n---\nB\n", "ai").unwrap();
        assert_eq!(got, "---\naliases: [x]\ncategories: [ai]\n---\nB\n");
    }

    #[test]
    fn inject_replaces_existing_line_and_is_idempotent() {
        let once = inject_category_line(NOTE, "neovim").unwrap();
        assert_eq!(inject_category_line(&once, "neovim"), None, "same slug: unchanged");
        let twice = inject_category_line(&once, "writing").unwrap();
        assert!(twice.contains("categories: [writing]\n"));
        assert!(!twice.contains("neovim"));
        assert_eq!(twice.matches("categories:").count(), 1);
    }

    #[test]
    fn inject_leaves_notes_without_front_matter_alone() {
        assert_eq!(inject_category_line("Just a body\n", "ai"), None);
    }

    #[test]
    fn manual_from_frontmatter_needs_both_keys() {
        assert_eq!(manual_from_frontmatter("---\ncategories: [ai]\ncategory_source: manual\n---\n"), Some("ai".into()));
        assert_eq!(manual_from_frontmatter("---\ncategories: [ai]\n---\n"), None);
        assert_eq!(manual_from_frontmatter("---\ncategory_source: manual\n---\n"), None);
    }

    #[test]
    fn to_json_carries_path_order_group_degree() {
        let n = notes(&[("don't stop", "[[duckdb]]"), ("duckdb", ""), ("lonely", "")]);
        let g = build_graph(&n);
        let mut c = cat("data-lakes", &["duckdb"]);
        c.group = Some("Data engineering".into());
        c.name = "Data lakes".into();
        let (asg, _) = assign(&g, &[c.clone()], &BTreeMap::new());
        let j = to_json(&asg, &[c], &g);
        let e = &j["don't stop"];
        assert_eq!(e["category"], "data-lakes");
        assert_eq!(e["name"], "Data lakes");
        assert_eq!(e["group"], "Data engineering");
        assert_eq!(e["order"], 1);
        assert_eq!(e["path"], "/brain/dont-stop");
        assert_eq!(e["source"], "links");
        assert_eq!(e["score"], 1);
        assert_eq!(e["degree"], 1);
        assert_eq!(e["reason"], "Links with duckdb in Data lakes");
        assert_eq!(e["related"], serde_json::json!(["duckdb"]), "same-category neighbours by weight");
        assert_eq!(j["duckdb"]["related"], serde_json::json!(["don't stop"]));
        assert_eq!(j["duckdb"]["source"], "hub");
        let o = &j["lonely"];
        assert_eq!((o["category"].as_str(), o["name"].as_str(), o["order"].as_u64()), (Some("other"), Some("Other"), Some(2)));
        assert!(o["group"].is_null());
    }


    #[test]
    fn report_lists_moved_notes_against_previous_json() {
        let n = notes(&[("a", "[[b]]"), ("b", ""), ("lonely", "")]);
        let g = build_graph(&n);
        let cats = [cat("a", &["a"])];
        let (asg, w) = assign(&g, &cats, &BTreeMap::new());
        let prev = serde_json::json!({"b": {"category": "zzz"}, "lonely": {"category": "other"}});
        let r = render_report(Some(&prev), &asg, &cats, &g, &w);
        assert!(r.contains("## moved since last run (1)"), "{r}");
        assert!(r.contains("- b: zzz -> a\n"), "{r}");
        assert!(r.contains("make purge-cdn"), "{r}");
        assert!(r.contains("## other (1)"), "{r}");
        assert!(r.contains("- lonely (degree 0)"), "{r}");
        let r2 = render_report(None, &asg, &cats, &g, &w);
        assert!(!r2.contains("moved since"), "{r2}");
        assert!(r2.contains("make purge-cdn"), "first run reminds about the full purge:\n{r2}");
    }


    #[test]
    fn parse_home_grid_maps_hrefs_to_stems_and_drops_duplicates() {
        let html = r#"<div class="topic-grid">
  <div class="topic-card data-engineering">
    <h3><a href="data-engineering" class="internal-link">Data Engineering</a></h3>
    <p>Visit the <a href="data-engineering" class="internal-link">Vault</a>, <a href="apache-kafka">Kafka</a>, <a href="missing-note">x</a>.</p>
  </div>
  <div class="topic-card writing-reading">
    <h3><a href="writing" class="internal-link">Writing</a> &amp; <a href="reading" class="internal-link">Reading</a></h3>
    <p><a href="apache-kafka">dup</a>, <a href="https://www.ssp.sh/brain/dont-stop/">abs</a></p>
  </div>
</div>"#;
        let stems: BTreeSet<String> = ["data engineering", "apache kafka", "writing", "reading", "don't stop"].iter().map(|s| s.to_string()).collect();
        let grid = parse_home_grid(html, &stems);
        assert_eq!(grid.len(), 2);
        assert_eq!(grid[0], ("data-engineering".to_string(), "Data Engineering".to_string(), vec!["data engineering".to_string(), "apache kafka".to_string()]));
        assert_eq!(grid[1].0, "writing-reading");
        assert_eq!(grid[1].1, "Writing & Reading");
        assert_eq!(grid[1].2, vec!["writing", "reading", "don't stop"], "duplicate kafka dropped, absolute href resolved");
    }

    #[test]
    fn communities_splits_two_disconnected_triangles() {
        let n = notes(&[("a", "[[b]] [[c]]"), ("b", "[[c]]"), ("c", ""), ("x", "[[y]] [[z]]"), ("y", "[[z]]"), ("z", ""), ("solo", "")]);
        let g = build_graph(&n);
        let comms = communities(&g);
        assert_eq!(comms.len(), 3);
        assert_eq!(comms[0].len(), 3);
        assert_eq!(comms[1].len(), 3);
        assert_eq!(comms[2], vec!["solo"]);
        let all: BTreeSet<&String> = comms.iter().flatten().collect();
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn propose_lists_grid_yaml_communities_maps_and_isolated() {
        let mut n = notes(&[("a", "[[b]] [[c]]"), ("b", "[[c]]"), ("c", ""), ("solo", "")]);
        n.insert("_index".into(), "<div class=\"topic-card pkm\"><h3><a href=\"a\">PKM</a></h3></div>".into());
        let g = build_graph(&n);
        let text = propose(&g, &n, None);
        assert!(text.contains("- slug: pkm\n  name: PKM\n  hubs: [a]\n"), "{text}");
        assert!(text.contains("## communities"), "{text}");
        assert!(text.contains("## isolated notes (1)\n\n- solo"), "{text}");
        assert!(text.contains("## map-like notes"), "{text}");
    }


    #[test]
    fn with_manual_categories_appends_unknown_slugs_as_implicit_categories() {
        let cats = vec![cat("a", &["h"])];
        let mut manual = BTreeMap::new();
        manual.insert("n1".to_string(), "moc".to_string());
        manual.insert("n2".to_string(), "a".to_string());
        manual.insert("n3".to_string(), "deep-life".to_string());
        let (all, added) = with_manual_categories(cats, &manual);
        assert_eq!(all.iter().map(|c| c.slug.as_str()).collect::<Vec<_>>(), vec!["a", "deep-life", "moc"]);
        assert_eq!(all[1].name, "Deep Life");
        assert_eq!(all[2].name, "Moc");
        assert!(all[2].hubs.is_empty() && all[2].description.is_none());
        assert_eq!(added, vec!["deep-life", "moc"]);
    }

    #[test]
    fn parse_links_strips_alias_heading_embed_and_path() {
        let got = parse_links("[[DuckDB|the duck]] ![[Delta Lake#History]] [[a/b/Apache Arrow.md]] [[x]]");
        assert_eq!(got, vec!["duckdb", "delta lake", "apache arrow", "x"]);
    }

    #[test]
    fn build_graph_drops_missing_self_and_index_counts_both_directions() {
        let n = notes(&[
            ("a", "[[b]] [[b]] [[a]] [[zzz]] [[_index]]"),
            ("b", "[[a]]"),
            ("c", ""),
        ]);
        let g = build_graph(&n);
        assert_eq!(g.adj["a"]["b"], 3);
        assert_eq!(g.adj["b"]["a"], 3);
        assert!(g.adj.get("c").map_or(true, |m| m.is_empty()));
        assert_eq!(g.degree("a"), 3);
        assert_eq!(g.degree("c"), 0);
        assert_eq!(g.stems, vec!["a", "b", "c"]);
        assert_eq!(g.out_deg["a"], 2, "two outgoing links to existing other notes");
        assert_eq!(g.in_deg["a"], 1);
    }
}
