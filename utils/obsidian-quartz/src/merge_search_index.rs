use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;

use regex::Regex;
use serde_json::{json, Map, Value};

use crate::enrich_with_blog::{build_blog_url_map, load_json, save_json_pretty, unicode_sanitize};

const BLOG_POSTS_DIR: &str = "../sspaeti-hugo-blog/content/posts";
const BLOG_CONTENT: &str = "../sspaeti-hugo-blog/static/indices/contentIndex.json";
const BRAIN_CONTENT: &str = "assets/indices/contentIndex.json";
const OUT_BRAIN: &str = "assets/indices/searchIndex-v2.json";
const OUT_BLOG: &str = "../sspaeti-hugo-blog/static/indices/searchIndex-v2.json";

fn truncate_to_date(iso: &str) -> String {
    iso.get(..10).unwrap_or("").to_string()
}

struct PostMeta {
    created: Option<String>,
    updated: Option<String>,
    categories: Vec<String>,
}

fn read_blog_post_meta(posts_dir: &Path) -> HashMap<String, PostMeta> {
    let date_re = Regex::new(r#"(?m)^date:\s*["']?(\d{4}-\d{2}-\d{2})"#).unwrap();
    let lastmod_re = Regex::new(r#"(?m)^lastmod:\s*["']?(\d{4}-\d{2}-\d{2})"#).unwrap();
    // handles: categories: ["Foo", "Bar"] or categories: [Foo, Bar]
    let cats_inline_re = Regex::new(r#"(?m)^categories:\s*\[([^\]]*)\]"#).unwrap();
    // handles block list:
    //   categories:
    //     - Foo
    let cats_block_re = Regex::new(r"(?ms)^categories:\s*\n((?:[ \t]*-[ \t]*[^\n]+\n)+)").unwrap();

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
        for fname in &["index.en.md", "index.md"] {
            let md = path.join(fname);
            if !md.exists() {
                continue;
            }
            if let Ok(text) = fs::read_to_string(&md) {
                let head: String = text.lines().take(40).collect::<Vec<_>>().join("\n");

                let created = date_re.captures(&head).map(|c| c[1].to_string());
                let updated = lastmod_re.captures(&head).map(|c| c[1].to_string());

                let categories = cats_inline_re
                    .captures(&head)
                    .map(|c| {
                        c[1].split(',')
                            .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                            .filter(|s| !s.is_empty())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_else(|| {
                        cats_block_re
                            .captures(&head)
                            .map(|c| {
                                c[1].lines()
                                    .map(|l| l.trim().trim_start_matches('-').trim().to_string())
                                    .filter(|s| !s.is_empty())
                                    .collect()
                            })
                            .unwrap_or_default()
                    });

                map.insert(source_id, PostMeta { created, updated, categories });
                break;
            }
        }
    }
    map
}

fn walk_md_files(dir: &Path, result: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_md_files(&path, result);
            } else if path.extension().map_or(false, |e| e == "md") {
                result.push(path);
            }
        }
    }
}

fn read_brain_note_dates(content_dir: &Path) -> HashMap<String, (Option<String>, Option<String>)> {
    let created_re = Regex::new(r#"(?m)^createddate:\s*["']?(\d{4}-\d{2}-\d{2})"#).unwrap();
    let lastmod_re = Regex::new(r#"(?m)^lastmod:\s*["']?(\d{4}-\d{2}-\d{2})"#).unwrap();

    let mut files = Vec::new();
    walk_md_files(content_dir, &mut files);

    let mut map = HashMap::new();
    for path in &files {
        let rel = match path.strip_prefix(content_dir) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let rel_str = match rel.to_str() {
            Some(s) => s,
            None => continue,
        };
        // Derive slug: strip .md, strip trailing /index, sanitize (matches hugo-obsidian), prepend /
        let no_ext = rel_str.trim_end_matches(".md");
        let slug_base = if no_ext.ends_with("/index") {
            &no_ext[..no_ext.len() - "/index".len()]
        } else {
            no_ext
        };
        let slug = format!("/{}", unicode_sanitize(slug_base));

        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let head: String = text.lines().take(30).collect::<Vec<_>>().join("\n");
        let created = created_re.captures(&head).map(|c| c[1].to_string());
        let lastmod = lastmod_re.captures(&head).map(|c| c[1].to_string());
        map.insert(slug, (created, lastmod));
    }
    map
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let brain_idx = load_json(BRAIN_CONTENT)?;
    let blog_idx = load_json(BLOG_CONTENT)?;

    let url_map = build_blog_url_map(Path::new(BLOG_POSTS_DIR));
    let meta_map = read_blog_post_meta(Path::new(BLOG_POSTS_DIR));
    let brain_dates = read_brain_note_dates(Path::new("content"));

    let mut merged: Map<String, Value> = Map::new();

    // Brain entries: prefix slug with /brain, dates from frontmatter
    let mut brain_count = 0usize;
    if let Some(obj) = brain_idx.as_object() {
        for (key, val) in obj {
            if key.starts_with("/blog/") { continue; }
            let url = format!("/brain{}", key);
            let (created, updated) = match brain_dates.get(key) {
                Some((Some(c), Some(u))) => (c.clone(), u.clone()),
                Some((Some(c), None))    => (c.clone(), c.clone()),
                Some((None, Some(u)))    => (u.clone(), u.clone()),
                _                        => (String::new(), String::new()),
            };
            let entry = json!({
                "title":    val["title"].as_str().unwrap_or(""),
                "content":  val["content"].as_str().unwrap_or(""),
                "source":   "brain",
                "tags":     val["tags"].as_array().cloned().unwrap_or_default(),
                "category": [],
                "created":  created,
                "updated":  updated,
            });
            merged.insert(url, entry);
            brain_count += 1;
        }
    }

    // Blog entries: map source_id → canonical /blog/<slug> URL
    let mut blog_count = 0usize;
    if let Some(obj) = blog_idx.as_object() {
        for (key, val) in obj {
            let Some(url) = url_map.get(key) else {
                continue;
            };
            let meta = meta_map.get(key);
            let created = meta
                .and_then(|m| m.created.clone())
                .unwrap_or_else(|| {
                    val["lastmodified"].as_str().map(truncate_to_date).unwrap_or_default()
                });
            let updated = meta
                .and_then(|m| m.updated.clone())
                .unwrap_or_else(|| created.clone());
            let cats: Vec<Value> = meta
                .map(|m| m.categories.iter().map(|c| Value::String(c.clone())).collect())
                .unwrap_or_default();

            let entry = json!({
                "title":    val["title"].as_str().unwrap_or(""),
                "content":  val["content"].as_str().unwrap_or(""),
                "source":   "blog",
                "tags":     val["tags"].as_array().cloned().unwrap_or_default(),
                "category": cats,
                "created":  created,
                "updated":  updated,
            });
            merged.insert(url.clone(), entry);
            blog_count += 1;
        }
    }

    let merged_val = Value::Object(merged);
    save_json_pretty(OUT_BRAIN, &merged_val)?;
    fs::copy(OUT_BRAIN, OUT_BLOG)?;

    println!(
        "merge-search-index: brain={} blog={} total={}",
        brain_count,
        blog_count,
        brain_count + blog_count
    );
    Ok(())
}
