use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::error::Error;

mod file_utils;
use file_utils::process_file;

mod handle_link_index;
use handle_link_index::convert_to_lower_case;

mod enrich_with_blog;
mod enrich_with_book;
mod merge_search_index;

mod svg_generator;
use svg_generator::{ImageConfig, generate_og_image, extract_title_from_md};

mod base_parser;
mod base_query;
mod base_renderer;


fn main() -> Result<(), Box<dyn Error>> {
   let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 && args[1] == "convert_to_lower_case" {
        //handle hugo linkindexes
        println!("Handling link indexes");
        let link_index_path = Path::new("assets/indices/linkIndex.json");
        convert_to_lower_case(link_index_path)?;
        println!("Handling link indexes: DONE");
    } else if args.len() > 1 && args[1] == "enrich-with-blog" {
        enrich_with_blog::run()?;
    } else if args.len() > 1 && args[1] == "enrich-with-book" {
        enrich_with_book::run()?;
    } else if args.len() > 1 && args[1] == "merge-search-index" {
        merge_search_index::run()?;
    } else {
        let second_brain_path = env::var("secondbrain")?;
        let public_folder_path_copy = env::var("public_secondbrain")?;
        let public_brain_image_path = env::var("public_secondbrain")?;

        let mut images_map: HashMap<String, PathBuf> = HashMap::new();
        build_images_map(Path::new(&second_brain_path), &mut images_map)?;

        match visit_dirs(Path::new(&second_brain_path), &public_folder_path_copy, &public_brain_image_path, &images_map, &second_brain_path) {
            Ok(_) => (),
            Err(e) => println!("An error occurred: {}", e),
        }
    }

    Ok(())
}

fn visit_dirs(dir: &Path, public_folder: &str, public_brain_image_path: &str, images_map: &HashMap<String, PathBuf>, vault_root: &str) -> std::io::Result<()> {
    // println!("Visiting directory: {}", dir.display());

    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // ignore symlinked folder that could contain #publish that breaks the build
                if path.ends_with("Book") || path.ends_with("Blog") {
                    println!("Visiting directory: {}", path.display());
                    continue;
                }

                visit_dirs(&path, public_folder, public_brain_image_path, images_map, vault_root)?;
            } else {
                ////DEBUG:
                ////skip if file is not "Folder Structure PARA.md"
                //if let Some(file_name) = path.file_name() {
                //    if file_name != "Data Orchestrators.md" {
                //        continue;
                //    }
                //}


                if let Some(extension) = path.extension() {
                    if extension == "md" {
                        process_file(&path, public_folder, public_brain_image_path, images_map)?;
                    } else if extension == "base" {
                        // Check if BASE file has publish: true property
                        use crate::base_parser::parse_base_file;

                        if let Ok(base_file) = parse_base_file(&path) {
                            if base_file.publish {
                                // Query from vault_root to get all matching files (published or not)
                                // The BASE table is a self-contained view that doesn't require
                                // individual files to be published
                                if let Err(e) = file_utils::process_base_file(&path, public_folder, &PathBuf::from(vault_root)) {
                                    eprintln!("Error processing BASE file: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn build_images_map(dir: &Path, images_map: &mut HashMap<String, PathBuf>) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                build_images_map(&path, images_map)?;
            } else {
                if let Some(extension) = path.extension() {
                    if extension == "png" || extension == "jpg" || extension == "gif" || extension == "webp" {
                        if let Some(file_name) = path.file_name() {
                            images_map.insert(file_name.to_str().unwrap().to_string(), path.clone());
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

