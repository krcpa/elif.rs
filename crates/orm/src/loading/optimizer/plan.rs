use crate::{
    error::{OrmError, OrmResult},
    relationships::{metadata::RelationshipMetadata, RelationshipType},
};
use std::collections::{HashMap, HashSet};

/// Represents a node in the query execution plan
#[derive(Debug, Clone)]
pub struct QueryNode {
    /// Unique identifier for this node
    pub id: String,
    /// Table to query
    pub table: String,
    /// Type of relationship from parent
    pub relationship_type: Option<RelationshipType>,
    /// Full relationship metadata if available
    pub relationship_metadata: Option<RelationshipMetadata>,
    /// Parent node ID (None for root)
    pub parent_id: Option<String>,
    /// Child node IDs
    pub children: Vec<String>,
    /// Depth in the relationship tree
    pub depth: usize,
    /// Estimated row count (can be updated from metadata)
    pub estimated_rows: usize,
    /// Whether this node can be executed in parallel with siblings
    pub parallel_safe: bool,
    /// Foreign key used to join with parent
    pub foreign_key: Option<String>,
    /// Additional constraints for optimization
    pub constraints: Vec<String>,
    /// Column names available in this table (for better query construction)
    pub available_columns: Vec<String>,
    /// Index hints for the optimizer
    pub index_hints: Vec<String>,
}

impl QueryNode {
    /// Create a new query node
    pub fn new(id: String, table: String) -> Self {
        Self {
            id,
            table,
            relationship_type: None,
            relationship_metadata: None,
            parent_id: None,
            children: Vec::new(),
            depth: 0,
            estimated_rows: 1000, // Default estimate
            parallel_safe: true,
            foreign_key: None,
            constraints: Vec::new(),
            available_columns: Vec::new(),
            index_hints: Vec::new(),
        }
    }

    /// Create a root node (no parent)
    pub fn root(id: String, table: String) -> Self {
        let mut node = Self::new(id, table);
        node.depth = 0;
        node
    }

    /// Create a child node with parent relationship
    pub fn child(
        id: String,
        table: String,
        parent_id: String,
        relationship_type: RelationshipType,
        foreign_key: String,
    ) -> Self {
        let mut node = Self::new(id, table);
        node.parent_id = Some(parent_id);
        node.relationship_type = Some(relationship_type);
        node.foreign_key = Some(foreign_key);
        node
    }

    /// Create a child node with full relationship metadata
    pub fn child_with_metadata(
        id: String,
        table: String,
        parent_id: String,
        metadata: RelationshipMetadata,
    ) -> Self {
        let mut node = Self::new(id, table);
        node.parent_id = Some(parent_id);
        node.relationship_type = Some(metadata.relationship_type);
        node.relationship_metadata = Some(metadata.clone());
        node.foreign_key = Some(metadata.foreign_key.primary_column().to_string());

        // Set estimated rows based on relationship type
        node.estimated_rows = match metadata.relationship_type {
            RelationshipType::HasMany
            | RelationshipType::ManyToMany
            | RelationshipType::MorphMany => 10000,
            _ => 1, // HasOne, BelongsTo, MorphOne, MorphTo
        };

        // Set parallel safety based on relationship characteristics
        node.parallel_safe = !metadata.relationship_type.requires_pivot();

        node
    }

    /// Add a child node ID
    pub fn add_child(&mut self, child_id: String) {
        if !self.children.contains(&child_id) {
            self.children.push(child_id);
        }
    }

    /// Set the depth for this node
    pub fn set_depth(&mut self, depth: usize) {
        self.depth = depth;
    }

    /// Set row count estimate
    pub fn set_estimated_rows(&mut self, rows: usize) {
        self.estimated_rows = rows;
    }

    /// Set parallel safety
    pub fn set_parallel_safe(&mut self, safe: bool) {
        self.parallel_safe = safe;
    }

    /// Add a constraint
    pub fn add_constraint(&mut self, constraint: String) {
        self.constraints.push(constraint);
    }

