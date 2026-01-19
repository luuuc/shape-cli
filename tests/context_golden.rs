//! Golden file tests for context export
//!
//! These tests verify that the `shape context` output format remains stable
//! for AI agent consumption. The compact format is the primary AI contract.

use serde_json::Value;
use tempfile::TempDir;

/// Get a command instance for the shape binary
fn shape_cmd() -> assert_cmd::Command {
    assert_cmd::Command::new(assert_cmd::cargo::cargo_bin!("shape"))
}

/// Create a project with test data for context testing
fn setup_context_test_project() -> TempDir {
    let dir = TempDir::new().unwrap();

    // Initialize
    shape_cmd().arg("init").arg(dir.path()).assert().success();

    // Create brief
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "new", "Test Anchor", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    let brief_id = json["id"].as_str().unwrap().to_string();

    // Create tasks with various states
    let output1 = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", &brief_id, "Ready Task", "--format", "json"])
        .assert()
        .success();
    let task1_id =
        serde_json::from_str::<Value>(&String::from_utf8_lossy(&output1.get_output().stdout))
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

    let output2 = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", &brief_id, "Blocked Task", "--format", "json"])
        .assert()
        .success();
    let task2_id =
        serde_json::from_str::<Value>(&String::from_utf8_lossy(&output2.get_output().stdout))
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

    let output3 = shape_cmd()
        .current_dir(dir.path())
        .args([
            "task",
            "add",
            &brief_id,
            "In Progress Task",
            "--format",
            "json",
        ])
        .assert()
        .success();
    let task3_id =
        serde_json::from_str::<Value>(&String::from_utf8_lossy(&output3.get_output().stdout))
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

    let output4 = shape_cmd()
        .current_dir(dir.path())
        .args([
            "task",
            "add",
            &brief_id,
            "Completed Task",
            "--format",
            "json",
        ])
        .assert()
        .success();
    let task4_id =
        serde_json::from_str::<Value>(&String::from_utf8_lossy(&output4.get_output().stdout))
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

    // Set up dependency: task2 blocked by task1
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "dep", &task2_id, &task1_id])
        .assert()
        .success();

    // Start task3
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "start", &task3_id])
        .assert()
        .success();

    // Complete task4
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "done", &task4_id])
        .assert()
        .success();

    dir
}

// =============================================================================
// Compact Format Schema Tests
// =============================================================================

