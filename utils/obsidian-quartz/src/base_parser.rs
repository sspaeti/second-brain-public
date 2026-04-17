use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Represents the complete BASE file structure
#[derive(Debug, Deserialize, Serialize)]
pub struct BaseFile {
    #[serde(default)]
    pub publish: bool,
    #[serde(default)]
    pub description: Option<String>,
    pub filters: Option<FilterNode>,
    pub views: Vec<View>,
    #[serde(default)]
    pub formulas: HashMap<String, String>,
    #[serde(default)]
    pub properties: HashMap<String, PropertyConfig>,
}

/// Filter node supporting and/or/not logic
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum FilterNode {
    Simple(String),
    Complex {
        #[serde(default)]
        and: Vec<FilterNode>,
        #[serde(default)]
        or: Vec<FilterNode>,
        #[serde(default)]
        not: Vec<FilterNode>,
    },
}

/// Property display configuration
#[derive(Debug, Deserialize, Serialize)]
pub struct PropertyConfig {
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
}

/// A view configuration (table, cards, list, map)
#[derive(Debug, Deserialize, Serialize)]
pub struct View {
    #[serde(rename = "type")]
    pub view_type: String,
    pub name: String,
    #[serde(default)]
    pub order: Vec<String>,
    #[serde(default)]
    pub sort: Vec<SortConfig>,
    pub filters: Option<FilterNode>,
    pub limit: Option<usize>,
    #[serde(rename = "groupBy")]
    pub group_by: Option<GroupBy>,
    #[serde(rename = "columnSize")]
    #[serde(default)]
    pub column_size: HashMap<String, i32>,
}

/// Sort configuration for a view
#[derive(Debug, Deserialize, Serialize)]
pub struct SortConfig {
    pub property: String,
    pub direction: String, // ASC or DESC
}

/// Group by configuration
#[derive(Debug, Deserialize, Serialize)]
pub struct GroupBy {
    pub property: String,
    pub direction: String,
}

/// Parse a .base file from the given path
pub fn parse_base_file(path: &Path) -> Result<BaseFile, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let base: BaseFile = serde_yaml::from_str(&content)?;
    Ok(base)
}

/// Extract simple filter expressions from a filter node
pub fn extract_filter_expressions(filter: &FilterNode) -> Vec<String> {
    let mut expressions = Vec::new();

    match filter {
        FilterNode::Simple(expr) => {
            expressions.push(expr.clone());
        }
        FilterNode::Complex { and, or, not } => {
            for f in and {
                expressions.extend(extract_filter_expressions(f));
            }
            for f in or {
                expressions.extend(extract_filter_expressions(f));
            }
            for f in not {
                expressions.extend(extract_filter_expressions(f));
            }
        }
    }

    expressions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_filter_simple() {
        let yaml = r#"
filters:
  and:
    - file.path.contains("Coffee Beans")
    - file.ext.contains("md")
views:
  - type: table
    name: Test
    order:
      - file.name
"#;
        let base: BaseFile = serde_yaml::from_str(yaml).unwrap();
        assert!(base.filters.is_some());

        let expressions = extract_filter_expressions(base.filters.as_ref().unwrap());
        assert_eq!(expressions.len(), 2);
    }
}
