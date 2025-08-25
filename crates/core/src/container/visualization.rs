use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use crate::container::descriptor::{ServiceDescriptor, ServiceId};
use crate::container::ioc_container::IocContainer;
use crate::container::module::ModuleRegistry;
use crate::container::scope::ServiceScope;
use crate::errors::CoreError;

/// Dependency graph visualization formats
#[derive(Debug, Clone, PartialEq)]
pub enum VisualizationFormat {
    /// Graphviz DOT format
    Dot,
    /// Mermaid diagram format
    Mermaid,
    /// ASCII art tree
    Ascii,
    /// JSON representation
    Json,
    /// HTML interactive format
    Html,
}

/// Visualization style configuration
#[derive(Debug, Clone)]
pub struct VisualizationStyle {
    /// Show service lifetimes in visualization
    pub show_lifetimes: bool,
    /// Show service names/types
    pub show_names: bool,
    /// Color code by lifetime
    pub color_by_lifetime: bool,
    /// Group services by module
    pub group_by_module: bool,
    /// Show only specific service types
    pub filter_types: Option<Vec<String>>,
    /// Maximum depth to visualize
    pub max_depth: Option<usize>,
    /// Include service statistics
    pub include_stats: bool,
}

impl Default for VisualizationStyle {
    fn default() -> Self {
        Self {
            show_lifetimes: true,
            show_names: true,
            color_by_lifetime: true,
            group_by_module: false,
            filter_types: None,
            max_depth: None,
            include_stats: false,
        }
    }
}

/// Container dependency visualizer
// Debug removed due to ModuleRegistry not implementing Debug
pub struct DependencyVisualizer {
    descriptors: Vec<ServiceDescriptor>,
    dependency_graph: HashMap<ServiceId, Vec<ServiceId>>,
    reverse_graph: HashMap<ServiceId, Vec<ServiceId>>,
    modules: Option<ModuleRegistry>,
}

impl DependencyVisualizer {
    /// Create a new visualizer from service descriptors
    pub fn new(descriptors: Vec<ServiceDescriptor>) -> Self {
        let mut dependency_graph = HashMap::new();
        let mut reverse_graph: HashMap<ServiceId, Vec<ServiceId>> = HashMap::new();

        for descriptor in &descriptors {
            dependency_graph.insert(
                descriptor.service_id.clone(),
                descriptor.dependencies.clone(),
            );

            // Build reverse dependency graph
            for dependency in &descriptor.dependencies {
                reverse_graph
                    .entry(dependency.clone())
                    .or_default()
                    .push(descriptor.service_id.clone());
            }
        }

        Self {
            descriptors,
            dependency_graph,
            reverse_graph,
            modules: None,
        }
    }

    /// Create visualizer from IoC container
    pub fn from_container(container: &IocContainer) -> Self {
        let descriptors = container
            .registered_services()
            .into_iter()
            .filter_map(|_service_id| {
                // In a real implementation, we'd get the actual descriptors
                // For now, create minimal descriptors
                None // Skip creating descriptors for now due to complexity
            })
            .collect();

        Self::new(descriptors)
    }

    /// Add module information for module-based visualization
    pub fn with_modules(mut self, modules: ModuleRegistry) -> Self {
        self.modules = Some(modules);
        self
    }

    /// Generate visualization in specified format
    pub fn visualize(
        &self,
        format: VisualizationFormat,
        style: VisualizationStyle,
    ) -> Result<String, CoreError> {
        match format {
            VisualizationFormat::Dot => self.generate_dot(style),
            VisualizationFormat::Mermaid => self.generate_mermaid(style),
            VisualizationFormat::Ascii => self.generate_ascii(style),
            VisualizationFormat::Json => self.generate_json(style),
            VisualizationFormat::Html => self.generate_html(style),
        }
    }

