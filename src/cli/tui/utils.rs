//! Shared utilities for TUI views

/// Truncate a string to max_len characters, adding "..." if truncated
pub fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncate_at = max_len.saturating_sub(3);
        let truncated: String = s.chars().take(truncate_at).collect();
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_string() {
        assert_eq!(truncate_str("hello", 10), "hello");
    }

    #[test]
    fn truncate_exact_length() {
        assert_eq!(truncate_str("hello", 5), "hello");
    }

    #[test]
    fn truncate_long_string() {
        assert_eq!(truncate_str("hello world", 8), "hello...");
    }

    #[test]
    fn truncate_empty_string() {
        assert_eq!(truncate_str("", 5), "");
    }

    #[test]
    fn truncate_unicode() {
        // Unicode characters should be counted correctly
        assert_eq!(truncate_str("hello", 3), "...");
        assert_eq!(truncate_str("hi", 3), "hi");
    }
}
