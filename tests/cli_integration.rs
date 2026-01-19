//! CLI integration tests for Shape
//!
//! These tests verify the complete workflow from initialization through
//! task management, ensuring commands work together correctly.

use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Get a command instance for the shape binary
fn shape_cmd() -> assert_cmd::Command {
    assert_cmd::Command::new(assert_cmd::cargo::cargo_bin!("shape"))
}

/// Create a temporary directory and initialize a shape project
fn setup_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    shape_cmd().arg("init").arg(dir.path()).assert().success();
    dir
}

// =============================================================================
// Initialization Tests
// =============================================================================

#[test]
fn test_init_creates_structure() {
    let dir = TempDir::new().unwrap();

    shape_cmd()
        .arg("init")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized shape project"));

    // Verify directory structure
    assert!(dir.path().join(".shape").is_dir());
    assert!(dir.path().join(".shape/briefs").is_dir());
    assert!(dir.path().join(".shape/plugins").is_dir());
    assert!(dir.path().join(".shape/sync").is_dir());
    assert!(dir.path().join(".shape/config.toml").is_file());
    assert!(dir.path().join(".shape/.gitignore").is_file());
}

#[test]
fn test_init_is_idempotent() {
    let dir = TempDir::new().unwrap();

    // First init
    shape_cmd().arg("init").arg(dir.path()).assert().success();

    // Second init should also succeed
    shape_cmd().arg("init").arg(dir.path()).assert().success();
}

// =============================================================================
// Anchor Tests
// =============================================================================

#[test]
fn test_brief_new_creates_brief() {
    let dir = setup_project();

    shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "new", "Test Pitch"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created brief"));

    // Verify brief file was created
    let briefs: Vec<_> = fs::read_dir(dir.path().join(".shape/briefs"))
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
        .collect();

    assert_eq!(briefs.len(), 1);
}

#[test]
fn test_brief_list_shows_briefs() {
    let dir = setup_project();

    // Create an brief
    shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "new", "My Test Anchor"])
        .assert()
        .success();

    // List should show it
    shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("My Test Anchor"));
}

#[test]
fn test_brief_show_displays_details() {
    let dir = setup_project();

    // Create brief and capture the ID
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "new", "Detail Test", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let brief_id = json["id"].as_str().unwrap();

    // Show should display the brief
    shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "show", brief_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Detail Test"));
}

// =============================================================================
// Task Tests
// =============================================================================

#[test]
fn test_task_add_creates_task() {
    let dir = setup_project();

    // Create brief
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "new", "Task Test", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let brief_id = json["id"].as_str().unwrap();

    // Add task
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", brief_id, "My First Task"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"));
}

#[test]
fn test_task_list_shows_tasks() {
    let dir = setup_project();

    // Create brief
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "new", "List Test", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let brief_id = json["id"].as_str().unwrap();

    // Add tasks
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", brief_id, "Task One"])
        .assert()
        .success();

    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", brief_id, "Task Two"])
        .assert()
        .success();

    // List should show both
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "list", brief_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Task One"))
        .stdout(predicate::str::contains("Task Two"));
}

#[test]
fn test_task_start_and_done() {
    let dir = setup_project();

    // Create brief and task
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "new", "Status Test", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let brief_id = json["id"].as_str().unwrap();

    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", brief_id, "Status Task", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let task_id = json["id"].as_str().unwrap();

    // Start task
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "start", task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started task"));

    // Complete task
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "done", task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Completed task"));
}

// =============================================================================
// Dependency Tests
// =============================================================================

