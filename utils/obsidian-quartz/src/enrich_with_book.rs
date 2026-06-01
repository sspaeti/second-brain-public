use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use regex::Regex;
use serde_json::{json, Value};

const BOOK_LINK_INDEX: &str = "../../book/dedp/linkIndex.js";
const BOOK_SUMMARY_PUBLISHED: &str = "../../book/dedp/src/SUMMARY_published.md";
const BRAIN_LINK_INDEX: &str = "assets/indices/linkIndex.json";
const BRAIN_CONTENT: &str = "assets/indices/contentIndex.json";

fn load_json(path: &str) -> Result<Value, Box<dyn Error>> {
    let f = File::open(path)?;
    Ok(serde_json::from_reader(BufReader::new(f))?)
}

fn save_json_pretty(path: &str, v: &Value) -> Result<(), Box<dyn Error>> {
    let f = File::create(path)?;
    serde_json::to_writer_pretty(BufWriter::new(f), v)?;
    Ok(())
}

/// Parse `SUMMARY_published.md` and return the set of chapter IDs (e.g. `/part-2/4-ce/X`)
/// that are currently published. HTML comments `<!-- ... -->` are stripped first so
/// commented-out future chapters aren't included.
fn load_published_chapters(path: &Path) -> Option<std::collections::HashSet<String>> {
    let text = fs::read_to_string(path).ok()?;
    let comment_re = Regex::new(r"(?s)<!--.*?-->").unwrap();
    let cleaned = comment_re.replace_all(&text, "");
    let link_re = Regex::new(r"\[[^\]]*\]\(([^)]+\.md)\)").unwrap();
    let mut set = std::collections::HashSet::new();
    for cap in link_re.captures_iter(&cleaned) {
        let p = &cap[1];
        let chapter = format!("/{}", p.trim_end_matches(".md"));
        set.insert(chapter);
    }
    Some(set)
}

/// Read `linkIndex.js` which is `window.DEDP_LINK_INDEX = {...}` and return the inner JSON.
fn parse_book_link_index(path: &Path) -> Result<Value, Box<dyn Error>> {
    let text = fs::read_to_string(path)?;
    let json_start = text.find('{').ok_or("no JSON object found in book linkIndex.js")?;
    let json_end = text.rfind('}').ok_or("no closing brace in book linkIndex.js")?;
    let json_str = &text[json_start..=json_end];
    Ok(serde_json::from_str(json_str)?)
}

fn strip_fragment(s: &str, frag_re: &Regex) -> String {
    frag_re.replace(s, "").to_string()
}

