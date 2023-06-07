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
    println!("Opening file: {}", path.display());
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = Vec::new();
    let mut title = String::new();
    let mut tags: Vec<String> = Vec::new();

    let mut found_title = false;
    let mut found_publish = false;

    let mut in_frontmatter = false;
    let mut existing_frontmatter: HashMap<String, Value> = HashMap::new();
    let mut frontmatter_string = String::new();

    let re = Regex::new(r"\s*!\[\[(.*?)\]\](.*)").unwrap();

    // HashMap to store images to copy
    let mut images_to_copy: Vec<String> = Vec::new();
    
    let mut line_number = 0;
    for line in reader.lines() {
        let line = line?;
        lines.push(line.clone());

        line_number += 1;
        
        // Check if we're inside the frontmatter
        if line == "---" && line_number == 1 {
            println!("Found frontmatter");
            in_frontmatter = !in_frontmatter;
            
            // If we're exiting the frontmatter, parse the frontmatter string
            if !in_frontmatter {
                match serde_yaml::from_str(&frontmatter_string) {
                    Ok(frontmatter) => { 
                        // existing_frontmatter = serde_yaml::from_str(&frontmatter_string).unwrap();
                        existing_frontmatter = frontmatter;
                    }
                    Err(err) => {
                        eprintln!("Failed to parse frontmatter: {}", err);
                        eprintln!("Frontmatter content was:\n{}", frontmatter_string);
                    }
                }
                break;
            }
        } else if in_frontmatter {
            // Collect lines in the frontmatter to a string
            frontmatter_string.push_str(&line);
            frontmatter_string.push('\n');
        }


        // Extract title from the first line starting with "#"
        if line.starts_with("#") && title.is_empty() {
            title = line[1..].trim().to_string();
            found_title = true;
            continue;
        }

        // Check for the publish tag
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
        // println!("1 image: {}", &line);
        
        // Search for images and store them in `images_to_copy`
        if let Some(mat) = re.captures(&line) {
            if mat.len() > 1 {
                let image_name = &mat[1];
                if images_map.contains_key(image_name) {
                    println!("Found image: {}", image_name);
                    images_to_copy.push(image_name.to_string());
                    // images_to_copy.insert(image_name.to_string(), image_path.clone());

                } else {
                    println!("Image not found in map: {}", image_name);
                }
            }
        }
    }
    
    // If we found a publish tag, process the file
    if found_publish {
        // Copy images here
        for image_name in &images_to_copy {
            if let Some(image_path) = images_map.get(image_name) {
                let destination_path = format!("{}/{}", public_brain_image_path, image_name);
                println!("Copying image to: {}", destination_path);
                match copy(image_path, &destination_path) {
                    Ok(_) => println!("Successfully copied image."),
                    Err(e) => println!("Error while copying image: {} - {} -> {}", e, image_path.display(), destination_path),
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

        if existing_frontmatter.is_empty() {
            // Create frontmatter
            frontmatter = format!("---\nlastmod: '{}'\ntitle: \"{}\"\ntags:\n{}\n---\n", last_modified_str, title, frontmatter_tags);
        }
        else {
            // Merge frontmatter

            title = existing_frontmatter.get("title").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or(&title).to_string();
            let enabletoc = existing_frontmatter.get("enabletoc").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or("").to_string();
            last_modified_str = existing_frontmatter.get("lastmod").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or(&last_modified_str).to_string();

            if let Some(serde_yaml::Value::Sequence(seq)) = existing_frontmatter.get("tags") {
                let tags: Vec<String> = seq.iter().filter_map(|v| match v {
                    serde_yaml::Value::String(s) => Some(s.clone()),
                    _ => None,
                }).collect();
                if !tags.is_empty() {
                    frontmatter_tags = tags.iter().map(|tag| format!("- {}", tag)).collect::<Vec<String>>().join("\n");
                } // else keep the hashtags as tags from the note
            }

            frontmatter = format!("---\nlastmod: '{}'\ntitle: \"{}\"\n", last_modified_str, title);

            if enabletoc != "" {
                frontmatter.push_str(&format!("enableToc: {}\n", enabletoc));
            }

            frontmatter.push_str(&format!("tags:\n{}\n---\n", frontmatter_tags));
        }
        
        let dest_path = format!("{}/{}", public_folder, path.file_name().unwrap().to_str().unwrap());
        println!("Writing to file: {}", dest_path);
        let mut file = fs::File::create(&dest_path)?;
        file.write_all(frontmatter.as_bytes())?;
        
        for line in lines.iter() {
            if found_title {
                found_title = false;
                continue;
            }
            file.write_all(line.as_bytes())?;
            file.write_all(b"\n")?;
        }
    }
    Ok(())
}