#[test]
fn test_task_dependencies() {
    let dir = setup_project();

    // Create brief
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "new", "Dep Test", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let brief_id = json["id"].as_str().unwrap();

    // Create two tasks
    let output1 = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", brief_id, "First Task", "--format", "json"])
        .assert()
        .success();

    let stdout1 = String::from_utf8_lossy(&output1.get_output().stdout);
    let json1: serde_json::Value = serde_json::from_str(&stdout1).unwrap();
    let task1_id = json1["id"].as_str().unwrap();

    let output2 = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", brief_id, "Second Task", "--format", "json"])
        .assert()
        .success();

    let stdout2 = String::from_utf8_lossy(&output2.get_output().stdout);
    let json2: serde_json::Value = serde_json::from_str(&stdout2).unwrap();
    let task2_id = json2["id"].as_str().unwrap();

    // Add dependency: task2 depends on task1
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "dep", task2_id, task1_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("now blocked by"));

    // Check blocked shows task2
    shape_cmd()
        .current_dir(dir.path())
        .args(["blocked"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Second Task"));

    // Check ready shows task1 but not task2
    let ready_output = shape_cmd()
        .current_dir(dir.path())
        .args(["ready"])
        .assert()
        .success();

    let ready_stdout = String::from_utf8_lossy(&ready_output.get_output().stdout);
    assert!(ready_stdout.contains("First Task"));
    assert!(!ready_stdout.contains("Second Task"));
}

// =============================================================================
// Ready/Blocked Query Tests
// =============================================================================

#[test]
fn test_ready_shows_unblocked_tasks() {
    let dir = setup_project();

    // Create brief and tasks
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "new", "Ready Test", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let brief_id = json["id"].as_str().unwrap();

    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", brief_id, "Ready Task"])
        .assert()
        .success();

    // Ready should show the task
    shape_cmd()
        .current_dir(dir.path())
        .args(["ready"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Ready Task"));
}

#[test]
fn test_ready_json_format() {
    let dir = setup_project();

    // Create brief and task
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "new", "JSON Test", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let brief_id = json["id"].as_str().unwrap();

    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", brief_id, "JSON Task"])
        .assert()
        .success();

    // Ready with JSON format
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["ready", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert!(json.is_array());
    let tasks = json.as_array().unwrap();
    assert!(!tasks.is_empty());
    assert!(tasks[0]["title"].as_str().unwrap().contains("JSON Task"));
}

// =============================================================================
// Status Tests
// =============================================================================

#[test]
fn test_status_shows_overview() {
    let dir = setup_project();

    // Create brief and task
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "new", "Status Overview Test", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let brief_id = json["id"].as_str().unwrap();

    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", brief_id, "Status Task"])
        .assert()
        .success();

    // Status should show counts
    shape_cmd()
        .current_dir(dir.path())
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Project Status"))
        .stdout(predicate::str::contains("Briefs:"))
        .stdout(predicate::str::contains("Tasks:"));
}

// =============================================================================
// Context Export Tests
// =============================================================================

#[test]
fn test_context_export() {
    let dir = setup_project();

    // Create brief and task
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "new", "Context Test", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let brief_id = json["id"].as_str().unwrap();

    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", brief_id, "Context Task"])
        .assert()
        .success();

    // Context export
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["context"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert!(json["briefs"].is_array());
    assert!(json["tasks"].is_object());
}

#[test]
fn test_context_compact() {
    let dir = setup_project();

    // Create brief and task
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "new", "Compact Test", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let brief_id = json["id"].as_str().unwrap();

    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", brief_id, "Compact Task"])
        .assert()
        .success();

    // Compact context
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["context", "--compact"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Compact format has specific structure
    assert!(json["briefs"].is_array());
    assert!(json["ready"].is_array());
    assert!(json["in_progress"].is_array());
    assert!(json["blocked"].is_array());
    assert!(json["recently_done"].is_array());
}

// =============================================================================
// Verbose Flag Tests
// =============================================================================

#[test]
fn test_verbose_flag() {
    let dir = setup_project();

    // Verbose should show debug output to stderr
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["--verbose", "status"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    assert!(stderr.contains("[verbose]"));
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_not_in_project_error() {
    let dir = TempDir::new().unwrap();

    // Running commands without init should fail
    shape_cmd()
        .current_dir(dir.path())
        .args(["ready"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not in a shape project"));
}

#[test]
fn test_brief_invalid_id_error() {
    let dir = setup_project();

    // Invalid ID format should fail
    shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "show", "b-nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid brief ID"));
}

#[test]
fn test_brief_not_found_error() {
    let dir = setup_project();

    // Valid ID format but doesn't exist
    shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "show", "b-1234567"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("Not found")));
}

// =============================================================================
// Full Workflow Integration Test
// =============================================================================

#[test]
fn test_full_workflow() {
    let dir = setup_project();

    // 1. Create an brief
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "new", "Full Workflow Test", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let brief_id = json["id"].as_str().unwrap();

    // 2. Add multiple tasks
    let output1 = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", brief_id, "Build API", "--format", "json"])
        .assert()
        .success();
    let task1_id = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(
        &output1.get_output().stdout,
    ))
    .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let output2 = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", brief_id, "Write Tests", "--format", "json"])
        .assert()
        .success();
    let task2_id = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(
        &output2.get_output().stdout,
    ))
    .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let output3 = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", brief_id, "Deploy", "--format", "json"])
        .assert()
        .success();
    let task3_id = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(
        &output3.get_output().stdout,
    ))
    .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // 3. Set up dependencies: Tests depend on API, Deploy depends on Tests
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "dep", &task2_id, &task1_id])
        .assert()
        .success();

    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "dep", &task3_id, &task2_id])
        .assert()
        .success();

    // 4. Verify only API is ready initially
    let ready_output = shape_cmd()
        .current_dir(dir.path())
        .args(["ready", "--format", "json"])
        .assert()
        .success();

    let ready_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&ready_output.get_output().stdout)).unwrap();
    let ready_tasks = ready_json.as_array().unwrap();
    assert_eq!(ready_tasks.len(), 1);
    assert!(ready_tasks[0]["title"]
        .as_str()
        .unwrap()
        .contains("Build API"));

    // 5. Start and complete API task
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "start", &task1_id])
        .assert()
        .success();

    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "done", &task1_id])
        .assert()
        .success();

    // 6. Now Tests should be ready
    let ready_output = shape_cmd()
        .current_dir(dir.path())
        .args(["ready", "--format", "json"])
        .assert()
        .success();

    let ready_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&ready_output.get_output().stdout)).unwrap();
    let ready_tasks = ready_json.as_array().unwrap();
    assert_eq!(ready_tasks.len(), 1);
    assert!(ready_tasks[0]["title"]
        .as_str()
        .unwrap()
        .contains("Write Tests"));

    // 7. Complete remaining tasks
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "done", &task2_id])
        .assert()
        .success();

    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "done", &task3_id])
        .assert()
        .success();

    // 8. Verify status shows all complete
    let status_output = shape_cmd()
        .current_dir(dir.path())
        .args(["status", "--format", "json"])
        .assert()
        .success();

    let status_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&status_output.get_output().stdout)).unwrap();
    assert_eq!(status_json["tasks"]["done"].as_u64().unwrap(), 3);

    // 9. Context should show recently completed
    let context_output = shape_cmd()
        .current_dir(dir.path())
        .args(["context", "--compact"])
        .assert()
        .success();

    let context_json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(
        &context_output.get_output().stdout,
    ))
    .unwrap();
    assert!(context_json["recently_done"].as_array().unwrap().len() >= 3);
}

