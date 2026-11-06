//! Note categories: one category per note from the wikilink graph and the
//! author-curated hub list in `data/categories.yaml`.
//! Spec: docs/superpowers/specs/2026-11-06-note-categories-design.md

use std::collections::{BTreeMap, BTreeSet};

use std::error::Error;
use std::fs;
use std::path::Path;

use regex::Regex;

const CONTENT_DIR: &str = "content";

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
