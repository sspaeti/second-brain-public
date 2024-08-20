use std::fs;
use std::path::{Path, PathBuf};
use std::io::prelude::*;
use std::io::{BufReader, Write};
use chrono::{DateTime, Utc};

use regex::Regex;
use std::fs::copy;
use std::collections::HashMap;

use serde_yaml::Value;

pub fn process_file(path: &Path, public_folder: &str, public_brain_image_path: &str, images_map: &HashMap<String, PathBuf>) -> std::io::Result<()> {
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

    let re = Regex::new(r"\s*!\[\[(.*?(?:png|jpg|gif|webp))\]\](.*)").unwrap();
    
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
                if file_name_only == "_index.md" || file_name_only == "data engineering.md" || file_name_only == "data-engineering.md" {
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
            tags = tags_line.split(' ').map(|s| s.replace("#", "").to_string()).collect();
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
        for tag in tags.iter() {
            if tag != "publish" {
                frontmatter_tags.push_str(&format!("- {}\n", tag));
            }
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
            ////ignore writing tags as for now, as handling emojis (I guess) does not work properly
            ///with latest version.
            // if let Some(serde_yaml::Value::Sequence(existing_tags)) = existing_frontmatter.get_mut("tags") {
            //     for tag in new_tags {
            //         if !existing_tags.contains(&tag) {
            //             existing_tags.push(tag);
            //         }
            //     }
            // } else {
            //     existing_frontmatter.insert("tags".to_string(), serde_yaml::Value::Sequence(new_tags));
            // }

            // Sorting and reconstructing frontmatter
            let mut frontmatter_items: Vec<(&String, &serde_yaml::Value)> = existing_frontmatter.iter().collect();
            frontmatter_items.sort_by(|a, b| a.0.cmp(b.0));

            let mut sorted_frontmatter = String::from("---\n");
            for (key, value) in frontmatter_items {
                let value_str = match value {
                    serde_yaml::Value::String(s) => s.clone(),
                    serde_yaml::Value::Sequence(seq) => seq.iter()
                        .filter_map(|v| if let serde_yaml::Value::String(s) = v { Some(s.clone()) } else { None })
                        .collect::<Vec<_>>()
                        .join(", "),
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
