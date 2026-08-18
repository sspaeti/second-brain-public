use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use regex::Regex;
use serde_json::{json, Value};

const BLOG_LINK_INDEX: &str = "../sspaeti-hugo-blog/static/indices/linkIndex.json";
const BLOG_CONTENT: &str = "../sspaeti-hugo-blog/static/indices/contentIndex.json";
const BLOG_POSTS_DIR: &str = "../sspaeti-hugo-blog/content/posts";
const BRAIN_LINK_INDEX: &str = "assets/indices/linkIndex.json";
const BRAIN_CONTENT: &str = "assets/indices/contentIndex.json";

/// Walk the blog posts directory and build a map from date-prefixed folder ID
/// (e.g. `/2025-11-18-owning-things`) to its canonical Hugo URL
/// (e.g. `/blog/owning-things-attention`), respecting `url:` frontmatter overrides.
pub fn build_blog_url_map(posts_dir: &Path) -> HashMap<String, String> {
    let url_re = Regex::new(r#"(?m)^url:\s*["']?([^"'\s]+)["']?\s*$"#).unwrap();
    let mut map = HashMap::new();
    let entries = match fs::read_dir(posts_dir) {
        Ok(e) => e,
        Err(_) => return map,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let folder = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let source_id = format!("/{}", folder);
        let mut canonical: Option<String> = None;
        for fname in &["index.en.md", "index.md"] {
            let md = path.join(fname);
            if !md.exists() {
                continue;
            }
            if let Ok(text) = fs::read_to_string(&md) {
                let head: String = text.lines().take(40).collect::<Vec<_>>().join("\n");
                if let Some(caps) = url_re.captures(&head) {
                    let mut url = caps[1].to_string();
                    if url.len() > 1 {
                        url = url.trim_end_matches('/').to_string();
                    }
                    canonical = Some(url);
                    break;
                }
            }
        }
        let canonical = canonical.unwrap_or_else(|| {
            let slug = folder
                .splitn(4, '-')
                .nth(3)
                .unwrap_or(&folder)
                .to_string();
            format!("/blog/{}", slug)
        });
        map.insert(source_id, canonical);
    }
    map
}

pub fn load_json(path: &str) -> Result<Value, Box<dyn Error>> {
    let f = File::open(path)?;
    Ok(serde_json::from_reader(BufReader::new(f))?)
}

pub fn save_json_pretty(path: &str, v: &Value) -> Result<(), Box<dyn Error>> {
    let f = File::create(path)?;
    serde_json::to_writer_pretty(BufWriter::new(f), v)?;
    Ok(())
}

fn strip_fragment_and_slash(s: &str, frag_re: &Regex) -> String {
    let no_frag = frag_re.replace(s, "").to_string();
    if no_frag.len() > 1 {
        no_frag.trim_end_matches('/').to_string()
    } else {
        no_frag
    }
}

// The slug rules live in slug.rs (single copy in this crate, mirrored from
// hugo-obsidian's util.go). Re-exported here because merge_search_index and
// enrich_with_memories already import it from this module.
pub use crate::slug::unicode_sanitize;

// Brain canonical slug from a blog-linkIndex source/target string.
fn normalize_brain_slug(raw: &str, frag_re: &Regex) -> String {
    unicode_sanitize(&strip_fragment_and_slash(raw, frag_re))
}

fn normalize_endpoint_inplace(edge: &mut Value, key: &str, frag_re: &Regex) {
    let new_val = edge
        .get(key)
        .and_then(|v| v.as_str())
        .filter(|s| s.starts_with("/blog/"))
        .map(|s| strip_fragment_and_slash(s, frag_re))
        .filter(|norm| edge.get(key).and_then(|v| v.as_str()) != Some(norm.as_str()));
    if let Some(norm) = new_val {
        edge[key] = Value::String(norm);
    }
}

fn normalize_existing_blog_refs(brain_index: &mut Value, frag_re: &Regex) {
    if let Some(arr) = brain_index["links"].as_array_mut() {
        for edge in arr.iter_mut() {
            normalize_endpoint_inplace(edge, "source", frag_re);
            normalize_endpoint_inplace(edge, "target", frag_re);
        }
    }
    for section in &["links", "backlinks"] {
        if let Some(obj) = brain_index["index"][*section].as_object_mut() {
            for val in obj.values_mut() {
                if let Some(arr) = val.as_array_mut() {
                    for edge in arr.iter_mut() {
                        normalize_endpoint_inplace(edge, "source", frag_re);
                        normalize_endpoint_inplace(edge, "target", frag_re);
                    }
                }
            }
            let renames: Vec<(String, String)> = obj
                .keys()
                .filter(|k| k.starts_with("/blog/"))
                .map(|k| (k.clone(), strip_fragment_and_slash(k, frag_re)))
                .filter(|(old, new)| old != new)
                .collect();
            for (old_key, new_key) in renames {
                if let Some(old_val) = obj.remove(&old_key) {
                    if let Some(existing) = obj.get_mut(&new_key) {
                        if let (Some(ex_arr), Some(old_arr)) =
                            (existing.as_array_mut(), old_val.as_array())
                        {
                            for e in old_arr {
                                ex_arr.push(e.clone());
                            }
                        }
                    } else {
                        obj.insert(new_key, old_val);
                    }
                }
            }
        }
    }
}

fn normalize_blog_id(
    raw: &str,
    blog_url_map: &HashMap<String, String>,
    frag_re: &Regex,
) -> Option<String> {
    let cleaned = strip_fragment_and_slash(raw, frag_re);
    if cleaned.starts_with("/blog/") {
        return Some(cleaned.to_lowercase());
    }
    blog_url_map.get(&cleaned).cloned()
}

pub fn run() -> Result<(), Box<dyn Error>> {
    if !Path::new(BLOG_LINK_INDEX).exists() || !Path::new(BLOG_CONTENT).exists() {
        eprintln!(
            "enrich-with-blog: blog indices not found at {}, skipping",
            BLOG_LINK_INDEX
        );
        return Ok(());
    }

    let blog_index = load_json(BLOG_LINK_INDEX)?;
    let blog_content = load_json(BLOG_CONTENT)?;
    let mut brain_index = load_json(BRAIN_LINK_INDEX)?;
    let mut brain_content = load_json(BRAIN_CONTENT)?;

    let frag_re = Regex::new(r"#.*$").unwrap();

    normalize_existing_blog_refs(&mut brain_index, &frag_re);

    let blog_url_map = build_blog_url_map(Path::new(BLOG_POSTS_DIR));
    let mut blog_titles: HashMap<String, String> = HashMap::new();
    if let Value::Object(map) = &blog_content {
        for (key, val) in map {
            if let Some(canonical) = blog_url_map.get(key) {
                if let Some(title) = val.get("title").and_then(|v| v.as_str()) {
                    blog_titles.insert(canonical.clone(), title.to_string());
                }
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

    let mut new_edges: Vec<(String, String, String)> = Vec::new();
    let mut new_edges_seen: HashSet<(String, String)> = HashSet::new();
    let mut blog_ids_used: HashSet<String> = HashSet::new();
    let mut skipped_b2b = 0usize;
    let mut skipped_stale_blog = 0usize;
    let mut skipped_unpub_brain = 0usize;

    if let Some(arr) = blog_index["links"].as_array() {
        for edge in arr {
            let raw_src = edge["source"].as_str().unwrap_or("");
            let raw_tgt = edge["target"].as_str().unwrap_or("");
            let text = edge["text"].as_str().unwrap_or("").to_string();

            let s_norm = normalize_blog_id(raw_src, &blog_url_map, &frag_re);
            let t_norm = normalize_blog_id(raw_tgt, &blog_url_map, &frag_re);
            let s_is_blog = s_norm.is_some();
            let t_is_blog = t_norm.is_some();

            if s_is_blog == t_is_blog {
                if s_is_blog {
                    skipped_b2b += 1;
                }
                continue;
            }

            let (new_source, new_target) = if s_is_blog {
                let blog_id = s_norm.unwrap();
                if !blog_titles.contains_key(&blog_id) {
                    skipped_stale_blog += 1;
                    continue;
                }
                let brain_side = normalize_brain_slug(raw_tgt, &frag_re);
                if !brain_ids.contains(&brain_side) {
                    skipped_unpub_brain += 1;
                    continue;
                }
                (blog_id, brain_side)
            } else {
                let blog_id = t_norm.unwrap();
                if !blog_titles.contains_key(&blog_id) {
                    skipped_stale_blog += 1;
                    continue;
                }
                let brain_side = normalize_brain_slug(raw_src, &frag_re);
                if !brain_ids.contains(&brain_side) {
                    skipped_unpub_brain += 1;
                    continue;
                }
                (brain_side, blog_id)
            };

            let pair = (new_source.clone(), new_target.clone());
            if existing_pairs.contains(&pair) || new_edges_seen.contains(&pair) {
                continue;
            }
            new_edges_seen.insert(pair);

            if new_source.starts_with("/blog/") {
                blog_ids_used.insert(new_source.clone());
            }
            if new_target.starts_with("/blog/") {
                blog_ids_used.insert(new_target.clone());
            }

            new_edges.push((new_source, new_target, text));
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

    let mut new_blog_nodes = 0usize;
    if let Value::Object(map) = &mut brain_content {
        for blog_id in &blog_ids_used {
            if !map.contains_key(blog_id) {
                let title = blog_titles.get(blog_id).cloned().unwrap_or_else(|| {
                    blog_id.trim_start_matches("/blog/").replace('-', " ")
                });
                map.insert(
                    blog_id.clone(),
                    json!({
                        "title": title,
                        "content": "",
                        "lastmodified": "",
                        "tags": [],
                        "type": "blog",
                    }),
                );
                new_blog_nodes += 1;
            }
        }
    }

    save_json_pretty(BRAIN_LINK_INDEX, &brain_index)?;
    save_json_pretty(BRAIN_CONTENT, &brain_content)?;

    println!(
        "enrich-with-blog: added {} edges, {} blog nodes (skipped: blog2blog={}, stale-blog={}, unpublished-brain={})",
        new_edges.len(),
        new_blog_nodes,
        skipped_b2b,
        skipped_stale_blog,
        skipped_unpub_brain,
    );

    Ok(())
}
