use std::fs;
use std::process::Command;
use std::error::Error;
use std::path::Path;
use regex::Regex;

pub struct ImageConfig {
    pub title: String,
    pub description: Option<String>,
    pub width: u32,
    pub height: u32,
    pub output_path: String,
}

pub struct MermaidConfig {
    pub source: String,
    pub output_path: String,
    pub width: u32,
    pub height: u32,
}

/// Render a Mermaid diagram source to a 1200x630 WebP suitable for OG images.
/// Uses mmdc (mermaid-cli) to render PNG, then ImageMagick to composite onto a padded canvas.
pub fn generate_mermaid_og_image(config: &MermaidConfig) -> Result<(), Box<dyn Error>> {
    if Path::new(&config.output_path).exists() {
        return Ok(());
    }

    if let Some(parent) = Path::new(&config.output_path).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    let stem = config.output_path.trim_end_matches(".webp");
    let tmp_mmd = format!("{}.tmp.mmd", stem);
    let tmp_png = format!("{}.tmp.png", stem);

    fs::write(&tmp_mmd, &config.source)?;

    let bg = "#1F1F28";
    // Render mermaid at close to the target display width (so text stays at its
    // natural, readable size) with a high puppeteer scale factor for retina-sharp
    // anti-aliasing. Wider viewports spread the diagram out and shrink text on
    // downscale; -s only changes pixel density, not CSS layout.
    let render_width = config.width.saturating_sub(100).max(800); // ~1100 for 1200 canvas
    let render_w_str = render_width.to_string();
    let mmdc_out = Command::new("mmdc")
        .arg("-i").arg(&tmp_mmd)
        .arg("-o").arg(&tmp_png)
        .arg("-t").arg("dark")
        .arg("-b").arg(bg)
        .arg("-w").arg(&render_w_str)
        .arg("-s").arg("3")
        .arg("--quiet")
        .output();

    match mmdc_out {
        Ok(out) if !out.status.success() => {
            let err = String::from_utf8_lossy(&out.stderr).to_string();
            let _ = fs::remove_file(&tmp_mmd);
            return Err(format!("mmdc failed: {}", err).into());
        }
        Err(e) => {
            let _ = fs::remove_file(&tmp_mmd);
            return Err(format!("Failed to run mmdc: {}", e).into());
        }
        _ => {}
    }

    // Composite onto canvas: matching dark bg, ~50px padding each side, centered
    let pad = 50u32;
    let inner_w = config.width.saturating_sub(pad * 2);
    let inner_h = config.height.saturating_sub(pad * 2);
    let canvas = format!("{}x{}", config.width, config.height);
    let resize = format!("{}x{}", inner_w, inner_h);
    let canvas_fill = format!("xc:{}", bg);

    let magick_out = Command::new("magick")
        .arg("-size").arg(&canvas)
        .arg(&canvas_fill)
        .arg("(")
            .arg(&tmp_png)
            .arg("-filter").arg("Lanczos")
            .arg("-resize").arg(&resize)
        .arg(")")
        .arg("-gravity").arg("center")
        .arg("-composite")
        .arg("-quality").arg("92")
        .arg("-define").arg("webp:method=6")
        .arg(&config.output_path)
        .output();

    let _ = fs::remove_file(&tmp_mmd);
    let _ = fs::remove_file(&tmp_png);

    match magick_out {
        Ok(out) if !out.status.success() => {
            Err(format!(
                "magick composite failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )
            .into())
        }
        Err(e) => Err(format!("Failed to run magick: {}", e).into()),
        _ => Ok(()),
    }
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
    let svg = create_svg(&words, &config.description, config.width, config.height)?;
    
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
        .split(|c: char| c.is_whitespace())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    // Group words to create a balanced layout for Design 4 split-screen
    // We need to fit on the right side with max 4 lines
    let mut grouped_words = Vec::new();
    let mut current_line = String::new();

    // Account for right margin and logo space
    // Available: 1200px width - 360px start - 100px right margin = 740px usable
    // With font sizes 44-60px (avg ~30px/char), we can fit ~24-26 chars
    let target_length = 25;  // Balanced for proper margins with buffer
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

fn create_svg(words: &[String], description: &Option<String>, _width: u32, _height: u32) -> Result<String, Box<dyn Error>> {
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
    let mut y_position = 200.0;  // Starting Y position (moved up to make room for description)
    let line_height = 60.0;  // Reduced line height to fit description
    let words_count = words.len() as f32;

    // Reduced font sizes to make room for description (44-60px instead of 52-76px)
    let title_font_size = if words_count > 3.0 {
        44.0  // 4 lines - smallest font
    } else if words_count > 2.0 {
        50.0  // 3 lines
    } else if words_count > 1.0 {
        56.0  // 2 lines
    } else {
        60.0  // 1 line - largest font
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

    // Generate description text if available
    let mut description_text = String::new();
    if let Some(desc) = description {
        // Split description into lines (24px font ≈ 14px/char, 740px / 14px ≈ 52 chars)
        let desc_lines = split_description(desc, 60, 3);
        let mut desc_y = y_position + 30.0;  // Start 30px below title
        let desc_line_height = 34.0;
        let desc_font_size = 24.0;

        for line in desc_lines {
            let escaped_line = line
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&apos;");

            description_text.push_str(&format!(
                "    <text x=\"360\" y=\"{}\" fill=\"#C8C093\" font-family=\"Arial, sans-serif\" font-size=\"{}\" font-weight=\"normal\">{}</text>\n",
                desc_y, desc_font_size, escaped_line
            ));
            desc_y += desc_line_height;
        }
    }

    // Replace placeholders in template
    let svg = template
        .replace("{{TITLE_PLACEHOLDER}}", &title_text)
        .replace("{{DESCRIPTION_PLACEHOLDER}}", &description_text);

    Ok(svg)
}

/// Split description text into multiple lines for display
fn split_description(text: &str, max_chars: usize, max_lines: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in words {
        let test_line = if current_line.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current_line, word)
        };

        if test_line.len() <= max_chars {
            current_line = test_line;
        } else {
            if !current_line.is_empty() {
                lines.push(current_line);
                current_line = word.to_string();
            } else {
                // Single word too long, add it anyway
                lines.push(word.to_string());
            }
        }

        // Stop if we've reached max lines
        if lines.len() >= max_lines {
            break;
        }
    }

    if !current_line.is_empty() && lines.len() < max_lines {
        lines.push(current_line);
    }

    lines
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
