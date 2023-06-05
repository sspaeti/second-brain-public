extern crate glob;
extern crate chrono;
extern crate pandoc;
extern crate regex;
extern create yaml-front-matter;

use std::env;
use std::fs;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::path::Path;
use glob::glob;

use std::collections::BTreeMap;
use regex::Regex;
use chrono::DateTime;


fn main() {
    let second_brain_path = env::var("secondbrain").expect("SecondBrain path not found");
    let second_brain_public = env::var("public_secondbrain").expect("Public SecondBrain path not found");
    let public_brain_image_path = format!("{}/images", &second_brain_public);

    // loop through public files and add referenced images, fix h1 headers
    for entry in glob(&format!("{}/**/*.md", second_brain_public)).unwrap() {
        if let Ok(file_path) = entry {
            list_images_from_markdown(&file_path, &second_brain_path, &second_brain_public);
        }
    }

    find_hashtag(&second_brain_path, &second_brain_public);
}

fn find_hashtag(second_brain_path: &str, second_brain_public: &str) {
    for entry in glob(&format!("{}/**/*.md", second_brain_path)).unwrap() {
        if let Ok(file_path) = entry {
            let f = fs::File::open(&file_path).unwrap();
            let reader = BufReader::new(f);
            let lines: Vec<String> = reader.lines().collect::<Result<_, _>>().unwrap();
            for line in lines.iter().rev() {
                if line.contains("#publish") {
                    let destination_file_path = format!("{}/{}", second_brain_public, file_path.file_name().unwrap().to_str().unwrap().to_lowercase());
                    fs::copy(&file_path, &destination_file_path).unwrap();

                    let last_modified = fs::metadata(&file_path).unwrap().modified().unwrap();
                    // convert last_modified to datetime 
                    let datetime = DateTime::from(last_modified);

                    let timestamp = datetime.format("%Y-%m-%d %H:%M:%S").to_string();

                    add_h1_as_title_frontmatter(&destination_file_path, &timestamp);
                    break;
                }
            }
        }
    }
}

fn add_h1_as_title_frontmatter(file_path: &str, last_modified: &str) {
    let f = fs::File::open(&file_path).unwrap();
    let reader = BufReader::new(f);
    let lines: Vec<String> = reader.lines().collect::<Result<_, _>>().unwrap();
    let mut headers = Vec::new();

    for line in &lines {
        if line.starts_with("# ") {
            headers.push(line.trim_start_matches("# ").to_string());
        }
    }

    let mut new_lines = lines.into_iter().filter(|line| !line.starts_with("# ")).collect::<Vec<String>>();

    let mut fm = BTreeMap::new();
    if headers.len() > 0 {
        fm.insert("title".to_string(), headers[0].clone());
    }
    fm.insert("lastmod".to_string(), last_modified.to_string());

    let fm_str = serde_yaml::to_string(&fm).unwrap();
    new_lines.insert(0, format!("---\n{}\n---", fm_str));

    let mut file = fs::File::create(&file_path).unwrap();
    file.write_all(new_lines.join("\n").as_bytes()).unwrap();
}
fn list_images_from_markdown(file_path: &Path, second_brain_path: &str, second_brain_public: &str) {
    let contents = fs::read_to_string(file_path).unwrap();
    let re = Regex::new(r"!\[\[(.*?)\]\](.*)\s").unwrap();
    let images = re.captures_iter(&contents);

    for image in images {
        let image_name = &image[1];
        if !image_name.is_empty() {
            find_image_and_copy(image_name, second_brain_path, second_brain_public);
        }
    }
}

fn find_image_and_copy(image_name: &str, root_path: &str, second_brain_public: &str) {
    let pattern = format!("{}/{}", root_path, image_name);
    for entry in glob(&pattern).unwrap() {
        if let Ok(file_path) = entry {
            let destination_path = format!("{}/images/{}", second_brain_public, file_path.file_name().unwrap().to_str().unwrap());
            fs::copy(file_path, Path::new(&destination_path)).expect("Failed to copy image");
        }
    }
}
