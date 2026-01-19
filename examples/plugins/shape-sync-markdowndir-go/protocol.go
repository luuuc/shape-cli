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
	Briefs []Brief `json:"briefs"`
}

// Brief represents a brief from Shape.
type Brief struct {
	ID       string `json:"id"`
	Title    string `json:"title"`
	Status   string `json:"status"`
	Appetite string `json:"appetite,omitempty"`
	Body     string `json:"body"`
}

// IdMapping represents a mapping between local and remote IDs.
type IdMapping struct {
	LocalID    string `json:"local_id"`
	RemoteID   string `json:"remote_id"`
	EntityType string `json:"entity_type"`
}

// SyncResult contains the result data for push/pull operations.
type SyncResult struct {
	Briefs    []Brief     `json:"briefs,omitempty"`
	Tasks     []any       `json:"tasks,omitempty"`
	Mappings  []IdMapping `json:"mappings"`
	Pushed    int         `json:"pushed"`
	Pulled    int         `json:"pulled"`
	Conflicts int         `json:"conflicts"`
	Errors    []string    `json:"errors"`
}

// StatusResult contains the result data for status operations.
type StatusResult struct {
	MappedBriefs int `json:"mapped_briefs"`
	MappedTasks  int `json:"mapped_tasks"`
}

// Manifest describes the plugin's capabilities.
type Manifest struct {
	Name        string   `json:"name"`
	Version     string   `json:"version"`
	Description string   `json:"description"`
	Type        string   `json:"type"`
	Operations  []string `json:"operations"`
}
