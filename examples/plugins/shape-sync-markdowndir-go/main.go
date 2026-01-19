package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"os"
)

var manifest = Manifest{
	Name:        "shape-sync-markdowndir-go",
	Version:     "1.0.0",
	Description: "Sync briefs to markdown directory (Go)",
	Type:        "sync",
	Operations:  []string{"push", "pull", "status", "test"},
}

func printManifest() {
	json.NewEncoder(os.Stdout).Encode(manifest)
}

func main() {
	if len(os.Args) > 1 && os.Args[1] == "--manifest" {
		printManifest()
		return
	}

	reader := bufio.NewReader(os.Stdin)
	line, err := reader.ReadString('\n')
	if err != nil {
		fmt.Fprintf(os.Stderr, "failed to read input: %v\n", err)
		os.Exit(1)
	}

	var req PluginRequest
	if err := json.Unmarshal([]byte(line), &req); err != nil {
		resp := PluginResponse{Success: false, Error: "invalid request: " + err.Error()}
		json.NewEncoder(os.Stdout).Encode(resp)
		return
	}

	resp := handleRequest(req)
	json.NewEncoder(os.Stdout).Encode(resp)
}
