use std::fs;
use std::path::Path;
use std::error::Error;
use regex::Regex;

pub struct ImageConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub output_path: String,
}

pub fn generate_og_image(config: &ImageConfig) -> Result<(), Box<dyn Error>> {
    let words: Vec<String> = split_title(&config.title);
    let svg = create_svg(&words, config.width, config.height)?;
    fs::write(&config.output_path, svg)?;
    Ok(())
}

fn split_title(title: &str) -> Vec<String> {
    let clean_title = title.replace('"', "");
    let words: Vec<String> = clean_title
        .split(|c: char| c.is_whitespace() || c == ':')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    
    // Group words to create a balanced layout
    let mut grouped_words = Vec::new();
    let mut current_line = String::new();
    let target_length = 20;

    for word in words {
        if current_line.len() + word.len() + 1 <= target_length {
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(&word);
        } else {
            if !current_line.is_empty() {
                grouped_words.push(current_line);
            }
            current_line = word;
        }
    }
    if !current_line.is_empty() {
        grouped_words.push(current_line);
    }

    grouped_words
}

fn create_svg(words: &[String], width: u32, height: u32) -> Result<String, Box<dyn Error>> {
    let background_color = "#000000";
    let text_color = "#FFFFFF";
    let accent_color = "#D4AF37"; // Gold color from your current theme
    
    let mut svg = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}">
    <rect width="100%" height="100%" fill="{background_color}"/>
    <rect x="40" y="40" width="calc(100% - 80)" height="calc(100% - 80)" 
          stroke="{accent_color}" stroke-width="2" fill="none"/>
    
    <!-- Header -->
    <text x="60" y="100" fill="{accent_color}" font-family="Arial" font-size="24" font-weight="bold">
        SECOND BRAIN
    </text>
"#
    );

    // Add main title words
    let mut y_position = 180.0;
    let line_height = 60.0;
    let max_font_size = 48.0;
    let min_font_size = 36.0;

    for (i, word) in words.iter().enumerate() {
        let font_size = if words.len() > 3 {
            min_font_size
        } else {
            max_font_size
        };

        svg.push_str(&format!(
            r#"    <text x="60" y="{}" fill="{text_color}" font-family="Arial" font-size="{}" font-weight="bold">{}</text>
"#,
            y_position, font_size, word
        ));
        y_position += line_height;
    }

    // Add footer
    svg.push_str(&format!(
        r#"    <text x="60" y="{}" fill="{accent_color}" font-family="Arial" font-size="20">
        A Digital Vault of Knowledge
    </text>
</svg>"#,
        height - 60
    ));

    Ok(svg)
}

// Function to extract title from frontmatter
pub fn extract_title_from_md(file_path: &Path) -> Result<String, Box<dyn Error>> {
    let content = fs::read_to_string(file_path)?;
    let re = Regex::new(r#"title:\s*"([^"]*)""#)?;
    
    if let Some(captures) = re.captures(&content) {
        if let Some(title) = captures.get(1) {
            return Ok(title.as_str().to_string());
        }
    }
    
    Err("No title found in frontmatter".into())
}
