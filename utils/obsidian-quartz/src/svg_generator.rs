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
    let words: Vec<String> = split_title(&config.title);
    let svg = create_svg(&words, config.width, config.height)?;
    
    // First save the SVG to a temporary file
    let temp_svg_path = config.output_path.clone();
    fs::write(&temp_svg_path, svg)?;
    
    // Convert to WebP using modern ImageMagick command
    let output_path = temp_svg_path.replace(".svg", ".webp");
    
    let status = Command::new("magick")
        .arg(&temp_svg_path)
        // .arg("-quality")
        // .arg("90")  // for JPG quality
        // .arg("-resize")
        // .arg("1200x630!") // force exact size
        .arg(&output_path)
        .status()?;

    if !status.success() {
        return Err("ImageMagick conversion failed".into());
    }
    
    // Optionally remove the temporary SVG file
    fs::remove_file(temp_svg_path)?;
    
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
    // Twitter/LinkedIn Card aspect ratio is typically 1.91:1
    // Twitter: 1200x628px
    // LinkedIn: 1104x576px
    let background_color = "#000000";
    let text_color = "#FFFFFF";
    let accent_color = "#D4AF37";
    
    let mut svg = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 628">
    <!-- Background -->
    <rect width="100%" height="100%" fill="{background_color}"/>
    
    <!-- Decorative Grid - Adjusted for wider format -->
    <g stroke="{accent_color}" stroke-width="0.5" opacity="0.2">
        <line x1="0" y1="157" x2="1200" y2="157"/>
        <line x1="0" y1="314" x2="1200" y2="314"/>
        <line x1="0" y1="471" x2="1200" y2="471"/>
        <line x1="300" y1="0" x2="300" y2="628"/>
        <line x1="600" y1="0" x2="600" y2="628"/>
        <line x1="900" y1="0" x2="900" y2="628"/>
    </g>

    <!-- Hexagonal Network - Scaled and centered -->
    <g stroke="{accent_color}" stroke-width="2" fill="none">
        <!-- Center Hexagon - Moved right for better balance -->
        <path d="M 600 250 L 670 285 L 670 355 L 600 390 L 530 355 L 530 285 Z"/>
        
        <!-- Connected Hexagons - Adjusted positions -->
        <path d="M 460 155 L 530 190 L 530 260 L 460 295 L 390 260 L 390 190 Z" opacity="0.7"/>
        <path d="M 740 155 L 810 190 L 810 260 L 740 295 L 670 260 L 670 190 Z" opacity="0.7"/>
        <path d="M 460 385 L 530 420 L 530 490 L 460 525 L 390 490 L 390 420 Z" opacity="0.7"/>
        <path d="M 740 385 L 810 420 L 810 490 L 740 525 L 670 490 L 670 420 Z" opacity="0.7"/>
        
        <!-- Connection Lines - Adjusted -->
        <line x1="530" y1="285" x2="530" y2="355" opacity="0.5"/>
        <line x1="670" y1="285" x2="670" y2="355" opacity="0.5"/>
        <line x1="460" y1="295" x2="460" y2="385" opacity="0.5"/>
        <line x1="740" y1="295" x2="740" y2="385" opacity="0.5"/>
    </g>

    <!-- Glowing Center - Adjusted position -->
    <circle cx="600" cy="314" r="40" fill="{accent_color}" opacity="0.2">
        <animate attributeName="opacity" values="0.1;0.3;0.1" dur="4s" repeatCount="indefinite"/>
    </circle>

    <!-- Header - Second Brain -->
    <text x="600" y="100" fill="{accent_color}" font-family="Arial" font-size="32" font-weight="bold" text-anchor="middle">
        SECOND BRAIN
    </text>
"#
    );

    // Add main title words - Optimized for social media card
    let mut y_position = 200.0;
    let line_height = 80.0;
    let words_count = words.len() as f32;
    
    // Dynamically adjust font size based on title length
    let title_font_size = if words_count > 3.0 {
        72.0
    } else if words_count > 2.0 {
        84.0
    } else {
        96.0
    };

    for word in words.iter() {
        svg.push_str(&format!(
            r#"    <text x="600" y="{}" fill="{text_color}" font-family="Arial" font-size="{}" font-weight="bold" text-anchor="middle">{}</text>
"#,
            y_position, title_font_size, word
        ));
        y_position += line_height;
    }

    // Add footer - Adjusted position
    svg.push_str(&format!(
        r#"    <text x="600" y="560" fill="{accent_color}" font-family="Arial" font-size="28" text-anchor="middle" letter-spacing="2">
        A Digital Vault of Knowledge
    </text>
</svg>"#
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