// =============================================================================
// Standalone Task Tests
// =============================================================================

#[test]
fn test_standalone_task_add() {
    let dir = setup_project();

    // Create a standalone task (no parent)
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", "Fix typo in README"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stdout(predicate::str::contains("t-")); // Standalone tasks have t- prefix

    let _ = output;
}

#[test]
fn test_standalone_task_json_output() {
    let dir = setup_project();

    // Create standalone task with JSON output
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", "Standalone task", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Verify standalone flag is true
    assert!(json["standalone"].as_bool().unwrap());
    // Verify ID starts with t-
    assert!(json["id"].as_str().unwrap().starts_with("t-"));
}

#[test]
fn test_standalone_task_list() {
    let dir = setup_project();

    // Create standalone tasks
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", "Standalone One"])
        .assert()
        .success();

    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", "Standalone Two"])
        .assert()
        .success();

    // List with --standalone flag
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "list", "--standalone"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Standalone One"))
        .stdout(predicate::str::contains("Standalone Two"));
}

#[test]
fn test_standalone_task_show() {
    let dir = setup_project();

    // Create standalone task
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", "Show me", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let task_id = json["id"].as_str().unwrap();

    // Show should display standalone task with Type: Standalone
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "show", task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Type: Standalone"))
        .stdout(predicate::str::contains("Show me"));
}

#[test]
fn test_standalone_task_lifecycle() {
    let dir = setup_project();

    // Create standalone task
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", "Lifecycle test", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let task_id = json["id"].as_str().unwrap();

    // Start task
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "start", task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started task"));

    // Complete task
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "done", task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Completed task"));
}

