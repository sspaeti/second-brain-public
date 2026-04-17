use std::fs;
use std::path::{Path, PathBuf};
use std::io::prelude::*;
use std::io::{BufReader, Write};
use chrono::{DateTime, Utc};

use regex::Regex;
use std::fs::copy;
use std::collections::HashMap;

use serde_yaml::Value;

use crate::svg_generator::{ImageConfig, generate_og_image, extract_title_from_md};

// Constant for emojis to exclude from tags
pub const EXCLUDED_TAG_EMOJIS: [char; 6] = ['🗃', '🌻', '🗺', '🌍', '📬', '📚'];

/// Extracts a description from the markdown content (first paragraph after frontmatter)
/// Cleans wikilinks and markdown formatting, limits to ~150 characters
fn extract_description(lines: &[String], frontmatter_end: usize) -> Option<String> {
    let wikilink_re = Regex::new(r"\[\[([^\]|]+)(?:\|([^\]]+))?\]\]").unwrap();
    let bold_italic_re = Regex::new(r"(\*\*|__|\*|_|`|~~)").unwrap();
    let heading_re = Regex::new(r"^#+\s+").unwrap();
    let callout_re = Regex::new(r"^>\s*\[!\w+\]").unwrap();

    let mut description = String::new();
    let max_chars = 180;  // 3 lines × 60 chars with proper margins

    // Find first non-empty paragraph after frontmatter
    for (index, line) in lines.iter().enumerate() {
        // Skip frontmatter lines (frontmatter_end is 1-based line number of closing ---)
        if index < frontmatter_end {
            continue;
        }

        let trimmed = line.trim();

        // Skip empty lines, headings, callouts, blockquotes, images, horizontal rules
        if trimmed.is_empty()
            || heading_re.is_match(trimmed)
            || callout_re.is_match(trimmed)
            || trimmed.starts_with(">")  // Skip all blockquote/callout lines
            || trimmed.starts_with("!")
            || trimmed.starts_with("![[")
            || trimmed.starts_with("---")
            || trimmed.starts_with("Created:")
            || trimmed.starts_with("Origin:")
            || trimmed.starts_with("References:")
            || trimmed.starts_with("Tags:")
        {
            continue;
        }

        // Add line to description
        if !description.is_empty() {
            description.push(' ');
        }
        description.push_str(trimmed);

        // Stop if we have enough text or reached end of paragraph
        if description.len() >= max_chars {
            break;
        }

        // Check if next line is empty (end of paragraph)
        if let Some(next_line) = lines.get(index + 1) {
            if next_line.trim().is_empty() {
                break;
            }
        }
    }

    if description.is_empty() {
        return None;
    }

    // Clean markdown links: [Text](URL) → Text
    let markdown_link_re = Regex::new(r"\[([^\]]+)\]\([^\)]+\)").unwrap();
    description = markdown_link_re.replace_all(&description, "$1").to_string();

    // Clean wikilinks: [[Link]] → Link, [[Link|Text]] → Text
    description = wikilink_re.replace_all(&description, |caps: &regex::Captures| {
        if let Some(text) = caps.get(2) {
            text.as_str().to_string()
        } else {
            caps.get(1).unwrap().as_str().to_string()
        }
    }).to_string();

    // Remove bold, italic, strikethrough, code formatting
    description = bold_italic_re.replace_all(&description, "").to_string();

    // Remove list markers and blockquote markers
    let list_marker_re = Regex::new(r"(?:^|\s)[-*+>]\s+").unwrap();
    description = list_marker_re.replace_all(&description, " ").to_string();

    // Remove numbered list markers (1., 2., etc.)
    let numbered_list_re = Regex::new(r"(?:^|\s)\d+\.\s+").unwrap();
    description = numbered_list_re.replace_all(&description, " ").to_string();

    // Collapse multiple spaces to single space
    let multi_space_re = Regex::new(r"\s+").unwrap();
    description = multi_space_re.replace_all(&description, " ").to_string();

    // Limit to max_chars, preferring complete sentences
    if description.len() > max_chars {
        // Find a safe UTF-8 boundary
        let mut truncate_at = max_chars;
        while truncate_at > 0 && !description.is_char_boundary(truncate_at) {
            truncate_at -= 1;
        }

        // Try to find the last sentence ending (., !, ?) within the limit
        let truncated = &description[..truncate_at];
        let sentence_endings = [". ", "! ", "? "];
        let mut last_sentence_end = None;

        for ending in &sentence_endings {
            if let Some(pos) = truncated.rfind(ending) {
                // Include the punctuation + space
                let end_pos = pos + ending.len();
                if last_sentence_end.is_none() || end_pos > last_sentence_end.unwrap() {
                    last_sentence_end = Some(end_pos);
                }
            }
        }

        // If we found a sentence ending, truncate there (no ellipsis needed)
        if let Some(end_pos) = last_sentence_end {
            description.truncate(end_pos);
            description = description.trim().to_string();
        } else {
            // No sentence ending found, truncate at word boundary with ellipsis
            description.truncate(truncate_at);
            if let Some(last_space) = description.rfind(' ') {
                description.truncate(last_space);
            }
            description = description.trim().to_string();
            description.push_str("...");
        }
    }

    Some(description.trim().to_string())
}

