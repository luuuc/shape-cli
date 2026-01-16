//! Anchor type plugin interface
//!
//! Anchor type plugins provide:
//! - Templates for new anchors
//! - Parsing and validation of frontmatter
//! - Status definitions

use serde::{Deserialize, Serialize};

use super::loader::PluginLoader;
use super::protocol::PluginRequest;

/// Template for creating a new anchor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorTemplate {
    /// Default frontmatter fields
    pub frontmatter: serde_json::Value,

    /// Default body content
    pub body: String,

    /// Valid status values for this anchor type
    pub statuses: Vec<String>,
}

/// Parsed anchor data with validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseResult {
    /// Whether the anchor is valid
    pub valid: bool,

    /// Parsed metadata (if valid)
    pub metadata: Option<serde_json::Value>,

    /// Validation errors (if invalid)
    pub errors: Vec<ValidationError>,
}

/// A validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Field that failed validation
    pub field: String,

    /// Error message
    pub message: String,
}

/// Anchor type plugin wrapper
pub struct AnchorTypePlugin<'a> {
    loader: &'a PluginLoader,
    plugin_name: String,
}

impl<'a> AnchorTypePlugin<'a> {
    /// Creates a new anchor type plugin wrapper
    pub fn new(loader: &'a PluginLoader, plugin_name: impl Into<String>) -> Self {
        Self {
            loader,
            plugin_name: plugin_name.into(),
        }
    }

    /// Gets the template for a new anchor
    pub fn template(&self, title: &str) -> anyhow::Result<AnchorTemplate> {
        let request = PluginRequest::new(
            "template",
            serde_json::json!({
                "title": title,
            }),
        );

        let response = self.loader.execute(&self.plugin_name, &request)?;

        if !response.success {
            anyhow::bail!(
                "Plugin error: {}",
                response.error.unwrap_or_else(|| "Unknown error".to_string())
            );
        }

        let data = response
            .data
            .ok_or_else(|| anyhow::anyhow!("No template data returned"))?;

        let template: AnchorTemplate =
            serde_json::from_value(data).context("Failed to parse template")?;

        Ok(template)
    }

    /// Parses and validates an anchor
    pub fn parse(&self, frontmatter: &serde_json::Value, body: &str) -> anyhow::Result<ParseResult> {
        let request = PluginRequest::new(
            "parse",
            serde_json::json!({
                "frontmatter": frontmatter,
                "body": body,
            }),
        );

        let response = self.loader.execute(&self.plugin_name, &request)?;

        if !response.success {
            anyhow::bail!(
                "Plugin error: {}",
                response.error.unwrap_or_else(|| "Unknown error".to_string())
            );
        }

        let data = response
            .data
            .ok_or_else(|| anyhow::anyhow!("No parse result returned"))?;

        let result: ParseResult =
            serde_json::from_value(data).context("Failed to parse result")?;

        Ok(result)
    }

    /// Gets valid statuses for this anchor type
    pub fn statuses(&self) -> anyhow::Result<Vec<String>> {
        let request = PluginRequest::new("statuses", serde_json::json!({}));

        let response = self.loader.execute(&self.plugin_name, &request)?;

        if !response.success {
            anyhow::bail!(
                "Plugin error: {}",
                response.error.unwrap_or_else(|| "Unknown error".to_string())
            );
        }

        let data = response
            .data
            .ok_or_else(|| anyhow::anyhow!("No statuses returned"))?;

        let statuses: Vec<String> =
            serde_json::from_value(data).context("Failed to parse statuses")?;

        Ok(statuses)
    }
}

use anyhow::Context;

/// Built-in minimal anchor type (no plugin required)
pub struct MinimalAnchorType;

impl MinimalAnchorType {
    /// Gets the template for a minimal anchor
    pub fn template(title: &str) -> AnchorTemplate {
        AnchorTemplate {
            frontmatter: serde_json::json!({
                "title": title,
                "status": "proposed",
            }),
            body: format!("# {}\n\n## Description\n\n", title),
            statuses: Self::statuses(),
        }
    }

    /// Returns valid statuses
    pub fn statuses() -> Vec<String> {
        vec![
            "proposed".to_string(),
            "in_progress".to_string(),
            "shipped".to_string(),
            "archived".to_string(),
        ]
    }

    /// Validates a minimal anchor
    pub fn validate(frontmatter: &serde_json::Value) -> ParseResult {
        let mut errors = Vec::new();

        // Check required fields
        if frontmatter.get("title").is_none() {
            errors.push(ValidationError {
                field: "title".to_string(),
                message: "Title is required".to_string(),
            });
        }

        // Check status is valid
        if let Some(status) = frontmatter.get("status").and_then(|v| v.as_str()) {
            if !Self::statuses().contains(&status.to_string()) {
                errors.push(ValidationError {
                    field: "status".to_string(),
                    message: format!("Invalid status: {}. Valid values: {:?}", status, Self::statuses()),
                });
            }
        }

        ParseResult {
            valid: errors.is_empty(),
            metadata: if errors.is_empty() {
                Some(frontmatter.clone())
            } else {
                None
            },
            errors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_template() {
        let template = MinimalAnchorType::template("Test Pitch");

        assert_eq!(
            template.frontmatter.get("title").unwrap().as_str().unwrap(),
            "Test Pitch"
        );
        assert!(template.body.contains("# Test Pitch"));
    }

    #[test]
    fn minimal_statuses() {
        let statuses = MinimalAnchorType::statuses();

        assert!(statuses.contains(&"proposed".to_string()));
        assert!(statuses.contains(&"in_progress".to_string()));
        assert!(statuses.contains(&"shipped".to_string()));
    }

    #[test]
    fn minimal_validation_valid() {
        let frontmatter = serde_json::json!({
            "title": "Test",
            "status": "proposed",
        });

        let result = MinimalAnchorType::validate(&frontmatter);
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn minimal_validation_missing_title() {
        let frontmatter = serde_json::json!({
            "status": "proposed",
        });

        let result = MinimalAnchorType::validate(&frontmatter);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.field == "title"));
    }

    #[test]
    fn minimal_validation_invalid_status() {
        let frontmatter = serde_json::json!({
            "title": "Test",
            "status": "invalid_status",
        });

        let result = MinimalAnchorType::validate(&frontmatter);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.field == "status"));
    }
}
