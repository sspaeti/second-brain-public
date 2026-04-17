use crate::base_parser::View;
use crate::base_query::Note;

/// Generate HTML table from notes and view configuration
pub fn render_table_view(notes: &[Note], view: &View) -> String {
    let mut html = String::new();

    // Start container
    html.push_str("<div class=\"base-table-container\">\n");

    // Table header with view name
    if !view.name.is_empty() && view.name != "Table" {
        html.push_str(&format!("<h3 class=\"base-table-title\">{}</h3>\n", view.name));
    }

    // Start table
    html.push_str("<table class=\"base-table\">\n");

    // Table head
    html.push_str("  <thead>\n    <tr>\n");
    for column in &view.order {
        let display_name = get_display_name(column);
        html.push_str(&format!("      <th>{}</th>\n", display_name));
    }
    html.push_str("    </tr>\n  </thead>\n");

    // Table body
    html.push_str("  <tbody>\n");
    for note in notes {
        html.push_str("    <tr>\n");
        for column in &view.order {
            let value = get_column_value(note, column);
            let formatted_value = format_value(&value, column);
            html.push_str(&format!("      <td>{}</td>\n", formatted_value));
        }
        html.push_str("    </tr>\n");
    }
    html.push_str("  </tbody>\n");

    // End table
    html.push_str("</table>\n");
    html.push_str("</div>\n");

    html
}

/// Generate HTML cards view (basic implementation)
pub fn render_cards_view(notes: &[Note], view: &View) -> String {
    let mut html = String::new();

    html.push_str("<div class=\"base-cards-container\">\n");

    if !view.name.is_empty() && view.name != "View" {
        html.push_str(&format!("<h3 class=\"base-cards-title\">{}</h3>\n", view.name));
    }

    html.push_str("<div class=\"base-cards-grid\">\n");

    for note in notes {
        html.push_str("  <div class=\"base-card\">\n");

        // Card header with file name
        html.push_str(&format!("    <h4 class=\"base-card-title\">{}</h4>\n", note.file_name));

        // Card content
        html.push_str("    <div class=\"base-card-content\">\n");
        for column in &view.order {
            if column == "file.name" {
                continue; // Already shown in title
            }
            let display_name = get_display_name(column);
            let value = get_column_value(note, column);
            if !value.is_empty() {
                html.push_str(&format!(
                    "      <div class=\"base-card-field\"><strong>{}:</strong> {}</div>\n",
                    display_name, value
                ));
            }
        }
        html.push_str("    </div>\n");

        html.push_str("  </div>\n");
    }

    html.push_str("</div>\n");
    html.push_str("</div>\n");

    html
}

/// Generate HTML list view
pub fn render_list_view(notes: &[Note], view: &View) -> String {
    let mut html = String::new();

    html.push_str("<div class=\"base-list-container\">\n");

    if !view.name.is_empty() && view.name != "List" {
        html.push_str(&format!("<h3 class=\"base-list-title\">{}</h3>\n", view.name));
    }

    html.push_str("<ul class=\"base-list\">\n");

    for note in notes {
        html.push_str("  <li class=\"base-list-item\">\n");
        html.push_str(&format!("    <strong>{}</strong>\n", note.file_name));

        // Show key properties inline
        let mut properties = Vec::new();
        for column in &view.order {
            if column == "file.name" {
                continue;
            }
            let value = get_column_value(note, column);
            if !value.is_empty() {
                let display_name = get_display_name(column);
                properties.push(format!("{}: {}", display_name, value));
            }
        }

        if !properties.is_empty() {
            html.push_str(&format!("    <span class=\"base-list-properties\">({})</span>\n", properties.join(", ")));
        }

        html.push_str("  </li>\n");
    }

    html.push_str("</ul>\n");
    html.push_str("</div>\n");

    html
}

/// Get display name for a column
fn get_display_name(column: &str) -> String {
    if column == "file.name" {
        "Name".to_string()
    } else if column.starts_with("file.") {
        // Convert file.mtime -> Mtime, etc.
        let name = column.strip_prefix("file.").unwrap();
        capitalize_first(name)
    } else {
        // Use the property name as-is (e.g., "Rösterei", "Rating")
        column.to_string()
    }
}

/// Get column value from note
fn get_column_value(note: &Note, column: &str) -> String {
    if column == "file.name" {
        note.file_name.clone()
    } else if column == "file.path" {
        note.file_path.to_string_lossy().to_string()
    } else {
        note.properties.get(column).cloned().unwrap_or_default()
    }
}

/// Format value for display (handle ratings, dates, etc.)
fn format_value(value: &str, column: &str) -> String {
    if value.is_empty() {
        return "-".to_string();
    }

    // Special formatting for file.name - make it a wikilink
    if column == "file.name" {
        return format!("[[{}]]", value);
    }

    // Special formatting for Rating
    if column == "Rating" {
        if let Ok(rating) = value.parse::<f32>() {
            let stars = (rating as i32).min(5);
            return "⭐".repeat(stars as usize);
        }
    }

    // Special formatting for Price
    if column == "Price" {
        if value.contains('.') || value.parse::<f32>().is_ok() {
            return format!("CHF {}", value);
        }
    }

    value.to_string()
}

/// Capitalize first letter of string
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base_query::Note;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_render_empty_table() {
        let notes = vec![];
        let view = View {
            view_type: "table".to_string(),
            name: "Test Table".to_string(),
            order: vec!["file.name".to_string(), "Rating".to_string()],
            sort: vec![],
            filters: None,
            limit: None,
            group_by: None,
            column_size: HashMap::new(),
        };

        let html = render_table_view(&notes, &view);
        assert!(html.contains("base-table"));
        assert!(html.contains("Test Table"));
    }

    #[test]
    fn test_render_table_with_notes() {
        let mut props = HashMap::new();
        props.insert("Rating".to_string(), "5".to_string());

        let note = Note {
            file_name: "Test Coffee".to_string(),
            file_path: PathBuf::from("/test/coffee.md"),
            properties: props,
        };

        let view = View {
            view_type: "table".to_string(),
            name: "Test".to_string(),
            order: vec!["file.name".to_string(), "Rating".to_string()],
            sort: vec![],
            filters: None,
            limit: None,
            group_by: None,
            column_size: HashMap::new(),
        };

        let html = render_table_view(&[note], &view);
        assert!(html.contains("Test Coffee"));
        assert!(html.contains("⭐⭐⭐⭐⭐"));
    }
}
