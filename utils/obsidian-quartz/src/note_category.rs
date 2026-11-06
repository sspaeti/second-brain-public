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