#[test]
fn test_standalone_task_dependencies() {
    let dir = setup_project();

    // Create two standalone tasks
    let output1 = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", "First standalone", "--format", "json"])
        .assert()
        .success();

    let stdout1 = String::from_utf8_lossy(&output1.get_output().stdout);
    let task1_id = serde_json::from_str::<serde_json::Value>(&stdout1).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let output2 = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", "Second standalone", "--format", "json"])
        .assert()
        .success();

    let stdout2 = String::from_utf8_lossy(&output2.get_output().stdout);
    let task2_id = serde_json::from_str::<serde_json::Value>(&stdout2).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Add dependency: task2 depends on task1
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "dep", &task2_id, &task1_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("now blocked by"));

    // Check blocked shows task2
    shape_cmd()
        .current_dir(dir.path())
        .args(["blocked"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Second standalone"));
}

#[test]
fn test_mixed_briefed_and_standalone_tasks() {
    let dir = setup_project();

    // Create an brief
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "new", "Mixed Test", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let brief_id = json["id"].as_str().unwrap();

    // Add briefed task
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", brief_id, "Brief task"])
        .assert()
        .success()
        .stdout(predicate::str::contains("b-")); // Brief tasks have b- prefix

    // Add standalone task
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", "Standalone task"])
        .assert()
        .success()
        .stdout(predicate::str::contains("t-")); // Standalone tasks have t- prefix

    // List all tasks should show both
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Brief task"))
        .stdout(predicate::str::contains("Standalone task"));

    // Ready should include both types
    let ready_output = shape_cmd()
        .current_dir(dir.path())
        .args(["ready", "--format", "json"])
        .assert()
        .success();

    let ready_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&ready_output.get_output().stdout)).unwrap();
    let ready_tasks = ready_json.as_array().unwrap();
    assert!(ready_tasks.len() >= 2);
}

#[test]
fn test_standalone_subtask() {
    let dir = setup_project();

    // Create standalone parent task
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", "Parent task", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let parent_id = json["id"].as_str().unwrap();

    // Create subtask under standalone task
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", parent_id, "Subtask of standalone"])
        .assert()
        .success()
        .stdout(predicate::str::contains(parent_id)) // Subtask ID includes parent
        .stdout(predicate::str::contains(".1")); // Sequence number
}

#[test]
fn test_status_shows_standalone_count() {
    let dir = setup_project();

    // Create standalone tasks
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", "Standalone for status"])
        .assert()
        .success();

    // Status should show standalone count
    let status_output = shape_cmd()
        .current_dir(dir.path())
        .args(["status", "--format", "json"])
        .assert()
        .success();

    let status_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&status_output.get_output().stdout)).unwrap();

    assert!(status_json["standalone_tasks"].is_object());
    assert!(status_json["standalone_tasks"]["total"].as_u64().unwrap() >= 1);
}

#[test]
fn test_context_includes_standalone_tasks() {
    let dir = setup_project();

    // Create standalone task
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", "Standalone for context"])
        .assert()
        .success();

    // Context should include standalone_tasks section
    let context_output = shape_cmd()
        .current_dir(dir.path())
        .args(["context"])
        .assert()
        .success();

    let context_json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(
        &context_output.get_output().stdout,
    ))
    .unwrap();

    assert!(context_json["standalone_tasks"].is_object());
    assert!(context_json["standalone_tasks"]["ready"].is_array());
}

// =============================================================================
// Compaction Tests
// =============================================================================

