package main

import (
	"encoding/json"
	"os"
	"time"
)

const syncFile = "shape-sync.json"

func handleTest() PluginResponse {
	return PluginResponse{Success: true}
}

func handlePush(params json.RawMessage) PluginResponse {
	var p PushParams
	if err := json.Unmarshal(params, &p); err != nil {
		return PluginResponse{Success: false, Error: "invalid params: " + err.Error()}
	}

	if p.Briefs == nil {
		p.Briefs = []json.RawMessage{}
	}
	if p.Tasks == nil {
		p.Tasks = []json.RawMessage{}
	}

	data := SyncFile{
		Briefs:   p.Briefs,
		Tasks:    p.Tasks,
		SyncedAt: time.Now().UTC().Format(time.RFC3339),
	}

	content, err := json.MarshalIndent(data, "", "  ")
	if err != nil {
		return PluginResponse{Success: false, Error: "failed to marshal: " + err.Error()}
	}

	if err := os.WriteFile(syncFile, content, 0644); err != nil {
		return PluginResponse{Success: false, Error: "failed to write file: " + err.Error()}
	}

	result := SyncResult{
		Pushed:    len(p.Briefs),
		Pulled:    0,
		Conflicts: 0,
		Errors:    []string{},
		Mappings:  []json.RawMessage{},
	}

	resultJSON, _ := json.Marshal(result)
	return PluginResponse{Success: true, Data: resultJSON}
}

func handlePull() PluginResponse {
	content, err := os.ReadFile(syncFile)
	if err != nil {
		if os.IsNotExist(err) {
			result := SyncResult{
				Briefs:    []json.RawMessage{},
				Tasks:     []json.RawMessage{},
				Mappings:  []json.RawMessage{},
				Pushed:    0,
				Pulled:    0,
				Conflicts: 0,
				Errors:    []string{},
			}
			resultJSON, _ := json.Marshal(result)
			return PluginResponse{Success: true, Data: resultJSON}
		}
		return PluginResponse{Success: false, Error: "failed to read file: " + err.Error()}
	}

	var data SyncFile
	if err := json.Unmarshal(content, &data); err != nil {
		return PluginResponse{Success: false, Error: "failed to parse file: " + err.Error()}
	}

	if data.Briefs == nil {
		data.Briefs = []json.RawMessage{}
	}
	if data.Tasks == nil {
		data.Tasks = []json.RawMessage{}
	}

	result := SyncResult{
		Briefs:    data.Briefs,
		Tasks:     data.Tasks,
		Mappings:  []json.RawMessage{},
		Pushed:    0,
		Pulled:    len(data.Briefs),
		Conflicts: 0,
		Errors:    []string{},
	}

	resultJSON, _ := json.Marshal(result)
	return PluginResponse{Success: true, Data: resultJSON}
}

func handleStatus() PluginResponse {
	result := StatusResult{
		MappedBriefs: 0,
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
