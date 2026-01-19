package main

import "encoding/json"

// PluginRequest represents an incoming request from shape-cli.
type PluginRequest struct {
	Operation string          `json:"operation"`
	Params    json.RawMessage `json:"params"`
}

// PluginResponse represents a response to shape-cli.
type PluginResponse struct {
	Success bool            `json:"success"`
	Data    json.RawMessage `json:"data,omitempty"`
	Error   string          `json:"error,omitempty"`
}

// PushParams contains the parameters for a push operation.
type PushParams struct {
	Briefs []json.RawMessage `json:"briefs"`
	Tasks  []json.RawMessage `json:"tasks"`
}

// SyncResult contains the result data for push/pull operations.
type SyncResult struct {
	Briefs    []json.RawMessage `json:"briefs,omitempty"`
	Tasks     []json.RawMessage `json:"tasks,omitempty"`
	Mappings  []json.RawMessage `json:"mappings"`
	Pushed    int               `json:"pushed"`
	Pulled    int               `json:"pulled"`
	Conflicts int               `json:"conflicts"`
	Errors    []string          `json:"errors"`
}

// StatusResult contains the result data for status operations.
type StatusResult struct {
	MappedBriefs int `json:"mapped_briefs"`
	MappedTasks  int `json:"mapped_tasks"`
}

// SyncFile represents the structure of the shape-sync.json file.
type SyncFile struct {
	Briefs   []json.RawMessage `json:"briefs"`
	Tasks    []json.RawMessage `json:"tasks"`
	SyncedAt string            `json:"synced_at"`
}

// Manifest describes the plugin's capabilities.
type Manifest struct {
	Name        string   `json:"name"`
	Version     string   `json:"version"`
	Description string   `json:"description"`
	Type        string   `json:"type"`
	Operations  []string `json:"operations"`
}