    /// Check if this node is a root node
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }

    /// Check if this node is a leaf node
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Update metadata for this node
    pub fn set_metadata(&mut self, metadata: RelationshipMetadata) {
        self.relationship_type = Some(metadata.relationship_type);
        self.relationship_metadata = Some(metadata.clone());
        self.foreign_key = Some(metadata.foreign_key.primary_column().to_string());

        // Update estimates based on metadata
        self.estimated_rows = match metadata.relationship_type {
            RelationshipType::HasMany
            | RelationshipType::ManyToMany
            | RelationshipType::MorphMany => 10000,
            _ => 1,
        };

        self.parallel_safe = !metadata.relationship_type.requires_pivot();
    }

    /// Set column information for better query construction
    pub fn set_columns(&mut self, columns: Vec<String>) {
        self.available_columns = columns;
    }

    /// Add index hints for optimization
    pub fn add_index_hint(&mut self, index: String) {
        if !self.index_hints.contains(&index) {
            self.index_hints.push(index);
        }
    }

    /// Get the primary key column name (defaults to "id")
    pub fn primary_key(&self) -> &str {
        if let Some(metadata) = &self.relationship_metadata {
            &metadata.local_key
        } else {
            "id"
        }
    }

    /// Get the foreign key column name for relationships
    pub fn get_foreign_key(&self) -> Option<&str> {
        self.foreign_key.as_deref()
    }

    /// Check if this node represents a collection relationship
    pub fn is_collection(&self) -> bool {
        self.relationship_type
            .map(|rt| rt.is_collection())
            .unwrap_or(false)
    }

    /// Check if this node requires a pivot table
    pub fn requires_pivot(&self) -> bool {
        self.relationship_type
            .map(|rt| rt.requires_pivot())
            .unwrap_or(false)
    }
}