    /// Generate Graphviz DOT format
    fn generate_dot(&self, style: VisualizationStyle) -> Result<String, CoreError> {
        let mut dot = String::new();
        writeln!(dot, "digraph ServiceDependencies {{").unwrap();
        writeln!(dot, "    rankdir=TB;").unwrap();
        writeln!(dot, "    node [shape=rectangle];").unwrap();
        writeln!(dot).unwrap();

        // Define lifetime colors
        if style.color_by_lifetime {
            writeln!(dot, "    // Lifetime color scheme").unwrap();
            writeln!(dot, "    // Singleton: lightblue").unwrap();
            writeln!(dot, "    // Scoped: lightgreen").unwrap();
            writeln!(dot, "    // Transient: lightyellow").unwrap();
            writeln!(dot).unwrap();
        }

        // Add service nodes
        for descriptor in &self.descriptors {
            if let Some(ref filter) = style.filter_types {
                if !filter
                    .iter()
                    .any(|f| descriptor.service_id.type_name().contains(f))
                {
                    continue;
                }
            }

            let service_name = self.format_service_name(&descriptor.service_id, &style);
            let mut node_attrs = Vec::new();

            if style.color_by_lifetime {
                let color = match descriptor.lifetime {
                    ServiceScope::Singleton => "lightblue",
                    ServiceScope::Scoped => "lightgreen",
                    ServiceScope::Transient => "lightyellow",
                };
                node_attrs.push(format!("fillcolor={}", color));
                node_attrs.push("style=filled".to_string());
            }

            if style.show_lifetimes {
                let lifetime_text = format!("\\n({:?})", descriptor.lifetime);
                node_attrs.push(format!("label=\"{}{}\"", service_name, lifetime_text));
            } else {
                node_attrs.push(format!("label=\"{}\"", service_name));
            }

            writeln!(
                dot,
                "    \"{}\" [{}];",
                descriptor.service_id.type_name(),
                node_attrs.join(", ")
            )
            .unwrap();
        }

        writeln!(dot).unwrap();

        // Add dependency edges
        for (service_id, dependencies) in &self.dependency_graph {
            if let Some(ref filter) = style.filter_types {
                if !filter.iter().any(|f| service_id.type_name().contains(f)) {
                    continue;
                }
            }

            for dependency in dependencies {
                writeln!(
                    dot,
                    "    \"{}\" -> \"{}\";",
                    service_id.type_name(),
                    dependency.type_name()
                )
                .unwrap();
            }
        }

        writeln!(dot, "}}").unwrap();
        Ok(dot)
    }

    /// Generate Mermaid diagram format
    fn generate_mermaid(&self, style: VisualizationStyle) -> Result<String, CoreError> {
        let mut mermaid = String::new();
        writeln!(mermaid, "graph TD").unwrap();

        // Add service nodes with lifetimes
        for descriptor in &self.descriptors {
            if let Some(ref filter) = style.filter_types {
                if !filter
                    .iter()
                    .any(|f| descriptor.service_id.type_name().contains(f))
                {
                    continue;
                }
            }

            let service_name = self.format_service_name(&descriptor.service_id, &style);
            let node_id = self.sanitize_id(descriptor.service_id.type_name());

            let lifetime_indicator = if style.show_lifetimes {
                match descriptor.lifetime {
                    ServiceScope::Singleton => "●",
                    ServiceScope::Scoped => "◐",
                    ServiceScope::Transient => "○",
                }
            } else {
                ""
            };

            if style.color_by_lifetime {
                let class_name = match descriptor.lifetime {
                    ServiceScope::Singleton => "singleton",
                    ServiceScope::Scoped => "scoped",
                    ServiceScope::Transient => "transient",
                };
                writeln!(
                    mermaid,
                    "    {}[\"{} {}\"]::{}",
                    node_id, lifetime_indicator, service_name, class_name
                )
                .unwrap();
            } else {
                writeln!(
                    mermaid,
                    "    {}[\"{} {}\"]",
                    node_id, lifetime_indicator, service_name
                )
                .unwrap();
            }
        }

        writeln!(mermaid).unwrap();

        // Add dependency relationships
        for (service_id, dependencies) in &self.dependency_graph {
            if let Some(ref filter) = style.filter_types {
                if !filter.iter().any(|f| service_id.type_name().contains(f)) {
                    continue;
                }
            }

            let service_node_id = self.sanitize_id(service_id.type_name());
            for dependency in dependencies {
                let dep_node_id = self.sanitize_id(dependency.type_name());
                writeln!(mermaid, "    {} --> {}", service_node_id, dep_node_id).unwrap();
            }
        }

        // Add style classes
        if style.color_by_lifetime {
            writeln!(mermaid).unwrap();
            writeln!(mermaid, "    classDef singleton fill:#add8e6").unwrap();
            writeln!(mermaid, "    classDef scoped fill:#90ee90").unwrap();
            writeln!(mermaid, "    classDef transient fill:#ffffe0").unwrap();
        }

        Ok(mermaid)
    }