pub fn process_file(path: &Path, public_folder: &str, public_brain_image_path: &str, images_map: &HashMap<String, PathBuf>) -> std::io::Result<()> {

    const OG_WIDTH: u32 = 1200;
    const OG_HEIGHT: u32 = 630;

    // println!("Opening file: {}", path.display());
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = Vec::new();
    let mut title = String::new();
    let mut tags: Vec<String> = Vec::new();

    let mut found_title = false;
    let mut found_publish = false;

    let mut in_frontmatter = false;
    let mut line_end_frontmatter = 0;
    let mut existing_frontmatter: HashMap<String, Value> = HashMap::new();
    let mut frontmatter_string = String::new();
    let mut enabletoc_value = String::new(); // To store the existing enableToc value

    let re = Regex::new(r"\s*!\[\[(.*?(?:png|jpg|gif|webp|mp4))\]\](.*)").unwrap();
    let created_re = Regex::new(r"Created:?\s+\[\[.*?(\d{4}-\d{2}-\d{2}).*?\]\]").unwrap();

    // Ugly fix as enableToc not working: Check if the file name is _index.md right after obtaining the file name
    let file_name_only = path.file_name()
                    .and_then(|f| f.to_str())
                    .map(|s| s.to_lowercase())
                    .unwrap_or_else(|| String::new());

    // HashMap to store images to copy
    let mut images_to_copy: Vec<String> = Vec::new();
    let mut created_date: Option<String> = None;
    let mut created_date_line_index: Option<usize> = None;

    let mut line_number = 0;
    for line in reader.lines() {
        let line = line?;
        lines.push(line.clone());

        line_number += 1;
        
        // Check if we're inside the frontmatter
        if ( line == "---" && line_number == 1 ) || ( in_frontmatter && line == "---") {
            in_frontmatter = !in_frontmatter;

            // If there is an existing frontmatter, parse the frontmatter string
            if !in_frontmatter {
                line_end_frontmatter = line_number;

                match serde_yaml::from_str(&frontmatter_string) {
                    Ok(frontmatter) => { 
                        // existing_frontmatter = serde_yaml::from_str(&frontmatter_string).unwrap();
                        existing_frontmatter = frontmatter;
                        // println!("Existing frontmatter: {:?}", existing_frontmatter);

                        //if existing frontmatter, check if it contains #publish tag
                        if let Some(tags_values) = existing_frontmatter.get("tags").and_then(|v| v.as_sequence()) {
                            let contains_publish = tags_values.iter().any(|tag| {
                                if let Some(tag_str) = tag.as_str() {
                                    tag_str.contains("publish")
                                } else {
                                    false
                                }
                            });
                            if contains_publish {
                                found_publish = true;
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!("Failed to parse frontmatter: {}", err);
                        eprintln!("Frontmatter content was:\n{}", frontmatter_string);
                    }
                }
                if file_name_only == "_index.md" || file_name_only == "data engineering.md" || file_name_only == "data engineering toolkit.md" || file_name_only == "cv.md" || file_name_only == "newsletter.md" {
                    enabletoc_value = "false".to_string();
                }
                else if let Some(enabletoc) = existing_frontmatter.get("enableToc").and_then(|v| v.as_str()) {
                    enabletoc_value = enabletoc.to_string();
                } else {
                    enabletoc_value = "".to_string(); // Default to empty if not present
                }
                // break;
            }
        } else if in_frontmatter {
            // Collect lines in the frontmatter to a string
            frontmatter_string.push_str(&line);
            frontmatter_string.push('\n');
            // println!("Frontmatter string: {}", &line);
        }

        // Extract title from the first line starting with "#"
        if line.starts_with("#") && title.is_empty() {
            title = line[1..].trim().to_string();
            found_title = true;
            continue;
        }

        if line.contains("#publish") {
            found_publish = true;
        }

        // Look for tags line, extract tags, and remove it from the lines
        if line.starts_with("Tags:") {
            let tags_line = line[5..].trim();
            // Extract tags, but filter out emoji-containing tags right away
            tags = tags_line.split(' ')
                .map(|s| s.replace("#", "").to_string())
                .filter(|s| !EXCLUDED_TAG_EMOJIS.iter().any(|emoji| s.contains(*emoji)))
                .collect();
            lines.pop();
            continue;
        }
        
        // Search for images and store them in `images_to_copy`
        if let Some(mat) = re.captures(&line) {
            if mat.len() > 1 {
                let image_name = &mat[1];
                if images_map.contains_key(image_name) {
                    // println!("Found image: {}", image_name);
                    images_to_copy.push(image_name.to_string());
                    // images_to_copy.insert(image_name.to_string(), image_path.clone());

                }
                // else {
                //     println!("Image not found in map: {}", image_name);
                // }
            }
        }

        // Extract created date from "Created [[YYYY-MM-DD]]" pattern
        if created_date.is_none() {
            if let Some(mat) = created_re.captures(&line) {
                if mat.len() > 1 {
                    created_date = Some(mat[1].to_string());
                    created_date_line_index = Some(lines.len() - 1);
                    // println!("Found created date: {}", &mat[1]);
                }
            }
        }
    }
    
    // If we found a publish tag, process the file
    if found_publish {
        // Create content/_img/feature directory if it doesn't exist
        let feature_dir = Path::new("content/_img/feature/gen");
        if !feature_dir.exists() {
            fs::create_dir_all(feature_dir)?;
        }

        // Extract description for OG image and meta tags
        // Priority: 1) manual 'desc:' field, 2) auto-extract from first paragraph
        let description = existing_frontmatter
            .get("desc")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| extract_description(&lines, line_end_frontmatter));

        // Store description in frontmatter if we extracted it and it doesn't exist
        if let Some(ref desc) = description {
            if !existing_frontmatter.contains_key("description") {
                existing_frontmatter.insert("description".to_string(), Value::String(desc.clone()));
            }
        }

        // Only generate new SVG and update frontmatter if ogimage doesn't exist
        if !existing_frontmatter.contains_key("ogimage") {
            let file_stem = path.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase().replace(" ", "-"))  // Convert to lowercase and replace spaces
                .unwrap_or("default".to_string());

            let image_config = ImageConfig {
                title: title.clone(),
                description: description.clone(),
                width: OG_WIDTH,
                height: OG_HEIGHT,
                output_path: format!("content/_img/feature/gen/{}.svg", file_stem),
            };

            if let Err(e) = generate_og_image(&image_config) {
                eprintln!("Failed to generate OG image for {}: {}", path.display(), e);
            } else {
                // Only update frontmatter if we successfully generated a new image
                existing_frontmatter.insert("ogimage".to_string(),
                    Value::String(format!("gen/{}.webp", file_stem)));
                existing_frontmatter.insert("ogwidth".to_string(),
                    Value::Number(serde_yaml::Number::from(OG_WIDTH)));
                existing_frontmatter.insert("ogheight".to_string(),
                    Value::Number(serde_yaml::Number::from(OG_HEIGHT)));
            }
        }

        // println!("Found publish tag");
        // Copy images here
        for image_name in &images_to_copy {
            if let Some(image_path) = images_map.get(image_name) {
                let destination_path = format!("{}/{}", public_brain_image_path, image_name);
                // println!("Copying image to: {}", destination_path);
                if let Err(e) = copy(image_path, &destination_path) {
                    println!("Error while copying image: {} - {} -> {}", e, image_path.display(), destination_path);
                };
            }
        }
        
        // Read the last modified timestamp
        let metadata = fs::metadata(path)?;
        let last_modified: DateTime<Utc> = DateTime::from(metadata.modified()?);
        let mut last_modified_str = last_modified.format("%Y-%m-%d %H:%M:%S").to_string();
        let mut frontmatter = String::new();

        // Prepare tags for frontmatter
        let mut frontmatter_tags = String::new();
        let filtered_tags: Vec<String> = tags.iter()
            .filter(|tag| *tag != "publish" && !tag.is_empty())
            .cloned()
            .collect();
            
        for tag in filtered_tags.iter() {
            frontmatter_tags.push_str(&format!("- {}\n", tag));
        }

        // If enabletoc_value is not empty, update it in existing_frontmatter
        if !enabletoc_value.is_empty() {
            existing_frontmatter.insert("enableToc".to_string(), serde_yaml::Value::String(enabletoc_value.clone()));
        }

        if existing_frontmatter.is_empty() {
            // Create frontmatter
            // frontmatter = format!("---\nlastmod: '{}'\ntitle: \"{}\"\ntags:\n{}\n---\n", last_modified_str, title, frontmatter_tags);
            let mut frontmatter_parts = vec![
                format!("lastmod: '{}'", last_modified_str),
                format!("title: \"{}\"", title),
            ];
            if let Some(ref date) = created_date {
                frontmatter_parts.insert(0, format!("createddate: '{}'", date));
            }
            frontmatter = format!("---\n{}\n---\n", frontmatter_parts.join("\n"));
        }
        else {
            // Merge frontmatter
            let mut existing_frontmatter = existing_frontmatter.clone();

            title = existing_frontmatter.get("title").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or(&title).to_string();
            existing_frontmatter.remove("title");

            let enabletoc = existing_frontmatter.get("enableToc").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or("").to_string();
            if !enabletoc.is_empty() {
                existing_frontmatter.remove("enableToc");
            }

            last_modified_str = existing_frontmatter.get("lastmod").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or(&last_modified_str).to_string();
            existing_frontmatter.remove("lastmod");

            // Handle created date - check if it exists in frontmatter or use extracted one
            let date_str = existing_frontmatter.get("createddate")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or(created_date.clone());
            existing_frontmatter.remove("createddate");

            // Update existing frontmatter with new values
            existing_frontmatter.insert("title".to_string(), serde_yaml::Value::String(format!("\"{}\"", title)));
            existing_frontmatter.insert("lastmod".to_string(), serde_yaml::Value::String(last_modified_str.clone()));
            existing_frontmatter.insert("enableToc".to_string(), serde_yaml::Value::String(enabletoc.clone()));

            // Add created date if we have it
            if let Some(date) = date_str {
                existing_frontmatter.insert("createddate".to_string(), serde_yaml::Value::String(date));
            }
            
            // Handling tags
            let mut new_tags: Vec<serde_yaml::Value> = vec![];
            if !frontmatter_tags.is_empty() {
                new_tags = frontmatter_tags
                    .lines()
                    .map(|line| serde_yaml::Value::String(line.trim_start_matches("- ").to_string()))
                    .collect();
            }
            // Handle tags from the existing frontmatter - always remove old tags key
            existing_frontmatter.remove("tags");
            
            // Only proceed with tags if we have non-emoji, non-empty new tags
            let filtered_new_tags: Vec<serde_yaml::Value> = new_tags
                .into_iter()
                .filter(|v| {
                    if let serde_yaml::Value::String(s) = v {
                        !s.is_empty() && 
                        !EXCLUDED_TAG_EMOJIS.iter().any(|emoji| s.contains(*emoji)) &&
                        s != "publish"
                    } else {
                        false  // Only accept string tags
                    }
                })
                .collect();
            
            // Only insert tags key if we have actual content
            if !filtered_new_tags.is_empty() {
                let tag_count = filtered_new_tags.len();
                existing_frontmatter.insert("tags".to_string(), serde_yaml::Value::Sequence(filtered_new_tags));
                println!("Added {} non-empty tags", tag_count);
            }

            // Redundant check removed - we now handle empty tags earlier
            
            // Sorting and reconstructing frontmatter
            let mut frontmatter_items: Vec<(&String, &serde_yaml::Value)> = existing_frontmatter.iter().collect();
            frontmatter_items.sort_by(|a, b| a.0.cmp(b.0));

            let mut sorted_frontmatter = String::from("---\n");
            for (key, value) in frontmatter_items {
                // Generate the value string based on the type
                // Special handling for tags key
                if key == "tags" {
                    if let serde_yaml::Value::Sequence(seq) = value {
                        // Skip completely empty tag arrays
                        if seq.is_empty() {
                            continue;
                        }
                        
                        // Format non-empty tag arrays
                        sorted_frontmatter.push_str(&format!("{}: [{}]\n", key, 
                            seq.iter()
                               .filter_map(|v| {
                                   if let serde_yaml::Value::String(s) = v {
                                       if s.is_empty() { None } else { Some(s.clone()) }
                                   } else { None }
                               })
                               .collect::<Vec<_>>()
                               .join(", ")
                        ));
                        continue;
                    }
                }

                // Handle all other value types
                let value_str = match value {
                    serde_yaml::Value::String(s) => {
                        // Quote description field for proper YAML syntax highlighting
                        if key == "description" {
                            // Escape any double quotes in the string
                            let escaped = s.replace("\\", "\\\\").replace("\"", "\\\"");
                            format!("\"{}\"", escaped)
                        } else {
                            s.clone()
                        }
                    },
                    serde_yaml::Value::Sequence(seq) => {
                        seq.iter()
                            .filter_map(|v| if let serde_yaml::Value::String(s) = v { Some(s.clone()) } else { None })
                            .collect::<Vec<_>>()
                            .join(", ")
                    },
                    _ => serde_yaml::to_string(value).unwrap_or_default(),
                };

                sorted_frontmatter.push_str(&format!("{}: {}\n", key, value_str));
            }
            sorted_frontmatter.push_str("---\n");

            // Use sorted_frontmatter for writing to the file
            frontmatter = sorted_frontmatter;
            // Use sorted_frontmatter for writing to the file
            // frontmatter = format!("---\ntitle: \"{}\"\nlastmod: '{}'\nenableToc: \"{}\"\n{}\n---\n", title, last_modified_str, enabletoc, sorted_frontmatter);
            // println!("Merged frontmatter: {}", frontmatter);
        }
        // Build content string first
        let mut content = String::new();
        let mut is_first_heading = true; // Flag to identify the first heading
        let callout_re = Regex::new(r"^>\s*\[!\w+\]").unwrap();

        for (index, line) in lines.iter().enumerate() {
            // Skip lines that were part of the original frontmatter
            if index < line_end_frontmatter {
                continue;
            }

            // Skip the first H1 heading line
            if is_first_heading && line.starts_with("#") {
                is_first_heading = false;
                continue;
            }

            // Skip the "Created [[YYYY-MM-DD]]" line (now shown in page header)
            if Some(index) == created_date_line_index {
                continue;
            }

            // Skip empty "References:" line (e.g. "References: " or "References:")
            if line.starts_with("References:") && line["References:".len()..].trim().is_empty() {
                continue;
            }

            // Add line to content
            content.push_str(line);
            content.push('\n');

            // Insert blank blockquote line between callout header and content
            // so Goldmark produces separate <p> elements for title and body
            if callout_re.is_match(line) {
                if let Some(next_line) = lines.get(index + 1) {
                    let trimmed = next_line.trim_start();
                    if trimmed.starts_with('>') {
                        let after_gt = trimmed[1..].trim();
                        if !after_gt.is_empty() {
                            content.push_str(">\n");
                        }
                    }
                }
            }
        }

        // Inject BASE tables if present
        let vault_root = std::env::var("secondbrain")
            .map(|p| std::path::PathBuf::from(p))
            .unwrap_or_else(|_| path.parent().unwrap_or(Path::new(".")).to_path_buf());

        let processed_content = inject_base_tables_if_present(
            &content,
            path.parent(),
            &vault_root,
        ).unwrap_or_else(|e| {
            eprintln!("Failed to inject BASE tables: {}", e);
            content
        });

        // Writing to the file
        let file_name = path.file_name().unwrap().to_str().unwrap().to_lowercase();
        let dest_path = format!("{}/{}", public_folder, file_name);
        println!("Writing to file: {}", dest_path);
        let mut file = fs::File::create(&dest_path)?;
        file.write_all(frontmatter.as_bytes())?;
        file.write_all(processed_content.as_bytes())?;
    }
    Ok(())
}