#[test]
fn test_compact_format_has_required_top_level_keys() {
    let dir = setup_context_test_project();

    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["context", "--compact"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    // Verify required top-level keys exist
    assert!(json.is_object(), "Context must be a JSON object");
    assert!(json.get("briefs").is_some(), "Missing 'briefs' key");
    assert!(json.get("ready").is_some(), "Missing 'ready' key");
    assert!(
        json.get("in_progress").is_some(),
        "Missing 'in_progress' key"
    );
    assert!(json.get("blocked").is_some(), "Missing 'blocked' key");
    assert!(
        json.get("recently_done").is_some(),
        "Missing 'recently_done' key"
    );
}

#[test]
fn test_compact_format_briefs_are_array() {
    let dir = setup_context_test_project();

    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["context", "--compact"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let briefs = json.get("briefs").unwrap();
    assert!(briefs.is_array(), "'briefs' must be an array");

    // Each brief must have id, title, status
    for brief in briefs.as_array().unwrap() {
        assert!(brief.get("id").is_some(), "Anchor missing 'id'");
        assert!(brief.get("title").is_some(), "Anchor missing 'title'");
        assert!(brief.get("status").is_some(), "Anchor missing 'status'");
    }
}

#[test]
fn test_compact_format_ready_is_string_array() {
    let dir = setup_context_test_project();

    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["context", "--compact"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let ready = json.get("ready").unwrap();
    assert!(ready.is_array(), "'ready' must be an array");

    // Each item must be a string in format "id: title"
    for item in ready.as_array().unwrap() {
        assert!(item.is_string(), "Ready item must be a string");
        let s = item.as_str().unwrap();
        assert!(
            s.contains(": "),
            "Ready item must be in 'id: title' format, got: {}",
            s
        );
    }
}

#[test]
fn test_compact_format_blocked_includes_blockers() {
    let dir = setup_context_test_project();

    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["context", "--compact"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let blocked = json.get("blocked").unwrap();
    assert!(blocked.is_array(), "'blocked' must be an array");

    // Each blocked item should indicate what it's blocked by
    for item in blocked.as_array().unwrap() {
        assert!(item.is_string(), "Blocked item must be a string");
        let s = item.as_str().unwrap();
        assert!(
            s.contains("blocked by"),
            "Blocked item must indicate blocker: {}",
            s
        );
    }
}

#[test]
fn test_compact_format_in_progress_is_string_array() {
    let dir = setup_context_test_project();

    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["context", "--compact"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let in_progress = json.get("in_progress").unwrap();
    assert!(in_progress.is_array(), "'in_progress' must be an array");

    for item in in_progress.as_array().unwrap() {
        assert!(item.is_string(), "In-progress item must be a string");
    }
}

#[test]
fn test_compact_format_recently_done_is_string_array() {
    let dir = setup_context_test_project();

    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["context", "--compact"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let recently_done = json.get("recently_done").unwrap();
    assert!(recently_done.is_array(), "'recently_done' must be an array");

    for item in recently_done.as_array().unwrap() {
        assert!(item.is_string(), "Recently done item must be a string");
    }
}

// =============================================================================
// Full Format Schema Tests
// =============================================================================

#[test]
fn test_full_format_has_required_structure() {
    let dir = setup_context_test_project();

    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["context"]) // Full format (no --compact)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    // Verify required top-level keys
    assert!(json.get("briefs").is_some(), "Missing 'briefs' key");
    assert!(json.get("tasks").is_some(), "Missing 'tasks' key");
    assert!(json.get("summary").is_some(), "Missing 'summary' key");
}

#[test]
fn test_full_format_tasks_structure() {
    let dir = setup_context_test_project();

    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["context"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let tasks = json.get("tasks").unwrap();
    assert!(tasks.is_object(), "'tasks' must be an object");

    // Verify task categories exist
    assert!(tasks.get("ready").is_some(), "Missing tasks.ready");
    assert!(
        tasks.get("in_progress").is_some(),
        "Missing tasks.in_progress"
    );
    assert!(tasks.get("blocked").is_some(), "Missing tasks.blocked");
    assert!(
        tasks.get("recently_completed").is_some(),
        "Missing tasks.recently_completed"
    );
}

#[test]
fn test_full_format_summary_structure() {
    let dir = setup_context_test_project();

    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["context"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let summary = json.get("summary").unwrap();
    assert!(summary.is_object(), "'summary' must be an object");

    // Verify summary fields
    assert!(
        summary.get("total_briefs").is_some(),
        "Missing summary.total_briefs"
    );
    assert!(
        summary.get("total_tasks").is_some(),
        "Missing summary.total_tasks"
    );
    assert!(
        summary.get("ready_count").is_some(),
        "Missing summary.ready_count"
    );
    assert!(
        summary.get("blocked_count").is_some(),
        "Missing summary.blocked_count"
    );
    assert!(
        summary.get("in_progress_count").is_some(),
        "Missing summary.in_progress_count"
    );
}

#[test]
fn test_full_format_brief_includes_body() {
    let dir = setup_context_test_project();

    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["context"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let briefs = json.get("briefs").unwrap().as_array().unwrap();
    for brief in briefs {
        assert!(brief.get("id").is_some(), "Anchor missing 'id'");
        assert!(brief.get("title").is_some(), "Anchor missing 'title'");
        assert!(brief.get("type").is_some(), "Anchor missing 'type'");
        assert!(brief.get("status").is_some(), "Anchor missing 'status'");
        assert!(brief.get("body").is_some(), "Anchor missing 'body'");
        assert!(brief.get("meta").is_some(), "Anchor missing 'meta'");
    }
}

#[test]
fn test_full_format_ready_task_includes_details() {
    let dir = setup_context_test_project();

    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["context"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let ready = json["tasks"]["ready"].as_array().unwrap();
    for task in ready {
        assert!(task.get("id").is_some(), "Task missing 'id'");
        assert!(task.get("title").is_some(), "Task missing 'title'");
        assert!(task.get("brief").is_some(), "Task missing 'brief'");
    }
}

// =============================================================================
// Data Integrity Tests
// =============================================================================

#[test]
fn test_context_reflects_actual_state() {
    let dir = setup_context_test_project();

    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["context", "--compact"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    // Should have 1 brief
    assert_eq!(json["briefs"].as_array().unwrap().len(), 1);

    // Should have 1 ready task (Ready Task)
    let ready: Vec<&str> = json["ready"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(
        ready.iter().any(|s| s.contains("Ready Task")),
        "Ready Task not in ready list"
    );

    // Should have 1 blocked task (Blocked Task)
    let blocked: Vec<&str> = json["blocked"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(
        blocked.iter().any(|s| s.contains("Blocked Task")),
        "Blocked Task not in blocked list"
    );

    // Should have 1 in-progress task (In Progress Task)
    let in_progress: Vec<&str> = json["in_progress"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(
        in_progress.iter().any(|s| s.contains("In Progress Task")),
        "In Progress Task not in in_progress list"
    );

    // Should have 1 recently completed (Completed Task)
    let recently_done: Vec<&str> = json["recently_done"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(
        recently_done.iter().any(|s| s.contains("Completed Task")),
        "Completed Task not in recently_done list"
    );
}

#[test]
fn test_context_brief_filter() {
    let dir = TempDir::new().unwrap();

    // Initialize
    shape_cmd().arg("init").arg(dir.path()).assert().success();

    // Create two briefs
    let output1 = shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "new", "Anchor One", "--format", "json"])
        .assert()
        .success();
    let brief1_id =
        serde_json::from_str::<Value>(&String::from_utf8_lossy(&output1.get_output().stdout))
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

    let output2 = shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "new", "Anchor Two", "--format", "json"])
        .assert()
        .success();
    let _brief2_id =
        serde_json::from_str::<Value>(&String::from_utf8_lossy(&output2.get_output().stdout))
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

    // Add task to brief1
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", &brief1_id, "Anchor1 Task"])
        .assert()
        .success();

    // Filter context by brief1
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["context", "--compact", "--brief", &brief1_id])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    // Should only have 1 brief
    assert_eq!(json["briefs"].as_array().unwrap().len(), 1);
    assert!(json["briefs"][0]["title"]
        .as_str()
        .unwrap()
        .contains("Anchor One"));
}

#[test]
fn test_empty_project_context() {
    let dir = TempDir::new().unwrap();

    shape_cmd().arg("init").arg(dir.path()).assert().success();

    // Context on empty project should work
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["context", "--compact"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    // All arrays should be empty
    assert_eq!(json["briefs"].as_array().unwrap().len(), 0);
    assert_eq!(json["ready"].as_array().unwrap().len(), 0);
    assert_eq!(json["blocked"].as_array().unwrap().len(), 0);
    assert_eq!(json["in_progress"].as_array().unwrap().len(), 0);
    assert_eq!(json["recently_done"].as_array().unwrap().len(), 0);
}
