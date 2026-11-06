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
    g.stems = notes.keys().cloned().collect();
    let exists: BTreeSet<&String> = notes.keys().collect();
    for (stem, text) in notes {
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