/// Process a BASE file and generate a standalone markdown page for it
pub fn process_base_file(
    base_path: &Path,
    public_folder: &str,
    vault_root: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::base_parser::{parse_base_file, extract_filter_expressions};
    use crate::base_query::{query_notes, sort_notes};
    use crate::base_renderer::{render_table_view, render_cards_view, render_list_view};
    use std::fs;

    println!("Processing BASE file: {}", base_path.display());

    // Parse BASE file
    let base_file = parse_base_file(base_path)?;

    // Extract filter expressions
    let filter_expressions = if let Some(ref filters) = base_file.filters {
        extract_filter_expressions(filters)
    } else {
        vec![]
    };

    // Create temp BASES folder for copying source files
    let base_name = base_path.file_stem().unwrap().to_string_lossy().to_string();
    let temp_bases_dir = PathBuf::from(public_folder).join("BASES").join(&base_name);
    fs::create_dir_all(&temp_bases_dir)?;

    // Copy source files to temp BASES folder
    let base_dir = base_path.parent().unwrap_or(vault_root);
    copy_base_source_files(base_dir, vault_root, &filter_expressions, &temp_bases_dir)?;

    // Query matching notes from temp BASES folder
    // Use empty filter expressions since we're querying from a pre-filtered temp folder
    let notes = query_notes(&temp_bases_dir, &temp_bases_dir, &vec![])?;

    // Generate HTML for all views
    let mut html_content = String::new();

    // Add description if present
    if let Some(ref description) = base_file.description {
        html_content.push_str(description);
        html_content.push_str("\n\n");
    }

    for view in &base_file.views {
        // Only render table views for standalone BASE pages
        if view.view_type != "table" {
            continue;
        }

        let mut view_notes = notes.clone();

        // Sort notes
        sort_notes(&mut view_notes, &view.sort);

        // Render table view
        let view_html = render_table_view(&view_notes, view);

        html_content.push_str(&view_html);
        html_content.push_str("\n\n");
    }

    // Generate frontmatter
    let base_stem = base_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("base");

    let title = base_stem.replace("-", " ").replace("_", " ");
    let title_capitalized = title
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    // Create frontmatter with disabled backlinks and graph
    let frontmatter = format!(
        "---\ntitle: \"{}\"\nenableToc: false\nenableBacklinks: false\nenableGraph: false\n---\n\n",
        title_capitalized
    );

    // Write to file
    let output_filename = format!("{}.md", base_stem.to_lowercase().replace(" ", "-"));
    let dest_path = format!("{}/{}", public_folder, output_filename);

    println!("Writing BASE page to: {}", dest_path);

    let mut file = fs::File::create(&dest_path)?;
    file.write_all(frontmatter.as_bytes())?;
    file.write_all(html_content.as_bytes())?;

    // Clean up temp BASES folder and parent BASES directory
    if temp_bases_dir.exists() {
        fs::remove_dir_all(&temp_bases_dir)?;
    }

    // Remove parent BASES folder if it's empty
    let bases_parent = PathBuf::from(public_folder).join("BASES");
    if bases_parent.exists() {
        if let Ok(mut entries) = fs::read_dir(&bases_parent) {
            if entries.next().is_none() {
                // Directory is empty
                fs::remove_dir(&bases_parent)?;
            }
        }
    }

    Ok(())
}

