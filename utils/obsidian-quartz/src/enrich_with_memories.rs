use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::path::Path;

use regex::Regex;
use serde_json::{json, Value};

// Reuse the JSON + slug helpers from enrich_with_blog so the memory→brain slugs
// resolve identically to how hugo-obsidian slugified the brain notes.
use crate::enrich_with_blog::{load_json, save_json_pretty, unicode_sanitize};

const MEMORIES_DIR: &str = "../sspaeti-hugo-blog/content/memories";
const BRAIN_LINK_INDEX: &str = "assets/indices/linkIndex.json";
const BRAIN_CONTENT: &str = "assets/indices/contentIndex.json";

/// Fallback title from a memory bundle folder name: strip the leading
/// `YYYY-MM-DD-`, turn `-` into spaces, title-case each word.
/// Mirrors layouts/partials/memory-title.html.
fn derive_title(folder: &str, date_re: &Regex) -> String {
    let stripped = date_re.replace(folder, "");
    stripped
        .split('-')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut ch = w.chars();
            match ch.next() {
                Some(f) => f.to_uppercase().collect::<String>() + ch.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn run() -> Result<(), Box<dyn Error>> {
    if !Path::new(MEMORIES_DIR).exists() {
        eprintln!(
            "enrich-with-memories: memories dir not found at {}, skipping",
            MEMORIES_DIR
        );
        return Ok(());
    }

    let mut brain_index = load_json(BRAIN_LINK_INDEX)?;
    let mut brain_content = load_json(BRAIN_CONTENT)?;

    // Published brain notes = keys of contentIndex. A memory wikilink whose
    // target isn't here (unpublished / typo) is skipped, never a dangling node.
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

    let wikilink_re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
    let title_re = Regex::new(r#"(?m)^title:\s*["']?(.+?)["']?\s*$"#).unwrap();
    let date_re = Regex::new(r"^\d{4}-\d{2}-\d{2}-").unwrap();

    let mut new_edges: Vec<(String, String, String)> = Vec::new();
    let mut new_edges_seen: HashSet<(String, String)> = HashSet::new();
    let mut memory_titles: HashMap<String, String> = HashMap::new();
    let mut memory_ids_used: HashSet<String> = HashSet::new();
    let mut skipped_unpub = 0usize;

    // Stable order so output diffs are deterministic.
    let mut folders: Vec<_> = fs::read_dir(MEMORIES_DIR)?.flatten().map(|e| e.path()).collect();
    folders.sort();

    for path in folders {
        if !path.is_dir() {
            continue;
        }
        let md = path.join("index.md");
        if !md.exists() {
            continue;
        }
        let folder = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let text = match fs::read_to_string(&md) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let memory_id = format!("/memories/{}", folder);
        let title = title_re
            .captures(&text)
            .map(|c| c[1].trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| derive_title(&folder, &date_re));

        for caps in wikilink_re.captures_iter(&text) {
            let raw = caps[1].trim();
            // [[Target|Display]] — the target is before the pipe; the display
            // text becomes the edge label (matches process-wikilinks.html).
            let (target_raw, display) = match raw.split_once('|') {
                Some((t, d)) => (t.trim(), d.trim().to_string()),
                None => (raw, raw.to_string()),
            };
            if target_raw == "_index" {
                continue; // brain root special case, no memory edge
            }
            let brain_target = format!("/{}", unicode_sanitize(&target_raw.to_lowercase()));
            if !brain_ids.contains(&brain_target) {
                skipped_unpub += 1;
                continue;
            }
            let pair = (memory_id.clone(), brain_target.clone());
            if existing_pairs.contains(&pair) || new_edges_seen.contains(&pair) {
                continue;
            }
            new_edges_seen.insert(pair);
            memory_ids_used.insert(memory_id.clone());
            memory_titles
                .entry(memory_id.clone())
                .or_insert_with(|| title.clone());
            new_edges.push((memory_id.clone(), brain_target, display));
        }
    }

    for (src, tgt, text) in &new_edges {
        let edge = json!({ "source": src, "target": tgt, "text": text });

        if let Some(arr) = brain_index["links"].as_array_mut() {
            arr.push(edge.clone());
        }
        if let Some(links_obj) = brain_index["index"]["links"].as_object_mut() {
            links_obj
                .entry(src.clone())
                .or_insert_with(|| Value::Array(vec![]))
                .as_array_mut()
                .unwrap()
                .push(edge.clone());
        }
        if let Some(backlinks_obj) = brain_index["index"]["backlinks"].as_object_mut() {
            backlinks_obj
                .entry(tgt.clone())
                .or_insert_with(|| Value::Array(vec![]))
                .as_array_mut()
                .unwrap()
                .push(edge);
        }
    }

    let mut new_nodes = 0usize;
    if let Value::Object(map) = &mut brain_content {
        for mid in &memory_ids_used {
            if !map.contains_key(mid) {
                let title = memory_titles.get(mid).cloned().unwrap_or_else(|| {
                    mid.trim_start_matches("/memories/").replace('-', " ")
                });
                map.insert(
                    mid.clone(),
                    json!({
                        "title": title,
                        "content": "",
                        "lastmodified": "",
                        "tags": [],
                        "type": "memory",
                    }),
                );
                new_nodes += 1;
            }
        }
    }

    save_json_pretty(BRAIN_LINK_INDEX, &brain_index)?;
    save_json_pretty(BRAIN_CONTENT, &brain_content)?;

    println!(
        "enrich-with-memories: added {} edges, {} memory nodes (skipped: unpublished-brain={})",
        new_edges.len(),
        new_nodes,
        skipped_unpub,
    );

    Ok(())
}
