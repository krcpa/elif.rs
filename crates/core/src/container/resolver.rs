use std::collections::{HashMap, HashSet, VecDeque};
use crate::container::descriptor::{ServiceId, ServiceDescriptor};
use crate::errors::CoreError;

/// Dependency resolution path for error reporting
#[derive(Debug, Clone)]
pub struct ResolutionPath {
    pub services: Vec<ServiceId>,
}

impl ResolutionPath {
    /// Create a new resolution path
    pub fn new() -> Self {
        Self {
            services: Vec::new(),
        }
    }
    
    /// Add a service to the resolution path
    pub fn push(&mut self, service_id: ServiceId) {
        self.services.push(service_id);
    }
    
    /// Remove the last service from the resolution path
    pub fn pop(&mut self) -> Option<ServiceId> {
        self.services.pop()
    }
    
    /// Check if the path contains a service (for cycle detection)
    pub fn contains(&self, service_id: &ServiceId) -> bool {
        self.services.contains(service_id)
    }
    
    /// Get the path as a string for error messages
    pub fn path_string(&self) -> String {
        self.services
            .iter()
            .map(|id| format!("{}({})", id.type_name(), id.name.as_deref().unwrap_or("default")))
            .collect::<Vec<_>>()
            .join(" -> ")
    }
}

/// Dependency graph node
#[derive(Debug)]
pub struct DependencyNode {
    pub service_id: ServiceId,
    pub dependencies: Vec<ServiceId>,
    pub dependents: Vec<ServiceId>,
}

/// Dependency graph for analyzing service relationships
#[derive(Debug)]
pub struct DependencyGraph {
    nodes: HashMap<ServiceId, DependencyNode>,
}

impl DependencyGraph {
    /// Create a new dependency graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }
    
    /// Build dependency graph from service descriptors
    pub fn build_from_descriptors(descriptors: &[ServiceDescriptor]) -> Self {
        let mut graph = Self::new();
        
        // First pass: create all nodes
        for descriptor in descriptors {
            graph.add_service(&descriptor.service_id, &descriptor.dependencies);
        }
        
        // Second pass: build reverse dependencies
        graph.build_reverse_dependencies();
        
        graph
    }
    
    /// Add a service to the graph
    pub fn add_service(&mut self, service_id: &ServiceId, dependencies: &[ServiceId]) {
        let node = DependencyNode {
            service_id: service_id.clone(),
            dependencies: dependencies.to_vec(),
            dependents: Vec::new(),
        };
        self.nodes.insert(service_id.clone(), node);
    }
    
    /// Build reverse dependency relationships
    fn build_reverse_dependencies(&mut self) {
        let dependencies: Vec<(ServiceId, Vec<ServiceId>)> = self.nodes
            .iter()
            .map(|(id, node)| (id.clone(), node.dependencies.clone()))
            .collect();
        
        for (service_id, deps) in dependencies {
            for dep_id in deps {
                if let Some(dep_node) = self.nodes.get_mut(&dep_id) {
                    dep_node.dependents.push(service_id.clone());
                }
            }
        }
    }
    
    /// Detect circular dependencies
    pub fn detect_cycles(&self) -> Result<(), CoreError> {
        let mut visited = HashSet::new();
        let mut in_progress = HashSet::new();
        
        for service_id in self.nodes.keys() {
            if !visited.contains(service_id) {
                let mut path = ResolutionPath::new();
                self.detect_cycle_dfs(service_id, &mut visited, &mut in_progress, &mut path)?;
            }
        }
        
        Ok(())
    }
    
    /// DFS-based cycle detection
    fn detect_cycle_dfs(
        &self,
        service_id: &ServiceId,
        visited: &mut HashSet<ServiceId>,
        in_progress: &mut HashSet<ServiceId>,
        path: &mut ResolutionPath,
    ) -> Result<(), CoreError> {
        if in_progress.contains(service_id) {
            path.push(service_id.clone());
            return Err(CoreError::CircularDependency {
                path: path.path_string(),
                cycle_service: format!("{}({})", 
                    service_id.type_name(), 
                    service_id.name.as_deref().unwrap_or("default")
                ),
            });
        }
        
        if visited.contains(service_id) {
            return Ok(());
        }
        
        in_progress.insert(service_id.clone());
        path.push(service_id.clone());
        
        if let Some(node) = self.nodes.get(service_id) {
            for dep_id in &node.dependencies {
                self.detect_cycle_dfs(dep_id, visited, in_progress, path)?;
            }
        }
        
        path.pop();
        in_progress.remove(service_id);
        visited.insert(service_id.clone());
        
        Ok(())
    }
    
