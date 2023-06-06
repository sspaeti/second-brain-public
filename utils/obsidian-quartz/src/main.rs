use std::env;
use std::fs;
use std::path::Path;
use std::io::prelude::*;
use std::io::{BufReader, Write};
use std::os::unix::fs::MetadataExt;
use chrono::{DateTime, Utc};

fn main() {
    let second_brain_path = env::var("secondbrain").expect("Set the SECOND_BRAIN_PATH variable");
    let public_folder_path_copy = env::var("public_secondbrain").expect("Set the PUBLIC_FOLDER_PATH_COPY variable");

    visit_dirs(Path::new(&second_brain_path), &public_folder_path_copy).unwrap();
}

fn visit_dirs(dir: &Path, public_folder: &str) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, public_folder)?;
            } else {
                if let Some(extension) = path.extension() {
                    if extension == "md" {
                        process_file(&path, public_folder)?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn process_file(path: &Path, public_folder: &str) -> std::io::Result<()> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = Vec::new();
    let mut title = String::new();
    let mut found_title = false;
    let mut found_publish = false;

    for line in reader.lines() {
        let line = line?;
        lines.push(line.clone());
        if line.contains("#publish") {
            if title.is_empty() {
                title = String::from("Untitled");
            }
            found_publish = true;
            break;
        }

        if line.starts_with("#") && title.is_empty() {
            title = line[1..].trim().to_string();
            found_title = true;
        }
    }
    
    if found_publish {
        // Read the last modified timestamp
        let metadata = fs::metadata(path)?;
        let last_modified: DateTime<Utc> = DateTime::from(metadata.modified()?);
        let last_modified_str = last_modified.format("%Y-%m-%d %H:%M:%S").to_string();
        
        // Create frontmatter
        let frontmatter = format!("---\nlastmod: '{}'\ntitle: {}\n---\n", last_modified_str, title);
        
        let dest_path = format!("{}/{}", public_folder, path.file_name().unwrap().to_str().unwrap());
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
