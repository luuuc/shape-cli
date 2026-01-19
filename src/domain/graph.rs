//! Dependency graph for tasks
//!
//! Manages task dependencies with cycle detection and topological ordering.
//! Uses petgraph for graph operations.

use petgraph::algo::{is_cyclic_directed, toposort};
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;
use thiserror::Error;

use super::id::TaskId;
use super::task::{Task, TaskStatus};

#[derive(Debug, Error, PartialEq)]
pub enum GraphError {
    #[error("Adding dependency would create a cycle: {0} -> {1}")]
    CycleDetected(TaskId, TaskId),

    #[error("Task not found: {0}")]
    TaskNotFound(TaskId),

    #[error("Self-dependency not allowed: {0}")]
    SelfDependency(TaskId),
}

/// A dependency graph for tasks
#[derive(Debug, Default)]
pub struct DependencyGraph {
    /// The underlying directed graph
    graph: DiGraph<TaskId, ()>,

    /// Map from TaskId to node index
    node_map: HashMap<TaskId, NodeIndex>,
}

impl DependencyGraph {
    /// Creates an empty dependency graph
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    /// Builds a graph from a collection of tasks
    pub fn from_tasks<'a>(tasks: impl IntoIterator<Item = &'a Task>) -> Result<Self, GraphError> {
        let mut graph = Self::new();

        // First pass: add all nodes
        let tasks: Vec<_> = tasks.into_iter().collect();
        for task in &tasks {
            graph.add_task(task.id.clone());
        }

        // Second pass: add all edges
        for task in &tasks {
            for dep_id in &task.depends_on {
                graph.add_dependency(&task.id, dep_id)?;
            }
        }

        Ok(graph)
    }

    /// Adds a task to the graph
    pub fn add_task(&mut self, task_id: TaskId) {
        if !self.node_map.contains_key(&task_id) {
            let idx = self.graph.add_node(task_id.clone());
            self.node_map.insert(task_id, idx);
        }
    }

    /// Removes a task from the graph (and all its edges)
    pub fn remove_task(&mut self, task_id: &TaskId) -> bool {
        if let Some(idx) = self.node_map.remove(task_id) {
            self.graph.remove_node(idx);
            // Note: petgraph may reuse indices, so we need to rebuild the map
            self.rebuild_node_map();
            true
        } else {
            false
        }
    }

    /// Rebuilds the node map after removal
    fn rebuild_node_map(&mut self) {
        self.node_map.clear();
        for idx in self.graph.node_indices() {
            if let Some(task_id) = self.graph.node_weight(idx) {
                self.node_map.insert(task_id.clone(), idx);
            }
        }
    }

    /// Adds a dependency edge: `task` depends on `depends_on`
    ///
    /// The edge direction is: depends_on -> task
    /// This means "depends_on must be completed before task"
    pub fn add_dependency(&mut self, task: &TaskId, depends_on: &TaskId) -> Result<(), GraphError> {
        if task == depends_on {
            return Err(GraphError::SelfDependency(task.clone()));
        }

        let task_idx = self
            .node_map
            .get(task)
            .ok_or_else(|| GraphError::TaskNotFound(task.clone()))?;

        let dep_idx = self
            .node_map
            .get(depends_on)
            .ok_or_else(|| GraphError::TaskNotFound(depends_on.clone()))?;

        // Add edge: depends_on -> task
        self.graph.add_edge(*dep_idx, *task_idx, ());

        // Check for cycles
        if is_cyclic_directed(&self.graph) {
            // Remove the edge we just added
            if let Some(edge) = self.graph.find_edge(*dep_idx, *task_idx) {
                self.graph.remove_edge(edge);
            }
            return Err(GraphError::CycleDetected(task.clone(), depends_on.clone()));
        }

        Ok(())
    }

    /// Removes a dependency edge
    pub fn remove_dependency(&mut self, task: &TaskId, depends_on: &TaskId) -> bool {
        let task_idx = match self.node_map.get(task) {
            Some(idx) => *idx,
            None => return false,
        };

        let dep_idx = match self.node_map.get(depends_on) {
            Some(idx) => *idx,
            None => return false,
        };

        if let Some(edge) = self.graph.find_edge(dep_idx, task_idx) {
            self.graph.remove_edge(edge);
            true
        } else {
            false
        }
    }

    /// Returns tasks that are ready (no incomplete dependencies)
    pub fn ready_tasks(&self, statuses: &HashMap<TaskId, TaskStatus>) -> Vec<TaskId> {
        self.node_map
            .keys()
            .filter(|task_id| {
                // Task must not be complete
                let status = statuses.get(*task_id).copied().unwrap_or_default();
                if status.is_complete() {
                    return false;
                }

                // All dependencies must be complete
                self.dependencies(task_id).iter().all(|dep_id| {
                    statuses
                        .get(dep_id)
                        .map(|s| s.is_complete())
                        .unwrap_or(false)
                })
            })
            .cloned()
            .collect()
    }

    /// Returns tasks that are blocked (have incomplete dependencies)
    pub fn blocked_tasks(&self, statuses: &HashMap<TaskId, TaskStatus>) -> Vec<TaskId> {
        self.node_map
            .keys()
            .filter(|task_id| {
                // Task must not be complete
                let status = statuses.get(*task_id).copied().unwrap_or_default();
                if status.is_complete() {
                    return false;
                }

                // At least one dependency is not complete
                self.dependencies(task_id).iter().any(|dep_id| {
                    statuses
                        .get(dep_id)
                        .map(|s| !s.is_complete())
                        .unwrap_or(true)
                })
            })
            .cloned()
            .collect()
    }

    /// Returns the direct dependencies of a task
    pub fn dependencies(&self, task_id: &TaskId) -> Vec<TaskId> {
        let task_idx = match self.node_map.get(task_id) {
            Some(idx) => *idx,
            None => return vec![],
        };

        self.graph
            .neighbors_directed(task_idx, petgraph::Direction::Incoming)
            .filter_map(|idx| self.graph.node_weight(idx).cloned())
            .collect()
    }

    /// Returns the direct dependents of a task (tasks that depend on it)
    pub fn dependents(&self, task_id: &TaskId) -> Vec<TaskId> {
        let task_idx = match self.node_map.get(task_id) {
            Some(idx) => *idx,
            None => return vec![],
        };

        self.graph
            .neighbors_directed(task_idx, petgraph::Direction::Outgoing)
            .filter_map(|idx| self.graph.node_weight(idx).cloned())
            .collect()
    }

    /// Returns all tasks in topological order (dependencies before dependents)
    pub fn topological_order(&self) -> Result<Vec<TaskId>, GraphError> {
        match toposort(&self.graph, None) {
            Ok(order) => Ok(order
                .into_iter()
                .filter_map(|idx| self.graph.node_weight(idx).cloned())
                .collect()),
            Err(_) => {
                // This shouldn't happen if we maintain acyclicity
                Err(GraphError::CycleDetected(
                    TaskId::new(&super::id::AnchorId::new("cycle", chrono::Utc::now()), 0),
                    TaskId::new(&super::id::AnchorId::new("cycle", chrono::Utc::now()), 0),
                ))
            }
        }
    }

    /// Returns true if the graph contains the task
    pub fn contains(&self, task_id: &TaskId) -> bool {
        self.node_map.contains_key(task_id)
    }

    /// Returns the number of tasks in the graph
    pub fn len(&self) -> usize {
        self.node_map.len()
    }

    /// Returns true if the graph is empty
    pub fn is_empty(&self) -> bool {
        self.node_map.is_empty()
    }

    /// Returns all task IDs in the graph
    pub fn task_ids(&self) -> impl Iterator<Item = &TaskId> {
        self.node_map.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_task_id(seq: u32) -> TaskId {
        let anchor = super::super::id::AnchorId::new("Test", Utc::now());
        TaskId::new(&anchor, seq)
    }

    #[test]
    fn empty_graph() {
        let graph = DependencyGraph::new();
        assert!(graph.is_empty());
        assert_eq!(graph.len(), 0);
    }

    #[test]
    fn add_tasks() {
        let mut graph = DependencyGraph::new();
        let id1 = make_task_id(1);
        let id2 = make_task_id(2);

        graph.add_task(id1.clone());
        graph.add_task(id2.clone());

        assert_eq!(graph.len(), 2);
        assert!(graph.contains(&id1));
        assert!(graph.contains(&id2));
    }

    #[test]
    fn add_dependency() {
        let mut graph = DependencyGraph::new();
        let id1 = make_task_id(1);
        let id2 = make_task_id(2);

        graph.add_task(id1.clone());
        graph.add_task(id2.clone());

        // id2 depends on id1
        graph.add_dependency(&id2, &id1).unwrap();

        assert_eq!(graph.dependencies(&id2), vec![id1.clone()]);
        assert_eq!(graph.dependents(&id1), vec![id2.clone()]);
    }

    #[test]
    fn cycle_detection() {
        let mut graph = DependencyGraph::new();
        let id1 = make_task_id(1);
        let id2 = make_task_id(2);
        let id3 = make_task_id(3);

        graph.add_task(id1.clone());
        graph.add_task(id2.clone());
        graph.add_task(id3.clone());

        // id2 depends on id1
        graph.add_dependency(&id2, &id1).unwrap();
        // id3 depends on id2
        graph.add_dependency(&id3, &id2).unwrap();
        // id1 depends on id3 would create a cycle
        let result = graph.add_dependency(&id1, &id3);

        assert!(matches!(result, Err(GraphError::CycleDetected(_, _))));
    }

    #[test]
    fn self_dependency_rejected() {
        let mut graph = DependencyGraph::new();
        let id1 = make_task_id(1);

        graph.add_task(id1.clone());

        let result = graph.add_dependency(&id1, &id1);
        assert!(matches!(result, Err(GraphError::SelfDependency(_))));
    }

    #[test]
    fn ready_tasks() {
        let mut graph = DependencyGraph::new();
        let id1 = make_task_id(1);
        let id2 = make_task_id(2);
        let id3 = make_task_id(3);

        graph.add_task(id1.clone());
        graph.add_task(id2.clone());
        graph.add_task(id3.clone());

        // id2 depends on id1, id3 is independent
        graph.add_dependency(&id2, &id1).unwrap();

        let mut statuses = HashMap::new();
        statuses.insert(id1.clone(), TaskStatus::Todo);
        statuses.insert(id2.clone(), TaskStatus::Todo);
        statuses.insert(id3.clone(), TaskStatus::Todo);

        // id1 and id3 are ready (no deps), id2 is blocked
        let ready = graph.ready_tasks(&statuses);
        assert!(ready.contains(&id1));
        assert!(ready.contains(&id3));
        assert!(!ready.contains(&id2));

        // Complete id1
        statuses.insert(id1.clone(), TaskStatus::Done);

        let ready = graph.ready_tasks(&statuses);
        assert!(!ready.contains(&id1)); // completed tasks are not ready
        assert!(ready.contains(&id2)); // now ready
        assert!(ready.contains(&id3));
    }

    #[test]
    fn blocked_tasks() {
        let mut graph = DependencyGraph::new();
        let id1 = make_task_id(1);
        let id2 = make_task_id(2);

        graph.add_task(id1.clone());
        graph.add_task(id2.clone());
        graph.add_dependency(&id2, &id1).unwrap();

        let mut statuses = HashMap::new();
        statuses.insert(id1.clone(), TaskStatus::Todo);
        statuses.insert(id2.clone(), TaskStatus::Todo);

        let blocked = graph.blocked_tasks(&statuses);
        assert!(blocked.contains(&id2));
        assert!(!blocked.contains(&id1));
    }

    #[test]
    fn topological_order() {
        let mut graph = DependencyGraph::new();
        let id1 = make_task_id(1);
        let id2 = make_task_id(2);
        let id3 = make_task_id(3);

        graph.add_task(id1.clone());
        graph.add_task(id2.clone());
        graph.add_task(id3.clone());

        // id3 -> id2 -> id1 (id1 depends on id2, id2 depends on id3)
        graph.add_dependency(&id1, &id2).unwrap();
        graph.add_dependency(&id2, &id3).unwrap();

        let order = graph.topological_order().unwrap();

        // id3 should come before id2, id2 before id1
        let pos3 = order.iter().position(|id| id == &id3).unwrap();
        let pos2 = order.iter().position(|id| id == &id2).unwrap();
        let pos1 = order.iter().position(|id| id == &id1).unwrap();

        assert!(pos3 < pos2);
        assert!(pos2 < pos1);
    }

    #[test]
    fn remove_task() {
        let mut graph = DependencyGraph::new();
        let id1 = make_task_id(1);
        let id2 = make_task_id(2);

        graph.add_task(id1.clone());
        graph.add_task(id2.clone());
        graph.add_dependency(&id2, &id1).unwrap();

        assert!(graph.remove_task(&id1));
        assert!(!graph.contains(&id1));
        assert!(graph.contains(&id2));
        assert!(graph.dependencies(&id2).is_empty());
    }

    #[test]
    fn remove_dependency() {
        let mut graph = DependencyGraph::new();
        let id1 = make_task_id(1);
        let id2 = make_task_id(2);

        graph.add_task(id1.clone());
        graph.add_task(id2.clone());
        graph.add_dependency(&id2, &id1).unwrap();

        assert!(graph.remove_dependency(&id2, &id1));
        assert!(graph.dependencies(&id2).is_empty());
    }

    #[test]
    fn from_tasks() {
        let anchor = super::super::id::AnchorId::new("Test", Utc::now());
        let id1 = TaskId::new(&anchor, 1);
        let id2 = TaskId::new(&anchor, 2);

        let task1 = Task::new(id1.clone(), "Task 1");
        let mut task2 = Task::new(id2.clone(), "Task 2");
        task2.add_dependency(id1.clone());

        let graph = DependencyGraph::from_tasks([&task1, &task2]).unwrap();

        assert_eq!(graph.len(), 2);
        assert_eq!(graph.dependencies(&id2), vec![id1]);
    }

    #[test]
    fn unknown_task_returns_error() {
        let mut graph = DependencyGraph::new();
        let id1 = make_task_id(1);
        let id2 = make_task_id(2);

        graph.add_task(id1.clone());

        let result = graph.add_dependency(&id1, &id2);
        assert!(matches!(result, Err(GraphError::TaskNotFound(_))));
    }

    #[test]
    fn performance_500_tasks() {
        use std::time::Instant;

        let mut graph = DependencyGraph::new();
        let anchor = super::super::id::AnchorId::new("Perf", Utc::now());

        // Create 500 tasks
        let task_ids: Vec<_> = (1..=500).map(|i| TaskId::new(&anchor, i)).collect();

        for id in &task_ids {
            graph.add_task(id.clone());
        }

        // Create a linear dependency chain
        for i in 1..500 {
            graph
                .add_dependency(&task_ids[i], &task_ids[i - 1])
                .unwrap();
        }

        let mut statuses = HashMap::new();
        for id in &task_ids {
            statuses.insert(id.clone(), TaskStatus::Todo);
        }

        let start = Instant::now();
        let _ready = graph.ready_tasks(&statuses);
        let duration = start.elapsed();

        assert!(duration.as_millis() < 10, "Ready query took {:?}", duration);
    }
}