    /// Get topological order for dependency resolution
    pub fn topological_sort(&self) -> Result<Vec<ServiceId>, CoreError> {
        self.detect_cycles()?;
        
        let mut in_degree: HashMap<ServiceId, usize> = HashMap::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();
        
        // Calculate in-degrees
        for (service_id, node) in &self.nodes {
            in_degree.insert(service_id.clone(), node.dependencies.len());
            if node.dependencies.is_empty() {
                queue.push_back(service_id.clone());
            }
        }
        
        // Process queue
        while let Some(service_id) = queue.pop_front() {
            result.push(service_id.clone());
            
            if let Some(node) = self.nodes.get(&service_id) {
                for dependent_id in &node.dependents {
                    if let Some(degree) = in_degree.get_mut(dependent_id) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dependent_id.clone());
                        }
                    }
                }
            }
        }
        
        if result.len() != self.nodes.len() {
            return Err(CoreError::CircularDependency {
                path: "Complex circular dependency detected".to_string(),
                cycle_service: "Multiple services".to_string(),
            });
        }
        
        Ok(result)
    }
    
    /// Get dependencies for a service
    pub fn get_dependencies(&self, service_id: &ServiceId) -> Option<&[ServiceId]> {
        self.nodes.get(service_id).map(|node| node.dependencies.as_slice())
    }
    
    /// Get dependents for a service
    pub fn get_dependents(&self, service_id: &ServiceId) -> Option<&[ServiceId]> {
        self.nodes.get(service_id).map(|node| node.dependents.as_slice())
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Dependency resolver for managing service resolution
#[derive(Debug)]
pub struct DependencyResolver {
    graph: DependencyGraph,
    resolution_order: Vec<ServiceId>,
}

impl DependencyResolver {
    /// Create a new dependency resolver
    pub fn new(descriptors: &[ServiceDescriptor]) -> Result<Self, CoreError> {
        let graph = DependencyGraph::build_from_descriptors(descriptors);
        let resolution_order = graph.topological_sort()?;
        
        Ok(Self {
            graph,
            resolution_order,
        })
    }
    
    /// Get the resolution order for services
    pub fn resolution_order(&self) -> &[ServiceId] {
        &self.resolution_order
    }
    
    /// Get dependencies for a service
    pub fn get_dependencies(&self, service_id: &ServiceId) -> Option<&[ServiceId]> {
        self.graph.get_dependencies(service_id)
    }
    
    /// Validate that all dependencies can be satisfied
    pub fn validate_dependencies(&self, available_services: &HashSet<ServiceId>) -> Result<(), CoreError> {
        for service_id in &self.resolution_order {
            if let Some(dependencies) = self.get_dependencies(service_id) {
                for dep_id in dependencies {
                    if !available_services.contains(dep_id) {
                        return Err(CoreError::ServiceNotFound {
                            service_type: format!("{}({})", 
                                dep_id.type_name(),
                                dep_id.name.as_deref().unwrap_or("default")
                            ),
                        });
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;

    #[test]
    fn test_service_id_creation() {
        let id1 = ServiceId::of::<String>();
        let id2 = ServiceId::named::<String>("cache");
        
        assert_eq!(id1.type_id, TypeId::of::<String>());
        assert_eq!(id1.name, None);
        
        assert_eq!(id2.type_id, TypeId::of::<String>());
        assert_eq!(id2.name, Some("cache".to_string()));
        
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_dependency_graph_cycle_detection() {
        let mut graph = DependencyGraph::new();
        
        let service_a = ServiceId::named::<String>("A");
        let service_b = ServiceId::named::<String>("B");
        let service_c = ServiceId::named::<String>("C");
        
        // Create cycle: A -> B -> C -> A
        graph.add_service(&service_a, &[service_b.clone()]);
        graph.add_service(&service_b, &[service_c.clone()]);
        graph.add_service(&service_c, &[service_a.clone()]);
        
        graph.build_reverse_dependencies();
        
        let result = graph.detect_cycles();
        assert!(result.is_err());
        
        if let Err(CoreError::CircularDependency { path, .. }) = result {
            assert!(path.contains("A") && path.contains("B") && path.contains("C"));
        }
    }

    #[test]
    fn test_dependency_graph_topological_sort() {
        let mut graph = DependencyGraph::new();
        
        let service_a = ServiceId::named::<String>("A");
        let service_b = ServiceId::named::<String>("B");
        let service_c = ServiceId::named::<String>("C");
        
        // Create valid dependency chain: C -> B -> A (A depends on B, B depends on C)
        graph.add_service(&service_c, &[]);
        graph.add_service(&service_b, &[service_c.clone()]);
        graph.add_service(&service_a, &[service_b.clone()]);
        
        graph.build_reverse_dependencies();
        
        let sorted = graph.topological_sort().unwrap();
        
        // C should come before B, B should come before A
        let pos_c = sorted.iter().position(|id| id == &service_c).unwrap();
        let pos_b = sorted.iter().position(|id| id == &service_b).unwrap();
        let pos_a = sorted.iter().position(|id| id == &service_a).unwrap();
        
        assert!(pos_c < pos_b);
        assert!(pos_b < pos_a);
    }

    #[test]
    fn test_resolution_path() {
        let mut path = ResolutionPath::new();
        let service_a = ServiceId::named::<String>("A");
        let service_b = ServiceId::named::<String>("B");
        
        path.push(service_a.clone());
        path.push(service_b.clone());
        
        assert!(path.contains(&service_a));
        assert!(path.contains(&service_b));
        
        let popped = path.pop();
        assert_eq!(popped, Some(service_b.clone()));
        assert!(!path.contains(&service_b));
        assert!(path.contains(&service_a));
    }
}