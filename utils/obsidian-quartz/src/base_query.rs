use regex::Regex;
use serde_yaml::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Represents a note with its properties
#[derive(Debug, Clone)]
pub struct Note {
    pub file_name: String,
    pub file_path: PathBuf,
    pub properties: HashMap<String, String>,
}

/// Query notes from a directory based on filter expressions
pub fn query_notes(
    base_dir: &Path,
    vault_root: &Path,
    filter_expressions: &[String],
) -> Result<Vec<Note>, Box<dyn std::error::Error>> {
    let mut notes = Vec::new();

    // Parse filters to extract folder path and file extension requirements
    let folder_filter = extract_folder_from_filters(filter_expressions);
    let ext_filter = extract_extension_from_filters(filter_expressions);

    // Determine search directory
    let search_dir = if let Some(folder) = folder_filter {
        // Try to find the folder in vault
        vault_root.join(folder)
    } else {
        // Use base_dir if no folder specified
        base_dir.to_path_buf()
    };

    if !search_dir.exists() {
        return Ok(notes);
    }

    // Recursively scan directory for matching files
    scan_directory_recursive(&search_dir, &ext_filter, &mut notes)?;

    Ok(notes)
}

/// Recursively scan a directory for markdown files
fn scan_directory_recursive(
    dir: &Path,
    ext_filter: &Option<String>,
    notes: &mut Vec<Note>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Recursively scan subdirectories
            scan_directory_recursive(&path, ext_filter, notes)?;
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

            // Skip the BASE file itself and any index files
            let file_name = path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();

            if file_name.ends_with(".base")
                || file_name == "Coffee Beans.md"
                || file_name == "Coffee Beans (dataview).md"
                || file_name == "Coffee Beans Recommendations.md" {
                continue;
            }

            // Extract properties from the file
            if let Ok(properties) = extract_properties_from_file(&path) {
                notes.push(Note {
                    file_name: path
                        .file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                    file_path: path,
                    properties,
                });
            }
        }
    }

    Ok(())
}

/// Extract folder path from filter expressions
pub fn extract_folder_from_filters(filters: &[String]) -> Option<String> {
    let folder_re = Regex::new(r#"file\.path\.contains\("([^"]+)"\)"#).unwrap();

    for filter in filters {
        // Skip negation filters
        if filter.starts_with('!') {
            continue;
        }

        if let Some(caps) = folder_re.captures(filter) {
            if let Some(folder) = caps.get(1) {
                return Some(folder.as_str().to_string());
            }
        }
    }

    None
}

/// Extract exclusion paths from filter expressions (paths to exclude)
pub fn extract_exclusion_paths(filters: &[String]) -> Vec<String> {
    let exclusion_re = Regex::new(r#"!file\.path\.contains\("([^"]+)"\)"#).unwrap();
    let mut exclusions = Vec::new();

    for filter in filters {
        if let Some(caps) = exclusion_re.captures(filter) {
            if let Some(path) = caps.get(1) {
                exclusions.push(path.as_str().to_string());
            }
        }
    }

    exclusions
}

/// Extract file extension requirement from filter expressions
pub fn extract_extension_from_filters(filters: &[String]) -> Option<String> {
    let ext_re = Regex::new(r#"file\.ext\.contains\("([^"]+)"\)"#).unwrap();

    for filter in filters {
        if let Some(caps) = ext_re.captures(filter) {
            if let Some(ext) = caps.get(1) {
                return Some(ext.as_str().to_string());
            }
        }
    }

    None
}

/// Extract properties from a markdown file's frontmatter
fn extract_properties_from_file(path: &Path) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let mut properties = HashMap::new();

    // Check if file has frontmatter
    if !content.starts_with("---") {
        return Ok(properties);
    }

    // Find the end of frontmatter
    let lines: Vec<&str> = content.lines().collect();
    let mut frontmatter_end = 0;

    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            frontmatter_end = i;
            break;
        }
    }

    if frontmatter_end == 0 {
        return Ok(properties);
    }

    // Extract frontmatter YAML
    let frontmatter_str = lines[1..frontmatter_end].join("\n");

    // Parse YAML
    if let Ok(yaml_value) = serde_yaml::from_str::<Value>(&frontmatter_str) {
        if let Some(mapping) = yaml_value.as_mapping() {
            for (key, value) in mapping {
                if let Some(key_str) = key.as_str() {
                    let value_str = yaml_value_to_string(value);
                    properties.insert(key_str.to_string(), value_str);
                }
            }
        }
    }

    Ok(properties)
}

/// Convert YAML value to display string
fn yaml_value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Sequence(seq) => {
            // Join array elements with comma
            seq.iter()
                .map(yaml_value_to_string)
                .collect::<Vec<_>>()
                .join(", ")
        }
        Value::Mapping(_) => "[object]".to_string(),
        Value::Null => String::new(),
        _ => String::new(),
    }
}

/// Sort notes based on sort configuration
pub fn sort_notes(
    notes: &mut Vec<Note>,
    sort_configs: &[crate::base_parser::SortConfig],
) {
    notes.sort_by(|a, b| {
        for config in sort_configs {
            let a_val = get_sort_value(a, &config.property);
            let b_val = get_sort_value(b, &config.property);

            let cmp = if config.direction.to_uppercase() == "DESC" {
                b_val.cmp(&a_val)
            } else {
                a_val.cmp(&b_val)
            };

            if cmp != std::cmp::Ordering::Equal {
                return cmp;
            }
        }
        std::cmp::Ordering::Equal
    });
}

/// Get sortable value from note
fn get_sort_value(note: &Note, property: &str) -> String {
    if property == "file.name" {
        note.file_name.clone()
    } else if property == "file.mtime" {
        // For mtime, we'd need to check file metadata
        // For now, return empty string
        String::new()
    } else {
        note.properties.get(property).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_folder_from_filters() {
        let filters = vec![
            r#"file.path.contains("Coffee Beans")"#.to_string(),
            r#"file.ext.contains("md")"#.to_string(),
        ];

        let folder = extract_folder_from_filters(&filters);
        assert_eq!(folder, Some("Coffee Beans".to_string()));
    }

    #[test]
    fn test_extract_extension_from_filters() {
        let filters = vec![
            r#"file.path.contains("Coffee Beans")"#.to_string(),
            r#"file.ext.contains("md")"#.to_string(),
        ];

        let ext = extract_extension_from_filters(&filters);
        assert_eq!(ext, Some("md".to_string()));
    }
}
