package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

func getOutputDir() string {
	if dir := os.Getenv("SHAPE_SYNC_MARKDOWN_DIR"); dir != "" {
		return dir
	}
	return "./pitches"
}

func handleTest() PluginResponse {
	return PluginResponse{Success: true}
}

func handlePush(params json.RawMessage) PluginResponse {
	var p PushParams
	if err := json.Unmarshal(params, &p); err != nil {
		return PluginResponse{Success: false, Error: "invalid params: " + err.Error()}
	}

	if p.Briefs == nil {
		p.Briefs = []Brief{}
	}

	outputDir := getOutputDir()
	if err := os.MkdirAll(outputDir, 0755); err != nil {
		return PluginResponse{Success: false, Error: "failed to create directory: " + err.Error()}
	}

	mappings := make([]IdMapping, 0, len(p.Briefs))
	for _, brief := range p.Briefs {
		filename := fmt.Sprintf("%s-%s.md", brief.ID, slugify(brief.Title))
		filepath := filepath.Join(outputDir, filename)

		content := generateMarkdown(brief)
		if err := os.WriteFile(filepath, []byte(content), 0644); err != nil {
			return PluginResponse{Success: false, Error: "failed to write file: " + err.Error()}
		}

		mappings = append(mappings, IdMapping{
			LocalID:    brief.ID,
			RemoteID:   filename,
			EntityType: "brief",
		})
	}

	result := SyncResult{
		Pushed:    len(p.Briefs),
		Pulled:    0,
		Conflicts: 0,
		Errors:    []string{},
		Mappings:  mappings,
	}

	resultJSON, _ := json.Marshal(result)
	return PluginResponse{Success: true, Data: resultJSON}
}

func handlePull() PluginResponse {
	outputDir := getOutputDir()

	if _, err := os.Stat(outputDir); os.IsNotExist(err) {
		result := SyncResult{
			Briefs:    []Brief{},
			Tasks:     []any{},
			Mappings:  []IdMapping{},
			Pushed:    0,
			Pulled:    0,
			Conflicts: 0,
			Errors:    []string{},
		}
		resultJSON, _ := json.Marshal(result)
		return PluginResponse{Success: true, Data: resultJSON}
	}

	entries, err := os.ReadDir(outputDir)
	if err != nil {
		return PluginResponse{Success: false, Error: "failed to read directory: " + err.Error()}
	}

	briefs := []Brief{}
	for _, entry := range entries {
		if entry.IsDir() || !strings.HasSuffix(entry.Name(), ".md") {
			continue
		}

		content, err := os.ReadFile(filepath.Join(outputDir, entry.Name()))
		if err != nil {
			continue
		}

		if brief, ok := parseMarkdown(string(content)); ok {
			briefs = append(briefs, brief)
		}
	}

	result := SyncResult{
		Briefs:    briefs,
		Tasks:     []any{},
		Mappings:  []IdMapping{},
		Pushed:    0,
		Pulled:    len(briefs),
		Conflicts: 0,
		Errors:    []string{},
	}

	resultJSON, _ := json.Marshal(result)
	return PluginResponse{Success: true, Data: resultJSON}
}

func handleStatus() PluginResponse {
	outputDir := getOutputDir()
	count := 0

	if entries, err := os.ReadDir(outputDir); err == nil {
		for _, entry := range entries {
			if !entry.IsDir() && strings.HasSuffix(entry.Name(), ".md") {
				count++
			}
		}
	}

	result := StatusResult{
		MappedBriefs: count,
		MappedTasks:  0,
	}

	resultJSON, _ := json.Marshal(result)
	return PluginResponse{Success: true, Data: resultJSON}
}

func handleRequest(req PluginRequest) PluginResponse {
	switch req.Operation {
	case "test":
		return handleTest()
	case "push":
		return handlePush(req.Params)
	case "pull":
		return handlePull()
	case "status":
		return handleStatus()
	default:
		return PluginResponse{Success: false, Error: "Unknown operation: " + req.Operation}
	}
}
