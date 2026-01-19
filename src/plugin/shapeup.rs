//! ShapeUp anchor type plugin
//!
//! Implements the ShapeUp methodology with:
//! - Appetite fields (6-weeks, 2-weeks, 1-week)
//! - Problem, Solution, Rabbit Holes, No-Gos sections
//! - ShapeUp-specific statuses (proposed, betting, in_progress, shipped, archived)

use super::anchor_type::{AnchorTemplate, ParseResult, ValidationError};

/// ShapeUp anchor type implementation (built into core)
pub struct ShapeUpAnchorType;

impl ShapeUpAnchorType {
    /// Gets the template for a ShapeUp pitch
    pub fn template(title: &str) -> AnchorTemplate {
        let body = format!(
            r#"# {}

## Problem

What is the problem we're trying to solve? Who has this problem? Why is it worth solving now?

## Appetite

<!-- 6-weeks | 2-weeks | 1-week -->
6-weeks

## Solution

### Overview

Describe the solution at a high level.

### Key Flows

Walk through the main user flows.

## Rabbit Holes

What are we explicitly NOT doing? What paths should we avoid going down?

-

## No-Gos

What is out of scope for this pitch?

-
"#,
            title
        );

        AnchorTemplate {
            frontmatter: serde_json::json!({
                "title": title,
                "status": "proposed",
                "appetite": "6-weeks",
            }),
            body,
            statuses: Self::statuses(),
        }
    }

    /// Returns valid statuses for ShapeUp anchors
    pub fn statuses() -> Vec<String> {
        vec![
            "proposed".to_string(),
            "betting".to_string(),
            "in_progress".to_string(),
            "shipped".to_string(),
            "archived".to_string(),
        ]
    }

    /// Valid appetite values
    pub fn appetites() -> Vec<&'static str> {
        vec![
            "6-weeks", "6 weeks", "2-weeks", "2 weeks", "1-week", "1 week",
        ]
    }

    /// Validates a ShapeUp anchor frontmatter
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
                    message: format!(
                        "Invalid status: {}. Valid values: {:?}",
                        status,
                        Self::statuses()
                    ),
                });
            }
        }

        // Check appetite is valid
        if let Some(appetite) = frontmatter.get("appetite").and_then(|v| v.as_str()) {
            if !Self::appetites().contains(&appetite) {
                errors.push(ValidationError {
                    field: "appetite".to_string(),
                    message: format!(
                        "Invalid appetite: {}. Valid values: {:?}",
                        appetite,
                        Self::appetites()
                    ),
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
    fn shapeup_template() {
        let template = ShapeUpAnchorType::template("My Pitch");

        assert_eq!(
            template.frontmatter.get("title").unwrap().as_str().unwrap(),
            "My Pitch"
        );
        assert_eq!(
            template
                .frontmatter
                .get("appetite")
                .unwrap()
                .as_str()
                .unwrap(),
            "6-weeks"
        );
        assert!(template.body.contains("## Problem"));
        assert!(template.body.contains("## Appetite"));
        assert!(template.body.contains("## Solution"));
        assert!(template.body.contains("## Rabbit Holes"));
        assert!(template.body.contains("## No-Gos"));
    }

    #[test]
    fn shapeup_statuses() {
        let statuses = ShapeUpAnchorType::statuses();

        assert!(statuses.contains(&"proposed".to_string()));
        assert!(statuses.contains(&"betting".to_string()));
        assert!(statuses.contains(&"in_progress".to_string()));
        assert!(statuses.contains(&"shipped".to_string()));
        assert!(statuses.contains(&"archived".to_string()));
    }

    #[test]
    fn shapeup_validation_valid() {
        let frontmatter = serde_json::json!({
            "title": "Test",
            "status": "proposed",
            "appetite": "6-weeks",
        });

        let result = ShapeUpAnchorType::validate(&frontmatter);
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn shapeup_validation_missing_title() {
        let frontmatter = serde_json::json!({
            "status": "proposed",
            "appetite": "6-weeks",
        });

        let result = ShapeUpAnchorType::validate(&frontmatter);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.field == "title"));
    }

    #[test]
    fn shapeup_validation_invalid_appetite() {
        let frontmatter = serde_json::json!({
            "title": "Test",
            "status": "proposed",
            "appetite": "3-weeks",
        });

        let result = ShapeUpAnchorType::validate(&frontmatter);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.field == "appetite"));
    }

    #[test]
    fn shapeup_validation_invalid_status() {
        let frontmatter = serde_json::json!({
            "title": "Test",
            "status": "invalid_status",
            "appetite": "6-weeks",
        });

        let result = ShapeUpAnchorType::validate(&frontmatter);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.field == "status"));
    }

    #[test]
    fn shapeup_accepts_betting_status() {
        let frontmatter = serde_json::json!({
            "title": "Test",
            "status": "betting",
            "appetite": "2-weeks",
        });

        let result = ShapeUpAnchorType::validate(&frontmatter);
        assert!(result.valid);
    }
}