#[test]
fn test_compact_dry_run() {
    let dir = setup_project();

    // Create brief
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "new", "Compact Test", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let brief_id = json["id"].as_str().unwrap();

    // Create and complete 3 tasks (minimum for compaction)
    let mut task_ids = Vec::new();
    for i in 1..=3 {
        let output = shape_cmd()
            .current_dir(dir.path())
            .args([
                "task",
                "add",
                brief_id,
                &format!("Auth task {}", i),
                "--format",
                "json",
            ])
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        task_ids.push(json["id"].as_str().unwrap().to_string());
    }

    // Complete all tasks
    for task_id in &task_ids {
        shape_cmd()
            .current_dir(dir.path())
            .args(["task", "done", task_id])
            .assert()
            .success();
    }

    // Run compact with --dry-run and --days 0 (to compact immediately)
    shape_cmd()
        .current_dir(dir.path())
        .args(["compact", "--days", "0", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Would compact"))
        .stdout(predicate::str::contains("3 tasks"));
}

#[test]
fn test_compact_and_context_integration() {
    let dir = setup_project();

    // Create brief
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "new", "Compaction Integration", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let brief_id = json["id"].as_str().unwrap();

    // Create and complete 3 tasks with similar names (for smart summary)
    let mut task_ids = Vec::new();
    for task_name in [
        "Authentication login",
        "Authentication logout",
        "Authentication session",
    ] {
        let output = shape_cmd()
            .current_dir(dir.path())
            .args(["task", "add", brief_id, task_name, "--format", "json"])
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        task_ids.push(json["id"].as_str().unwrap().to_string());
    }

    // Complete all tasks
    for task_id in &task_ids {
        shape_cmd()
            .current_dir(dir.path())
            .args(["task", "done", task_id])
            .assert()
            .success();
    }

    // Run compact (not dry-run) with --days 0
    let compact_output = shape_cmd()
        .current_dir(dir.path())
        .args(["compact", "--days", "0", "--format", "json"])
        .assert()
        .success();

    let compact_json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(
        &compact_output.get_output().stdout,
    ))
    .unwrap();

    assert_eq!(compact_json["compacted"].as_u64().unwrap(), 3);
    assert_eq!(compact_json["groups"].as_array().unwrap().len(), 1);
    let representative_id = compact_json["groups"][0]["representative_id"]
        .as_str()
        .unwrap();

    // Context should now show compacted section instead of recently_done
    let context_output = shape_cmd()
        .current_dir(dir.path())
        .args(["context"])
        .assert()
        .success();

    let context_json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(
        &context_output.get_output().stdout,
    ))
    .unwrap();

    // Verify compacted section exists and has content
    let compacted = context_json["tasks"]["compacted"].as_array().unwrap();
    assert_eq!(compacted.len(), 1);
    assert_eq!(compacted[0]["task_count"].as_u64().unwrap(), 3);

    // Recently completed should not include compacted tasks
    let recently_completed = context_json["tasks"]["recently_completed"]
        .as_array()
        .unwrap();
    assert!(recently_completed.is_empty());

    // Test undo compaction
    shape_cmd()
        .current_dir(dir.path())
        .args(["compact", "--undo", representative_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Undone compaction"))
        .stdout(predicate::str::contains("3 tasks restored"));

    // After undo, context should show tasks in recently_completed again
    let context_after_undo = shape_cmd()
        .current_dir(dir.path())
        .args(["context", "--days", "7"])
        .assert()
        .success();

    let context_undo_json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(
        &context_after_undo.get_output().stdout,
    ))
    .unwrap();

    // Compacted section should be empty after undo
    let compacted_after = context_undo_json["tasks"]["compacted"].as_array().unwrap();
    assert!(compacted_after.is_empty());
}

// =============================================================================
// Daemon Tests
// =============================================================================

#[test]
fn test_daemon_status_not_running() {
    let dir = setup_project();

    // Daemon status should show not running and the project path
    shape_cmd()
        .current_dir(dir.path())
        .args(["daemon", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("STOPPED"))
        .stdout(predicate::str::contains("Project:"));
}

#[test]
fn test_daemon_status_json() {
    let dir = setup_project();

    // Daemon status with JSON output
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["daemon", "status", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert!(!json["running"].as_bool().unwrap());
    assert!(json["config"].is_object());
    assert!(json["config"]["enabled"].as_bool().unwrap());
    assert!(json["project"].is_string()); // Per-project daemon includes project path
}

#[test]
fn test_daemon_stop_when_not_running() {
    let dir = setup_project();

    // Stopping when not running should be graceful
    shape_cmd()
        .current_dir(dir.path())
        .args(["daemon", "stop"])
        .assert()
        .success()
        .stdout(predicate::str::contains("not running for this project"));
}

#[test]
fn test_daemon_logs_no_file() {
    let dir = setup_project();

    // Logs when no log file exists
    shape_cmd()
        .current_dir(dir.path())
        .args(["daemon", "logs"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "No daemon logs found for this project",
        ));
}

#[test]
fn test_daemon_config_in_project() {
    let dir = setup_project();

    // Default config doesn't include daemon section, but it's loaded with defaults
    // Just verify the command can read config
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["daemon", "status", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Check default daemon config values are present
    assert!(json["config"]["enabled"].as_bool().unwrap());
    assert!(json["config"]["auto_commit"].as_bool().unwrap());
    assert!(!json["config"]["auto_push"].as_bool().unwrap());
    assert_eq!(json["config"]["debounce_seconds"].as_u64().unwrap(), 5);
}

// =============================================================================
// Agent Coordination Tests
// =============================================================================

#[test]
fn test_claim_and_unclaim() {
    let dir = setup_project();

    // Create a standalone task
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", "Task to claim", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let task_id = json["id"].as_str().unwrap();

    // Claim the task
    let claim_output = shape_cmd()
        .current_dir(dir.path())
        .args(["claim", task_id, "--format", "json"])
        .assert()
        .success();

    let claim_json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(
        &claim_output.get_output().stdout,
    ))
    .unwrap();
    assert!(claim_json["claimed_by"].is_string());
    assert_eq!(claim_json["status"].as_str().unwrap(), "in_progress");

    // Verify task is now in progress
    let show_output = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "show", task_id, "--format", "json"])
        .assert()
        .success();

    let show_json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(
        &show_output.get_output().stdout,
    ))
    .unwrap();
    assert_eq!(show_json["status"].as_str().unwrap(), "in_progress");

    // Unclaim the task
    shape_cmd()
        .current_dir(dir.path())
        .args(["unclaim", task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Released claim"));
}

#[test]
fn test_claim_force_override() {
    let dir = setup_project();

    // Create a task
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", "Contested task", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let task_id = json["id"].as_str().unwrap();

    // First claim with agent-1
    shape_cmd()
        .current_dir(dir.path())
        .args(["claim", task_id, "--agent", "agent-1"])
        .assert()
        .success();

    // Second claim without force should fail
    shape_cmd()
        .current_dir(dir.path())
        .args(["claim", task_id, "--agent", "agent-2"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("claimed by"));

    // Force claim should succeed with reason
    shape_cmd()
        .current_dir(dir.path())
        .args([
            "claim",
            task_id,
            "--agent",
            "agent-2",
            "--force",
            "--reason",
            "Agent 1 crashed",
        ])
        .assert()
        .success();

    // Verify new claim
    let show_output = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "show", task_id, "--format", "json"])
        .assert()
        .success();

    let show_json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(
        &show_output.get_output().stdout,
    ))
    .unwrap();
    assert_eq!(show_json["claimed_by"].as_str().unwrap(), "agent-2");
}

