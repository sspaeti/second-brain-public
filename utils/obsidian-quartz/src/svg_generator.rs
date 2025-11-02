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

    // Group words to create a balanced layout for Design 4 split-screen
    // We need to fit on the right side with max 4 lines
    let mut grouped_words = Vec::new();
    let mut current_line = String::new();

    // Reduced target length to prevent overflow - Design 4 has less horizontal space
    // after the vertical divider at x=420
    let target_length = 20;  // Conservative length for right-side panel
    let max_lines = 4;

    let mut word_index = 0;
    while word_index < words.len() {
        let word = &words[word_index];
        let test_line = if current_line.is_empty() {
            word.clone()
        } else {
            format!("{} {}", current_line, word)
        };

        // Check if adding this word would exceed target length
        if test_line.len() <= target_length {
            current_line = test_line;
        } else {
            // Current line is done, start new line with this word
            if !current_line.is_empty() {
                grouped_words.push(current_line);
                current_line = word.clone();
            } else {
                // Single word is too long, add it anyway
                current_line = word.clone();
            }
        }

        // Limit to max 4 lines
        if grouped_words.len() >= max_lines - 1 && !current_line.is_empty() {
            // We're at max lines, just combine the rest
            for i in (word_index + 1)..words.len() {
                let remaining_word = &words[i];
                if current_line.len() + remaining_word.len() + 1 <= target_length * 2 {
                    current_line.push(' ');
                    current_line.push_str(remaining_word);
                }
            }
            break;
        }

        word_index += 1;
    }

    if !current_line.is_empty() {
        grouped_words.push(current_line);
    }

    // Limit to exactly 4 lines maximum
    if grouped_words.len() > max_lines {
        grouped_words.truncate(max_lines);
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
    // Standard OG layout (1200x630): title on right side starting at x=360
    let mut title_text = String::new();
    let mut y_position = 230.0;  // Starting Y position for standard OG format
    let line_height = 70.0;  // Compact line height to fit in 630px height
    let words_count = words.len() as f32;

    // Dynamically adjust font size based on title length (number of lines)
    // Standard OG format uses smaller fonts overall due to reduced height
    let title_font_size = if words_count > 3.0 {
        52.0  // 4 lines - smallest font
    } else if words_count > 2.0 {
        60.0  // 3 lines
    } else if words_count > 1.0 {
        68.0  // 2 lines
    } else {
        76.0  // 1 line - largest font
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
            "    <text x=\"360\" y=\"{}\" fill=\"#DCD7BA\" font-family=\"Arial, sans-serif\" font-size=\"{}\" font-weight=\"bold\">{}</text>\n",
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