/// Query execution plan for optimized loading
#[derive(Debug)]
pub struct QueryPlan {
    /// All nodes in the plan, keyed by ID
    pub nodes: HashMap<String, QueryNode>,
    /// Root node IDs (entry points)
    pub roots: Vec<String>,
    /// Execution phases (nodes that can run in parallel)
    pub execution_phases: Vec<Vec<String>>,
    /// Maximum depth of the plan
    pub max_depth: usize,
    /// Total estimated rows
    pub total_estimated_rows: usize,
    /// Plan metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl QueryPlan {
    /// Create an empty query plan
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            roots: Vec::new(),
            execution_phases: Vec::new(),
            max_depth: 0,
            total_estimated_rows: 0,
            metadata: HashMap::new(),
        }
    }

    /// Add a node to the plan
    pub fn add_node(&mut self, node: QueryNode) {
        self.max_depth = self.max_depth.max(node.depth);
        self.total_estimated_rows += node.estimated_rows;

        if node.parent_id.is_none() {
            self.roots.push(node.id.clone());
        }

        // Update parent-child relationships
        if let Some(parent_id) = &node.parent_id {
            if let Some(parent) = self.nodes.get_mut(parent_id) {
                parent.add_child(node.id.clone());
            }
        }

        self.nodes.insert(node.id.clone(), node);
    }

    /// Build execution phases (groups of nodes that can run in parallel)
    pub fn build_execution_phases(&mut self) -> OrmResult<()> {
        let mut phases = Vec::new();
        let mut visited = HashSet::new();
        let mut current_depth = 0;

        // Validate plan before building phases
        self.validate()?;

        while visited.len() < self.nodes.len() && current_depth <= self.max_depth {
            let mut phase_nodes = Vec::new();

            // Find all nodes at the current depth that haven't been visited
            for (id, node) in &self.nodes {
                if !visited.contains(id) && node.depth == current_depth {
                    // Check if all parent nodes have been visited
                    let ready = if let Some(parent_id) = &node.parent_id {
                        visited.contains(parent_id)
                    } else {
                        true // Root nodes are always ready
                    };

                    if ready && node.parallel_safe {
                        phase_nodes.push(id.clone());
                    }
                }
            }

            // If no parallel-safe nodes found, add sequential nodes
            if phase_nodes.is_empty() {
                for (id, node) in &self.nodes {
                    if !visited.contains(id) && node.depth == current_depth {
                        let ready = if let Some(parent_id) = &node.parent_id {
                            visited.contains(parent_id)
                        } else {
                            true
                        };

                        if ready {
                            phase_nodes.push(id.clone());
                            break; // Only one sequential node per phase
                        }
                    }
                }
            }

            if phase_nodes.is_empty() {
                current_depth += 1;
                continue;
            }

            // Mark phase nodes as visited
            for id in &phase_nodes {
                visited.insert(id.clone());
            }

            if !phase_nodes.is_empty() {
                phases.push(phase_nodes);
            }

            current_depth += 1;
        }

        self.execution_phases = phases;
        Ok(())
    }

    /// Validate the query plan for consistency
    pub fn validate(&self) -> OrmResult<()> {
        // Check for circular dependencies
        for root_id in &self.roots {
            let mut visited = HashSet::new();
            let mut path = Vec::new();

            if self.has_cycle_from(root_id, &mut visited, &mut path) {
                return Err(OrmError::Query(
                    "Circular dependency detected in query plan".into(),
                ));
            }
        }

        // Validate parent-child relationships
        for (id, node) in &self.nodes {
            if let Some(parent_id) = &node.parent_id {
                if !self.nodes.contains_key(parent_id) {
                    return Err(OrmError::Query(format!(
                        "Parent node '{}' not found for node '{}'",
                        parent_id, id
                    )));
                }
            }

            for child_id in &node.children {
                if !self.nodes.contains_key(child_id) {
                    return Err(OrmError::Query(format!(
                        "Child node '{}' not found for node '{}'",
                        child_id, id
                    )));
                }
            }
        }

        Ok(())
    }

    /// DFS helper for cycle detection
    fn has_cycle_from(
        &self,
        node_id: &str,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> bool {
        if path.contains(&node_id.to_string()) {
            return true; // Found a cycle
        }

        if visited.contains(node_id) {
            return false; // Already processed this subtree
        }

        path.push(node_id.to_string());
        visited.insert(node_id.to_string());

        if let Some(node) = self.nodes.get(node_id) {
            for child_id in &node.children {
                if self.has_cycle_from(child_id, visited, path) {
                    return true;
                }
            }
        }

        path.pop();
        false
    }

    /// Get nodes at a specific depth
    pub fn nodes_at_depth(&self, depth: usize) -> Vec<&QueryNode> {
        self.nodes
            .values()
            .filter(|node| node.depth == depth)
            .collect()
    }

    /// Get all leaf nodes
    pub fn leaf_nodes(&self) -> Vec<&QueryNode> {
        self.nodes.values().filter(|node| node.is_leaf()).collect()
    }

    /// Calculate plan complexity score
    pub fn complexity_score(&self) -> f64 {
        let depth_penalty = self.max_depth as f64 * 1.5;
        let node_penalty = self.nodes.len() as f64 * 0.5;
        let row_penalty = (self.total_estimated_rows as f64).log10() * 2.0;

        depth_penalty + node_penalty + row_penalty
    }

    /// Add metadata to the plan
    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
    }

    /// Get plan statistics
    pub fn statistics(&self) -> PlanStatistics {
        PlanStatistics {
            total_nodes: self.nodes.len(),
            root_nodes: self.roots.len(),
            leaf_nodes: self.leaf_nodes().len(),
            max_depth: self.max_depth,
            total_estimated_rows: self.total_estimated_rows,
            execution_phases: self.execution_phases.len(),
            complexity_score: self.complexity_score(),
            parallel_nodes: self.nodes.values().filter(|n| n.parallel_safe).count(),
        }
    }
}

impl Default for QueryPlan {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about a query plan
#[derive(Debug, Clone)]
pub struct PlanStatistics {
    pub total_nodes: usize,
    pub root_nodes: usize,
    pub leaf_nodes: usize,
    pub max_depth: usize,
    pub total_estimated_rows: usize,
    pub execution_phases: usize,
    pub complexity_score: f64,
    pub parallel_nodes: usize,
}