    /// Generate ASCII tree format
    fn generate_ascii(&self, style: VisualizationStyle) -> Result<String, CoreError> {
        let mut ascii = String::new();
        writeln!(ascii, "Service Dependency Tree").unwrap();
        writeln!(ascii, "=======================").unwrap();
        writeln!(ascii).unwrap();

        // Find root services (services with no dependents or explicitly marked as root)
        let mut roots = Vec::new();
        for descriptor in &self.descriptors {
            if !self.reverse_graph.contains_key(&descriptor.service_id)
                || self.reverse_graph[&descriptor.service_id].is_empty()
            {
                roots.push(&descriptor.service_id);
            }
        }

        // If no clear roots, use all services that don't depend on others
        if roots.is_empty() {
            for descriptor in &self.descriptors {
                if descriptor.dependencies.is_empty() {
                    roots.push(&descriptor.service_id);
                }
            }
        }

        let mut visited = HashSet::new();
        for root in roots {
            self.generate_ascii_tree(
                root,
                &style,
                &mut ascii,
                0,
                "",
                &mut visited,
                style.max_depth,
            )?;
        }

        if style.include_stats {
            writeln!(ascii).unwrap();
            writeln!(ascii, "Statistics:").unwrap();
            writeln!(ascii, "-----------").unwrap();
            writeln!(ascii, "Total services: {}", self.descriptors.len()).unwrap();

            let mut lifetime_counts = HashMap::new();
            for desc in &self.descriptors {
                *lifetime_counts.entry(desc.lifetime).or_insert(0) += 1;
            }

            for (lifetime, count) in lifetime_counts {
                writeln!(ascii, "{:?}: {}", lifetime, count).unwrap();
            }
        }

        Ok(ascii)
    }

    /// Generate ASCII tree for a specific service
    fn generate_ascii_tree(
        &self,
        service_id: &ServiceId,
        style: &VisualizationStyle,
        output: &mut String,
        depth: usize,
        prefix: &str,
        visited: &mut HashSet<ServiceId>,
        max_depth: Option<usize>,
    ) -> Result<(), CoreError> {
        if let Some(max_d) = max_depth {
            if depth >= max_d {
                return Ok(());
            }
        }

        if visited.contains(service_id) {
            writeln!(
                output,
                "{}├── {} (circular)",
                prefix,
                self.format_service_name(service_id, style)
            )
            .unwrap();
            return Ok(());
        }

        visited.insert(service_id.clone());

        let descriptor = self
            .descriptors
            .iter()
            .find(|d| &d.service_id == service_id);

        let service_display = if let Some(desc) = descriptor {
            if style.show_lifetimes {
                format!(
                    "{} ({:?})",
                    self.format_service_name(service_id, style),
                    desc.lifetime
                )
            } else {
                self.format_service_name(service_id, style)
            }
        } else {
            self.format_service_name(service_id, style)
        };

        writeln!(output, "{}├── {}", prefix, service_display).unwrap();

        if let Some(dependencies) = self.dependency_graph.get(service_id) {
            for (i, dependency) in dependencies.iter().enumerate() {
                let is_last = i == dependencies.len() - 1;
                let new_prefix = if is_last {
                    format!("{}    ", prefix)
                } else {
                    format!("{}│   ", prefix)
                };

                self.generate_ascii_tree(
                    dependency,
                    style,
                    output,
                    depth + 1,
                    &new_prefix,
                    visited,
                    max_depth,
                )?;
            }
        }

        visited.remove(service_id);
        Ok(())
    }

