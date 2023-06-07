use std::fs;
use std::path::{Path, PathBuf};
use std::io::prelude::*;
use std::io::{BufReader, Write};
use chrono::{DateTime, Utc};

use regex::Regex;
use std::fs::copy;
use std::collections::HashMap;

pub fn process_file(path: &Path, public_folder: &str, public_brain_image_path: &str, images_map: &HashMap<String, PathBuf>) -> std::io::Result<()> {
    // println!("Opening file: {}", path.display());
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = Vec::new();
    let mut title = String::new();
    let mut found_title = false;
    let mut found_publish = false;
    let mut tags: Vec<String> = Vec::new();
    let re = Regex::new(r"\s*!\[\[(.*?)\]\](.*)").unwrap();

    // HashMap to store images to copy
    let mut images_to_copy: Vec<String> = Vec::new();

    for line in reader.lines() {
        let line = line?;
        lines.push(line.clone());

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
        // Read the last modified timestamp
        let metadata = fs::metadata(path)?;
        let last_modified: DateTime<Utc> = DateTime::from(metadata.modified()?);
        let last_modified_str = last_modified.format("%Y-%m-%d %H:%M:%S").to_string();
        
        // Prepare tags for frontmatter
        let mut frontmatter_tags = String::new();
        for tag in tags.iter() {
            if tag != "publish" {
                frontmatter_tags.push_str(&format!("- {}\n", tag));
            }
        }


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

        // Create frontmatter
        let frontmatter = format!("---\nlastmod: '{}'\ntitle: \"{}\"\ntags:\n{}\n---\n", last_modified_str, title, frontmatter_tags);
        
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