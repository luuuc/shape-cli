//! Plugin protocol types
//!
//! Plugins communicate via JSON messages over stdin/stdout.
//! Each plugin must support the `--manifest` flag to declare capabilities.

use serde::{Deserialize, Serialize};

/// Plugin manifest declaring capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin name (e.g., "shape-anchor-shapeup")
    pub name: String,

    /// Plugin version
    pub version: String,

    /// Human-readable description
    pub description: String,

    /// Plugin type
    #[serde(rename = "type")]
    pub plugin_type: PluginType,

    /// Supported operations
    pub operations: Vec<String>,
}

/// Type of plugin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginType {
    /// Anchor type plugin (templates and validation)
    AnchorType,

    /// Sync plugin (external tool integration)
    Sync,
}

/// A message sent to a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRequest {
    /// The operation to perform
    pub operation: String,

    /// Operation-specific parameters
    pub params: serde_json::Value,
}

impl PluginRequest {
    pub fn new(operation: impl Into<String>, params: impl Into<serde_json::Value>) -> Self {
        Self {
            operation: operation.into(),
            params: params.into(),
        }
    }
}

/// A response from a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResponse {
    /// Whether the operation succeeded
    pub success: bool,

    /// Result data (if success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,

    /// Error message (if failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl PluginResponse {
    pub fn success(data: impl Into<serde_json::Value>) -> Self {
        Self {
            success: true,
            data: Some(data.into()),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// General plugin message wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginMessage {
    Request(PluginRequest),
    Response(PluginResponse),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_serialization() {
        let manifest = PluginManifest {
            name: "shape-anchor-shapeup".to_string(),
            version: "0.1.0".to_string(),
            description: "ShapeUp anchor type".to_string(),
            plugin_type: PluginType::AnchorType,
            operations: vec!["template".to_string(), "parse".to_string()],
        };

        let json = serde_json::to_string(&manifest).unwrap();
        let parsed: PluginManifest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, manifest.name);
        assert_eq!(parsed.plugin_type, PluginType::AnchorType);
    }

    #[test]
    fn request_serialization() {
        let request = PluginRequest::new("template", serde_json::json!({"title": "Test"}));
        let json = serde_json::to_string(&request).unwrap();

        assert!(json.contains("template"));
        assert!(json.contains("Test"));
    }

    #[test]
    fn response_success() {
        let response = PluginResponse::success(serde_json::json!({"result": "ok"}));

        assert!(response.success);
        assert!(response.data.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn response_error() {
        let response = PluginResponse::error("Something went wrong");

        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some("Something went wrong".to_string()));
    }
}