    /// Generate JSON representation
    fn generate_json(&self, style: VisualizationStyle) -> Result<String, CoreError> {
        use std::collections::BTreeMap; // For consistent ordering

        let mut json_data = BTreeMap::new();

        // Services array
        let mut services = Vec::new();
        for descriptor in &self.descriptors {
            if let Some(ref filter) = style.filter_types {
                if !filter
                    .iter()
                    .any(|f| descriptor.service_id.type_name().contains(f))
                {
                    continue;
                }
            }

            let mut service_data = BTreeMap::new();
            service_data.insert(
                "id".to_string(),
                serde_json::Value::String(descriptor.service_id.type_name().to_string()),
            );

            if style.show_names {
                service_data.insert(
                    "name".to_string(),
                    serde_json::Value::String(
                        self.format_service_name(&descriptor.service_id, &style),
                    ),
                );
            }

            if style.show_lifetimes {
                service_data.insert(
                    "lifetime".to_string(),
                    serde_json::Value::String(format!("{:?}", descriptor.lifetime)),
                );
            }

            let deps: Vec<serde_json::Value> = descriptor
                .dependencies
                .iter()
                .map(|dep| serde_json::Value::String(dep.type_name().to_string()))
                .collect();
            service_data.insert("dependencies".to_string(), serde_json::Value::Array(deps));

            services.push(serde_json::Value::Object(
                service_data.into_iter().collect(),
            ));
        }

        json_data.insert("services".to_string(), serde_json::Value::Array(services));

        // Dependencies edges
        let mut edges = Vec::new();
        for (service_id, dependencies) in &self.dependency_graph {
            if let Some(ref filter) = style.filter_types {
                if !filter.iter().any(|f| service_id.type_name().contains(f)) {
                    continue;
                }
            }

            for dependency in dependencies {
                let mut edge = BTreeMap::new();
                edge.insert(
                    "from".to_string(),
                    serde_json::Value::String(service_id.type_name().to_string()),
                );
                edge.insert(
                    "to".to_string(),
                    serde_json::Value::String(dependency.type_name().to_string()),
                );

                edges.push(serde_json::Value::Object(edge.into_iter().collect()));
            }
        }

        json_data.insert("edges".to_string(), serde_json::Value::Array(edges));

        // Statistics if requested
        if style.include_stats {
            let mut stats = BTreeMap::new();
            stats.insert(
                "total_services".to_string(),
                serde_json::Value::Number(serde_json::Number::from(self.descriptors.len())),
            );

            let total_deps: usize = self.dependency_graph.values().map(|deps| deps.len()).sum();
            stats.insert(
                "total_dependencies".to_string(),
                serde_json::Value::Number(serde_json::Number::from(total_deps)),
            );

            json_data.insert(
                "statistics".to_string(),
                serde_json::Value::Object(stats.into_iter().collect()),
            );
        }

        let json_value = serde_json::Value::Object(json_data.into_iter().collect());
        serde_json::to_string_pretty(&json_value).map_err(|e| CoreError::InvalidServiceDescriptor {
            message: format!("Failed to serialize JSON: {}", e),
        })
    }

    /// Generate interactive HTML visualization
    fn generate_html(&self, style: VisualizationStyle) -> Result<String, CoreError> {
        let _json_data = self.generate_json(style)?;

        // Simplified HTML placeholder - full interactive D3.js visualization would go here
        Ok("<html><body><h1>Service Dependencies Visualization</h1><p>Interactive visualization would be generated here</p></body></html>".to_string())
    }

    /// Format service name according to style
    fn format_service_name(&self, service_id: &ServiceId, style: &VisualizationStyle) -> String {
        if !style.show_names {
            return "Service".to_string();
        }

        if let Some(name) = &service_id.name {
            name.clone()
        } else {
            // Extract just the type name without module path
            let type_name = service_id.type_name();
            type_name
                .split("::")
                .last()
                .unwrap_or(type_name)
                .to_string()
        }
    }

    /// Sanitize ID for use in Mermaid diagrams
    fn sanitize_id(&self, id: &str) -> String {
        id.replace("::", "_")
            .replace("<", "_")
            .replace(">", "_")
            .replace(" ", "_")
            .replace("-", "_")
    }
}

/// Service explorer for interactive dependency investigation
// Debug removed due to DependencyVisualizer not implementing Debug
pub struct ServiceExplorer {
    visualizer: DependencyVisualizer,
}

impl ServiceExplorer {
    /// Create a new service explorer
    pub fn new(descriptors: Vec<ServiceDescriptor>) -> Self {
        Self {
            visualizer: DependencyVisualizer::new(descriptors),
        }
    }

    /// Find all paths between two services
    pub fn find_paths(&self, from: &ServiceId, to: &ServiceId) -> Vec<Vec<ServiceId>> {
        let mut paths = Vec::new();
        let mut current_path = Vec::new();
        let mut visited = HashSet::new();

        self.find_paths_recursive(from, to, &mut current_path, &mut visited, &mut paths);
        paths
    }

    /// Recursive path finding
    fn find_paths_recursive(
        &self,
        current: &ServiceId,
        target: &ServiceId,
        path: &mut Vec<ServiceId>,
        visited: &mut HashSet<ServiceId>,
        paths: &mut Vec<Vec<ServiceId>>,
    ) {
        if visited.contains(current) {
            return; // Avoid cycles
        }

        path.push(current.clone());
        visited.insert(current.clone());

        if current == target {
            paths.push(path.clone());
        } else if let Some(dependencies) = self.visualizer.dependency_graph.get(current) {
            for dependency in dependencies {
                self.find_paths_recursive(dependency, target, path, visited, paths);
            }
        }

        path.pop();
        visited.remove(current);
    }