/// Copy source files from vault to temp BASES folder for querying
fn copy_base_source_files(
    base_dir: &Path,
    vault_root: &Path,
    filter_expressions: &[String],
    temp_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::base_query::{extract_folder_from_filters, extract_extension_from_filters, extract_exclusion_paths};
    use std::fs;

    // Extract folder filter and exclusions
    let folder_filter = extract_folder_from_filters(filter_expressions);
    let ext_filter = extract_extension_from_filters(filter_expressions);
    let exclusions = extract_exclusion_paths(filter_expressions);

    // Determine source directory
    let source_dir = if let Some(folder) = folder_filter {
        vault_root.join(folder)
    } else {
        base_dir.to_path_buf()
    };

    if !source_dir.exists() {
        return Ok(());
    }

    // Recursively copy matching files, respecting exclusions
    copy_files_recursive(&source_dir, temp_dir, &ext_filter, vault_root, &exclusions)?;

    Ok(())
}

/// Recursively copy files from source to destination
fn copy_files_recursive(
    source: &Path,
    dest: &Path,
    ext_filter: &Option<String>,
    vault_root: &Path,
    exclusions: &[String],
) -> Result<usize, Box<dyn std::error::Error>> {
    use std::fs;
    let mut count = 0;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();

        // Check if path matches any exclusion pattern
        let path_str = path.to_string_lossy();
        let mut is_excluded = false;
        for exclusion in exclusions {
            let exclusion_full = vault_root.join(exclusion);
            if path_str.contains(&exclusion_full.to_string_lossy().to_string()) {
                is_excluded = true;
                break;
            }
        }

        if is_excluded {
            // Skip excluded paths (including all files in excluded directories)
            continue;
        }

        if path.is_dir() {
            // Recursively copy subdirectories (flatten structure)
            count += copy_files_recursive(&path, dest, ext_filter, vault_root, exclusions)?;
        } else if path.is_file() {
            // Check extension
            if let Some(required_ext) = ext_filter {
                if let Some(ext) = path.extension() {
                    if ext != required_ext.as_str() {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            // Skip meta files
            let file_name = path.file_name().unwrap().to_string_lossy().to_string();
            if file_name.ends_with(".base")
                || file_name == "Coffee Beans.md"
                || file_name == "Coffee Beans (dataview).md"
                || file_name == "Coffee Beans Recommendations.md" {
                continue;
            }

            // Copy file to dest (flatten - no subdirs)
            let dest_file = dest.join(&file_name);
            fs::copy(&path, &dest_file)?;
            count += 1;
        }
    }

    Ok(count)
}

/// Inject BASE table HTML if the content contains BASE file references
pub fn inject_base_tables_if_present(
    content: &str,
    source_dir: Option<&Path>,
    vault_root: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    use crate::base_parser::{parse_base_file, extract_filter_expressions};
    use crate::base_query::{query_notes, sort_notes};
    use crate::base_renderer::{render_table_view, render_cards_view, render_list_view};
    use regex::Regex;

    // Pattern to match [[filename.base]] or [[filename.base#ViewName]]
    let base_ref_re = Regex::new(r"\[\[([^\]]+\.base)(?:#([^\]]+))?\]\]").unwrap();

    let mut result = content.to_string();

    // Find all BASE file references
    let mut replacements = Vec::new();

    for cap in base_ref_re.captures_iter(content) {
        let base_file_name = &cap[1];
        let view_name = cap.get(2).map(|m| m.as_str());
        let full_match = cap.get(0).unwrap().as_str();

        // Resolve BASE file path
        let base_path = if let Some(dir) = source_dir {
            dir.join(base_file_name)
        } else {
            vault_root.join(base_file_name)
        };

        if !base_path.exists() {
            eprintln!("BASE file not found: {}", base_path.display());
            continue;
        }

        // Parse BASE file
        let base_file = match parse_base_file(&base_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Failed to parse BASE file {}: {}", base_path.display(), e);
                continue;
            }
        };

        // Extract filter expressions
        let filter_expressions = if let Some(ref filters) = base_file.filters {
            extract_filter_expressions(filters)
        } else {
            vec![]
        };

        // Query matching notes
        let base_dir = base_path.parent().unwrap_or(source_dir.unwrap_or(vault_root));
        let mut notes = match query_notes(base_dir, vault_root, &filter_expressions) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("Failed to query notes for BASE: {}", e);
                continue;
            }
        };

        // Render the appropriate view(s)
        let mut html = String::new();

        if let Some(view_name) = view_name {
            // Render specific view
            if let Some(view) = base_file.views.iter().find(|v| v.name == view_name) {
                // Sort notes
                sort_notes(&mut notes, &view.sort);

                // Render based on view type
                html = match view.view_type.as_str() {
                    "table" => render_table_view(&notes, view),
                    "cards" => render_cards_view(&notes, view),
                    "list" => render_list_view(&notes, view),
                    _ => format!("<p>Unsupported view type: {}</p>", view.view_type),
                };
            }
        } else {
            // Render all views (or just the first table view)
            for view in &base_file.views {
                let mut view_notes = notes.clone();

                // Sort notes
                sort_notes(&mut view_notes, &view.sort);

                // Render based on view type
                let view_html = match view.view_type.as_str() {
                    "table" => render_table_view(&view_notes, view),
                    "cards" => render_cards_view(&view_notes, view),
                    "list" => render_list_view(&view_notes, view),
                    _ => continue, // Skip unsupported views
                };

                html.push_str(&view_html);
                html.push_str("\n");

                // For now, only render the first table view to avoid clutter
                if view.view_type == "table" {
                    break;
                }
            }
        }

        replacements.push((full_match.to_string(), html));
    }

    // Apply all replacements
    for (pattern, replacement) in replacements {
        result = result.replace(&pattern, &replacement);
    }

    Ok(result)
}