/// Convert a book target ID like `brain:cron` or `brain:active-record#section`
/// to a brain node ID like `/cron`. Returns None for non-brain targets.
fn brain_id_from_book_target(raw: &str, frag_re: &Regex) -> Option<String> {
    let stripped = raw.strip_prefix("brain:")?;
    let no_frag = strip_fragment(stripped, frag_re);
    if no_frag.is_empty() {
        None
    } else {
        Some(format!("/{}", no_frag))
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    if !Path::new(BOOK_LINK_INDEX).exists() {
        eprintln!(
            "enrich-with-book: book link index not found at {}, skipping",
            BOOK_LINK_INDEX
        );
        return Ok(());
    }

    let book_data = parse_book_link_index(Path::new(BOOK_LINK_INDEX))?;
    let mut brain_index = load_json(BRAIN_LINK_INDEX)?;
    let mut brain_content = load_json(BRAIN_CONTENT)?;

    let published = load_published_chapters(Path::new(BOOK_SUMMARY_PUBLISHED));
    if published.is_none() {
        eprintln!(
            "enrich-with-book: {} not found, importing ALL chapters (no published filter)",
            BOOK_SUMMARY_PUBLISHED
        );
    }

    let frag_re = Regex::new(r"#.*$").unwrap();

    let mut chapter_titles: HashMap<String, String> = HashMap::new();
    if let Some(nodes) = book_data.get("nodes").and_then(|v| v.as_array()) {
        for node in nodes {
            let node_type = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if node_type != "chapter" {
                continue;
            }
            if let (Some(id), Some(title)) = (
                node.get("id").and_then(|v| v.as_str()),
                node.get("title").and_then(|v| v.as_str()),
            ) {
                chapter_titles.insert(id.to_string(), title.to_string());
            }
        }
    }

    let brain_ids: HashSet<String> = brain_content
        .as_object()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();

    let mut existing_pairs: HashSet<(String, String)> = HashSet::new();
    if let Some(arr) = brain_index["links"].as_array() {
        for edge in arr {
            if let (Some(s), Some(t)) = (edge["source"].as_str(), edge["target"].as_str()) {
                existing_pairs.insert((s.to_string(), t.to_string()));
            }
        }
    }

    let mut new_edges: Vec<(String, String)> = Vec::new();
    let mut new_edges_seen: HashSet<(String, String)> = HashSet::new();
    let mut book_ids_used: HashSet<String> = HashSet::new();
    let mut skipped_unknown_chapter = 0usize;
    let mut skipped_unpub_brain = 0usize;
    let mut skipped_unpublished_chapter = 0usize;
    let mut skipped_non_brain_edge = 0usize;

    if let Some(edges) = book_data.get("edges").and_then(|v| v.as_array()) {
        for edge in edges {
            let edge_type = edge.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if edge_type != "brain" {
                skipped_non_brain_edge += 1;
                continue;
            }
            let raw_src = edge.get("source").and_then(|v| v.as_str()).unwrap_or("");
            let raw_tgt = edge.get("target").and_then(|v| v.as_str()).unwrap_or("");

            if !chapter_titles.contains_key(raw_src) {
                skipped_unknown_chapter += 1;
                continue;
            }
            if let Some(ref pub_set) = published {
                if !pub_set.contains(raw_src) {
                    skipped_unpublished_chapter += 1;
                    continue;
                }
            }
            let brain_target = match brain_id_from_book_target(raw_tgt, &frag_re) {
                Some(b) => b,
                None => continue,
            };
            if !brain_ids.contains(&brain_target) {
                skipped_unpub_brain += 1;
                continue;
            }

            let book_source = format!("/book{}", raw_src);
            let pair = (book_source.clone(), brain_target.clone());
            if existing_pairs.contains(&pair) || new_edges_seen.contains(&pair) {
                continue;
            }
            new_edges_seen.insert(pair);
            book_ids_used.insert(book_source.clone());
            new_edges.push((book_source, brain_target));
        }
    }

    for (src, tgt) in &new_edges {
        let edge_obj = json!({ "source": src, "target": tgt, "text": "" });

        if let Some(arr) = brain_index["links"].as_array_mut() {
            arr.push(edge_obj.clone());
        }
        if let Some(links_obj) = brain_index["index"]["links"].as_object_mut() {
            links_obj
                .entry(src.clone())
                .or_insert_with(|| Value::Array(vec![]))
                .as_array_mut()
                .unwrap()
                .push(edge_obj.clone());
        }
        if let Some(backlinks_obj) = brain_index["index"]["backlinks"].as_object_mut() {
            backlinks_obj
                .entry(tgt.clone())
                .or_insert_with(|| Value::Array(vec![]))
                .as_array_mut()
                .unwrap()
                .push(edge_obj);
        }
    }

    let mut new_book_nodes = 0usize;
    if let Value::Object(map) = &mut brain_content {
        for book_id in &book_ids_used {
            if !map.contains_key(book_id) {
                let chapter_id = book_id.trim_start_matches("/book").to_string();
                let title = chapter_titles
                    .get(&chapter_id)
                    .cloned()
                    .unwrap_or_else(|| chapter_id.replace('/', " ").trim().to_string());
                map.insert(
                    book_id.clone(),
                    json!({
                        "title": title,
                        "content": "",
                        "lastmodified": "",
                        "tags": [],
                        "type": "book",
                    }),
                );
                new_book_nodes += 1;
            }
        }
    }

    save_json_pretty(BRAIN_LINK_INDEX, &brain_index)?;
    save_json_pretty(BRAIN_CONTENT, &brain_content)?;

    println!(
        "enrich-with-book: added {} edges, {} book nodes (skipped: non-brain-edge={}, unknown-chapter={}, unpublished-chapter={}, unpublished-brain={})",
        new_edges.len(),
        new_book_nodes,
        skipped_non_brain_edge,
        skipped_unknown_chapter,
        skipped_unpublished_chapter,
        skipped_unpub_brain,
    );

    Ok(())
}
