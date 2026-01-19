package main

import (
	"bytes"
	"fmt"
	"regexp"
	"strings"
	"time"
)

var nonAlphanumeric = regexp.MustCompile(`[^a-z0-9]+`)

// slugify converts a title to a URL-friendly slug.
func slugify(title string) string {
	slug := strings.ToLower(title)
	slug = nonAlphanumeric.ReplaceAllString(slug, "-")
	slug = strings.Trim(slug, "-")
	return slug
}

// generateMarkdown creates markdown content with YAML frontmatter for a brief.
func generateMarkdown(brief Brief) string {
	var buf bytes.Buffer

	buf.WriteString("---\n")
	buf.WriteString(fmt.Sprintf("id: %s\n", brief.ID))
	buf.WriteString(fmt.Sprintf("title: %q\n", brief.Title))
	buf.WriteString(fmt.Sprintf("status: %s\n", brief.Status))
	if brief.Appetite != "" {
		buf.WriteString(fmt.Sprintf("appetite: %s\n", brief.Appetite))
	}
	buf.WriteString(fmt.Sprintf("synced_at: %s\n", time.Now().UTC().Format(time.RFC3339)))
	buf.WriteString("---\n\n")
	buf.WriteString(brief.Body)

	return buf.String()
}

// parseMarkdown parses markdown content with YAML frontmatter into a Brief.
func parseMarkdown(content string) (Brief, bool) {
	if !strings.HasPrefix(content, "---\n") {
		return Brief{}, false
	}

	endIndex := strings.Index(content[4:], "\n---\n")
	if endIndex == -1 {
		return Brief{}, false
	}

	frontmatter := content[4 : 4+endIndex]
	body := strings.TrimPrefix(content[4+endIndex+5:], "\n")

	brief := Brief{Body: body}

	for _, line := range strings.Split(frontmatter, "\n") {
		parts := strings.SplitN(line, ": ", 2)
		if len(parts) != 2 {
			continue
		}
		key := parts[0]
		value := parts[1]

		// Remove surrounding quotes if present
		if len(value) >= 2 && value[0] == '"' && value[len(value)-1] == '"' {
			value = value[1 : len(value)-1]
		}

		switch key {
		case "id":
			brief.ID = value
		case "title":
			brief.Title = value
		case "status":
			brief.Status = value
		case "appetite":
			brief.Appetite = value
		}
	}

	return brief, brief.ID != ""
}
