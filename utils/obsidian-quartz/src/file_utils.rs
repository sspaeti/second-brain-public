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
pub const EXCLUDED_TAG_EMOJIS: [char; 5] = ['🗃', '🌻', '🗺', '🌍', '📬'];

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
    
    // Ugly fix as enableToc not working: Check if the file name is _index.md right after obtaining the file name
    let file_name_only = path.file_name()
                    .and_then(|f| f.to_str())
                    .map(|s| s.to_lowercase())
                    .unwrap_or_else(|| String::new());

    // HashMap to store images to copy
    let mut images_to_copy: Vec<String> = Vec::new();
    
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
    }
    
    // If we found a publish tag, process the file
    if found_publish {
        // Create content/_img/feature directory if it doesn't exist
        let feature_dir = Path::new("content/_img/feature/gen");
        if !feature_dir.exists() {
            fs::create_dir_all(feature_dir)?;
        }

        // Only generate new SVG and update frontmatter if ogimage doesn't exist
        if !existing_frontmatter.contains_key("ogimage") {
            let file_stem = path.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase().replace(" ", "-"))  // Convert to lowercase and replace spaces
                .unwrap_or("default".to_string());
            
            let image_config = ImageConfig {
                title: title.clone(),
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
            frontmatter = format!("---\nlastmod: '{}'\ntitle: \"{}\"\n---\n", last_modified_str, title);
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

            // Update existing frontmatter with new values
            existing_frontmatter.insert("title".to_string(), serde_yaml::Value::String(format!("\"{}\"", title)));
            existing_frontmatter.insert("lastmod".to_string(), serde_yaml::Value::String(last_modified_str.clone()));
            existing_frontmatter.insert("enableToc".to_string(), serde_yaml::Value::String(enabletoc.clone()));
            
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
                    serde_yaml::Value::String(s) => s.clone(),
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
        // Writing to the file
        let file_name = path.file_name().unwrap().to_str().unwrap().to_lowercase();
        let dest_path = format!("{}/{}", public_folder, file_name);
        println!("Writing to file: {}", dest_path);
        let mut file = fs::File::create(&dest_path)?;
        file.write_all(frontmatter.as_bytes())?;

        let mut is_first_heading = true; // Flag to identify the first heading
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

            // Write the line to the file
            file.write_all(line.as_bytes())?;
            file.write_all(b"\n")?;
        }
    }
    Ok(())
}