#[test]
fn test_next_suggests_ready_task() {
    let dir = setup_project();

    // Create brief
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["brief", "new", "Next Test", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let brief_id = json["id"].as_str().unwrap();

    // Create tasks with different priorities
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", brief_id, "Low priority task"])
        .assert()
        .success();

    let high_output = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", brief_id, "High priority task", "--format", "json"])
        .assert()
        .success();

    let high_json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(
        &high_output.get_output().stdout,
    ))
    .unwrap();
    let high_task_id = high_json["id"].as_str().unwrap();

    // Set high priority
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "meta", high_task_id, "priority", "high"])
        .assert()
        .success();

    // Next should recommend the high priority task
    let next_output = shape_cmd()
        .current_dir(dir.path())
        .args(["next", "--format", "json"])
        .assert()
        .success();

    let next_json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(
        &next_output.get_output().stdout,
    ))
    .unwrap();

    assert_eq!(
        next_json["recommended"]["priority"].as_str().unwrap(),
        "high"
    );
}

#[test]
fn test_note_adds_to_task() {
    let dir = setup_project();

    // Create task
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", "Task with notes", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let task_id = json["id"].as_str().unwrap();

    // Add a note
    shape_cmd()
        .current_dir(dir.path())
        .args(["note", task_id, "Found edge case in validation"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added note"));

    // Add another note
    shape_cmd()
        .current_dir(dir.path())
        .args(["note", task_id, "Fixed the edge case"])
        .assert()
        .success();

    // History should show the notes
    shape_cmd()
        .current_dir(dir.path())
        .args(["history", task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Found edge case"))
        .stdout(predicate::str::contains("Fixed the edge case"));
}

#[test]
fn test_link_and_unlink_artifacts() {
    let dir = setup_project();

    // Create task
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", "Task with links", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let task_id = json["id"].as_str().unwrap();

    // Link a commit
    shape_cmd()
        .current_dir(dir.path())
        .args(["link", task_id, "--commit", "abc123"])
        .assert()
        .success()
        .stdout(predicate::str::contains("commit:abc123"));

    // Link a file
    shape_cmd()
        .current_dir(dir.path())
        .args(["link", task_id, "--file", "src/main.rs"])
        .assert()
        .success();

    // History should show the links
    let history_output = shape_cmd()
        .current_dir(dir.path())
        .args(["history", task_id])
        .assert()
        .success();

    let history_stdout = String::from_utf8_lossy(&history_output.get_output().stdout);
    assert!(history_stdout.contains("commit") || history_stdout.contains("abc123"));

    // Unlink the commit
    shape_cmd()
        .current_dir(dir.path())
        .args(["unlink", task_id, "--commit", "abc123"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed"));
}

#[test]
fn test_block_and_unblock() {
    let dir = setup_project();

    // Create task
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", "Blockable task", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let task_id = json["id"].as_str().unwrap();

    // Block the task
    shape_cmd()
        .current_dir(dir.path())
        .args(["block", task_id, "Waiting for API spec"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Blocked"));

    // Next should not suggest blocked task
    let next_output = shape_cmd()
        .current_dir(dir.path())
        .args(["next", "--format", "json"])
        .assert()
        .success();

    let next_stdout = String::from_utf8_lossy(&next_output.get_output().stdout);
    // Either empty or doesn't contain our blocked task
    assert!(
        !next_stdout.contains("Blockable task") || next_stdout.contains("No tasks ready")
    );

    // Unblock the task
    shape_cmd()
        .current_dir(dir.path())
        .args(["unblock", task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Unblocked"));
}

#[test]
fn test_history_shows_timeline() {
    let dir = setup_project();

    // Create task
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", "Task for history", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let task_id = json["id"].as_str().unwrap();

    // Perform several actions
    shape_cmd()
        .current_dir(dir.path())
        .args(["claim", task_id])
        .assert()
        .success();

    shape_cmd()
        .current_dir(dir.path())
        .args(["note", task_id, "Working on it"])
        .assert()
        .success();

    shape_cmd()
        .current_dir(dir.path())
        .args(["unclaim", task_id])
        .assert()
        .success();

    // History should show all events
    let history_output = shape_cmd()
        .current_dir(dir.path())
        .args(["history", task_id, "--format", "json"])
        .assert()
        .success();

    let history_json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(
        &history_output.get_output().stdout,
    ))
    .unwrap();

    let history = history_json["history"].as_array().unwrap();
    assert!(history.len() >= 4); // created, started, claimed, note, unclaimed

    // Verify event types are present
    let event_types: Vec<&str> = history
        .iter()
        .map(|e| e["event"].as_str().unwrap())
        .collect();
    assert!(event_types.contains(&"created"));
    assert!(event_types.contains(&"claimed"));
    assert!(event_types.contains(&"note"));
    assert!(event_types.contains(&"unclaimed"));
}

#[test]
fn test_summary_project() {
    let dir = setup_project();

    // Create some tasks
    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", "Task 1"])
        .assert()
        .success();

    shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", "Task 2"])
        .assert()
        .success();

    // Project summary
    let summary_output = shape_cmd()
        .current_dir(dir.path())
        .args(["summary", "--format", "json"])
        .assert()
        .success();

    let summary_json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(
        &summary_output.get_output().stdout,
    ))
    .unwrap();

    assert!(summary_json["tasks"]["total"].as_u64().unwrap() >= 2);
    assert!(summary_json["tasks"]["ready"].as_u64().unwrap() >= 2);
}

#[test]
fn test_handoff_task() {
    let dir = setup_project();

    // Create and claim task
    let output = shape_cmd()
        .current_dir(dir.path())
        .args(["task", "add", "Task to handoff", "--format", "json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let task_id = json["id"].as_str().unwrap();

    shape_cmd()
        .current_dir(dir.path())
        .args(["claim", task_id, "--agent", "agent-1"])
        .assert()
        .success();

    // Handoff to human
    shape_cmd()
        .current_dir(dir.path())
        .args([
            "handoff",
            task_id,
            "Needs human review for security audit",
            "--to",
            "human",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Handed off"));

    // History should show handoff
    shape_cmd()
        .current_dir(dir.path())
        .args(["history", task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("handoff"))
        .stdout(predicate::str::contains("human"));
}
