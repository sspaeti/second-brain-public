use std::fs;
use std::process::Command;
use std::error::Error;
use std::path::Path;
use regex::Regex;

pub struct ImageConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub output_path: String,
}

pub fn generate_og_image(config: &ImageConfig) -> Result<(), Box<dyn Error>> {
    // Check if WebP file already exists
    let output_path = config.output_path.replace(".svg", ".webp");
    if Path::new(&output_path).exists() {
        // println!("WebP file already exists: {}", output_path);
        return Ok(());
    }
    
    // Sanitize the file paths to handle special characters
    let safe_svg_path = config.output_path.clone();
    
    // Create the directory if it doesn't exist
    if let Some(parent) = Path::new(&safe_svg_path).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    
    let words: Vec<String> = split_title(&config.title);
    let svg = create_svg(&words, config.width, config.height)?;
    
    // First save the SVG to a temporary file
    fs::write(&safe_svg_path, svg)?;
    
    // Convert to WebP using modern ImageMagick command with improved error handling
    let output = Command::new("magick")
        .arg(&safe_svg_path)
        .arg(&output_path)
        .output();
    
    match output {
        Ok(output) => {
            if !output.status.success() {
                let error_message = String::from_utf8_lossy(&output.stderr);
                eprintln!("ImageMagick conversion error: {}", error_message);
                return Err(format!("ImageMagick conversion failed: {}", error_message).into());
            }
        },
        Err(e) => {
            eprintln!("Failed to run ImageMagick: {}", e);
            return Err(format!("Failed to run ImageMagick: {}", e).into());
        }
    }
    
    // Optionally remove the temporary SVG file
    let _ = fs::remove_file(safe_svg_path); // Use let _ to ignore errors on cleanup
    
    Ok(())
}

fn split_title(title: &str) -> Vec<String> {
    // Clean the title by replacing problematic characters
    let clean_title = title
        .replace('"', "")
        .replace('&', "and")  // Replace & with "and" to avoid XML parsing issues
        .replace('<', "")     // Remove XML special chars
        .replace('>', "");

    let words: Vec<String> = clean_title
        .split(|c: char| c.is_whitespace() || c == ':')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    // Group words to create a balanced layout for the new landscape design
    // Target length is shorter now since we have a landscape format
    let mut grouped_words = Vec::new();
    let mut current_line = String::new();
    let target_length = 25;  // Increased from 20 for wider format

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

fn create_svg(words: &[String], _width: u32, _height: u32) -> Result<String, Box<dyn Error>> {
    // New landscape format: 1350 x 1080 (5:4 aspect ratio)
    // Load the template file - use path relative to the repo root
    // First try from current working directory, then from utils/obsidian-quartz/src
    let template_path = if Path::new("src/og_template.svg").exists() {
        Path::new("src/og_template.svg")
    } else if Path::new("utils/obsidian-quartz/src/og_template.svg").exists() {
        Path::new("utils/obsidian-quartz/src/og_template.svg")
    } else {
        // Fallback: try to find it relative to the binary location
        return Err("Template file not found. Please ensure og_template.svg is in src/ or utils/obsidian-quartz/src/".into());
    };

    let template = fs::read_to_string(template_path)
        .map_err(|e| format!("Failed to read template file from {:?}: {}", template_path, e))?;

    // Generate title text elements
    let mut title_text = String::new();
    let mut y_position = 600.0;
    let line_height = 120.0;
    let words_count = words.len() as f32;

    // Dynamically adjust font size based on title length
    // Landscape format allows for larger text
    let title_font_size = if words_count > 3.0 {
        78.0
    } else if words_count > 2.0 {
        88.0
    } else {
        98.0
    };

    for word in words.iter() {
        // Properly escape XML special characters for SVG
        let escaped_word = word
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;");

        title_text.push_str(&format!(
            "    <text x=\"100\" y=\"{}\" fill=\"#DCD7BA\" font-family=\"Arial, sans-serif\" font-size=\"{}\" font-weight=\"900\" letter-spacing=\"-2\">{}</text>\n",
            y_position, title_font_size, escaped_word
        ));
        y_position += line_height;
    }

    // Replace placeholder in template with actual title
    let svg = template.replace("{{TITLE_PLACEHOLDER}}", &title_text);

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