    /// Get services that depend on a given service
    pub fn get_dependents(&self, service_id: &ServiceId) -> Vec<&ServiceId> {
        self.visualizer
            .reverse_graph
            .get(service_id)
            .map(|deps| deps.iter().collect())
            .unwrap_or_default()
    }

    /// Get dependency depth for a service
    pub fn get_dependency_depth(&self, service_id: &ServiceId) -> usize {
        let mut max_depth = 0;
        let mut visited = HashSet::new();

        self.calculate_depth(service_id, 0, &mut max_depth, &mut visited);
        max_depth
    }

    /// Calculate maximum dependency depth recursively
    fn calculate_depth(
        &self,
        service_id: &ServiceId,
        current_depth: usize,
        max_depth: &mut usize,
        visited: &mut HashSet<ServiceId>,
    ) {
        if visited.contains(service_id) {
            return; // Avoid infinite recursion
        }

        *max_depth = (*max_depth).max(current_depth);
        visited.insert(service_id.clone());

        if let Some(dependencies) = self.visualizer.dependency_graph.get(service_id) {
            for dependency in dependencies {
                self.calculate_depth(dependency, current_depth + 1, max_depth, visited);
            }
        }

        visited.remove(service_id);
    }

    /// Export service information as a report
    pub fn generate_report(&self) -> String {
        let mut report = String::new();

        writeln!(report, "Service Dependency Analysis Report").unwrap();
        writeln!(report, "===================================").unwrap();
        writeln!(report).unwrap();

        writeln!(report, "Summary:").unwrap();
        writeln!(report, "--------").unwrap();
        writeln!(
            report,
            "Total services: {}",
            self.visualizer.descriptors.len()
        )
        .unwrap();
        writeln!(
            report,
            "Total dependencies: {}",
            self.visualizer
                .dependency_graph
                .values()
                .map(|deps| deps.len())
                .sum::<usize>()
        )
        .unwrap();
        writeln!(report).unwrap();

        // Services with most dependencies
        let mut services_by_deps: Vec<_> = self
            .visualizer
            .descriptors
            .iter()
            .map(|desc| (desc, desc.dependencies.len()))
            .collect();
        services_by_deps.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

        writeln!(report, "Services with Most Dependencies:").unwrap();
        writeln!(report, "-------------------------------").unwrap();
        for (desc, count) in services_by_deps.iter().take(10) {
            writeln!(
                report,
                "{}: {} dependencies",
                desc.service_id.type_name(),
                count
            )
            .unwrap();
        }
        writeln!(report).unwrap();

        // Services with most dependents
        let mut dependents_count: HashMap<&ServiceId, usize> = HashMap::new();
        for dependents in self.visualizer.reverse_graph.values() {
            for dependent in dependents {
                *dependents_count.entry(dependent).or_insert(0) += 1;
            }
        }

        let mut services_by_dependents: Vec<_> = dependents_count.into_iter().collect();
        services_by_dependents.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

        writeln!(report, "Most Depended-Upon Services:").unwrap();
        writeln!(report, "---------------------------").unwrap();
        for (service_id, count) in services_by_dependents.iter().take(10) {
            writeln!(report, "{}: {} dependents", service_id.type_name(), count).unwrap();
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::descriptor::{ServiceActivationStrategy, ServiceDescriptor};
    use std::any::{Any, TypeId};

    fn create_test_descriptor(
        type_name: &str,
        lifetime: ServiceScope,
        deps: Vec<&str>,
    ) -> ServiceDescriptor {
        let service_id = ServiceId {
            type_id: TypeId::of::<()>(),
            type_name: "test_service",
            name: Some(type_name.to_string()),
        };

        let dependencies: Vec<ServiceId> = deps
            .iter()
            .map(|dep| ServiceId {
                type_id: TypeId::of::<()>(),
                type_name: "test_dependency",
                name: Some(dep.to_string()),
            })
            .collect();

        ServiceDescriptor {
            service_id,
            implementation_id: TypeId::of::<()>(),
            lifetime,
            dependencies,
            activation_strategy: ServiceActivationStrategy::Factory(Box::new(|| {
                Ok(Box::new(()) as Box<dyn Any + Send + Sync>)
            })),
        }
    }

    #[test]
    fn test_dot_generation() {
        let descriptors = vec![
            create_test_descriptor("ServiceA", ServiceScope::Singleton, vec![]),
            create_test_descriptor("ServiceB", ServiceScope::Scoped, vec!["ServiceA"]),
        ];

        let visualizer = DependencyVisualizer::new(descriptors);
        let style = VisualizationStyle::default();

        let dot = visualizer.generate_dot(style).unwrap();

        assert!(dot.contains("digraph ServiceDependencies"));
        assert!(dot.contains("ServiceA"));
        assert!(dot.contains("ServiceB"));
        assert!(dot.contains("ServiceB\" -> \"ServiceA"));
    }

    #[test]
    fn test_mermaid_generation() {
        let descriptors = vec![
            create_test_descriptor("ServiceA", ServiceScope::Singleton, vec![]),
            create_test_descriptor("ServiceB", ServiceScope::Transient, vec!["ServiceA"]),
        ];

        let visualizer = DependencyVisualizer::new(descriptors);
        let style = VisualizationStyle::default();

        let mermaid = visualizer.generate_mermaid(style).unwrap();

        assert!(mermaid.contains("graph TD"));
        assert!(mermaid.contains("ServiceA"));
        assert!(mermaid.contains("ServiceB"));
        assert!(mermaid.contains("-->"));
    }

    #[test]
    fn test_ascii_generation() {
        let descriptors = vec![
            create_test_descriptor("ServiceA", ServiceScope::Singleton, vec![]),
            create_test_descriptor("ServiceB", ServiceScope::Scoped, vec!["ServiceA"]),
        ];

        let visualizer = DependencyVisualizer::new(descriptors);
        let style = VisualizationStyle::default();

        let ascii = visualizer.generate_ascii(style).unwrap();

        assert!(ascii.contains("Service Dependency Tree"));
        assert!(ascii.contains("ServiceA"));
        assert!(ascii.contains("ServiceB"));
    }

    #[test]
    fn test_json_generation() {
        let descriptors = vec![
            create_test_descriptor("ServiceA", ServiceScope::Singleton, vec![]),
            create_test_descriptor("ServiceB", ServiceScope::Transient, vec!["ServiceA"]),
        ];

        let visualizer = DependencyVisualizer::new(descriptors);
        let style = VisualizationStyle::default();

        let json = visualizer.generate_json(style).unwrap();

        // Parse JSON to verify it's valid
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("services").is_some());
        assert!(parsed.get("edges").is_some());
    }

    #[test]
    fn test_service_explorer() {
        let descriptors = vec![
            create_test_descriptor("ServiceA", ServiceScope::Singleton, vec![]),
            create_test_descriptor("ServiceB", ServiceScope::Scoped, vec!["ServiceA"]),
            create_test_descriptor("ServiceC", ServiceScope::Transient, vec!["ServiceB"]),
        ];

        let explorer = ServiceExplorer::new(descriptors);

        let service_a = ServiceId {
            type_id: TypeId::of::<()>(),
            type_name: "test_service",
            name: Some("ServiceA".to_string()),
        };
        let service_c = ServiceId {
            type_id: TypeId::of::<()>(),
            type_name: "test_service",
            name: Some("ServiceC".to_string()),
        };

        // Test path finding
        let paths = explorer.find_paths(&service_c, &service_a);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].len(), 3); // C -> B -> A

        // Test dependency depth
        let depth = explorer.get_dependency_depth(&service_c);
        assert_eq!(depth, 2); // C depends on B which depends on A
    }

    #[test]
    fn test_style_filtering() {
        let descriptors = vec![
            create_test_descriptor("UserService", ServiceScope::Singleton, vec![]),
            create_test_descriptor("PaymentService", ServiceScope::Scoped, vec!["UserService"]),
            create_test_descriptor("NotificationService", ServiceScope::Transient, vec![]),
        ];

        let visualizer = DependencyVisualizer::new(descriptors);
        let mut style = VisualizationStyle::default();
        style.filter_types = Some(vec!["User".to_string(), "Payment".to_string()]);

        let json = visualizer.generate_json(style).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        let services = parsed["services"].as_array().unwrap();
        assert_eq!(services.len(), 2); // Only UserService and PaymentService

        let service_names: Vec<&str> = services.iter().map(|s| s["id"].as_str().unwrap()).collect();
        assert!(service_names.contains(&"UserService"));
        assert!(service_names.contains(&"PaymentService"));
        assert!(!service_names.contains(&"NotificationService"));
    }
}
