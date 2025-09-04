//! Route diagnostics and error reporting for development and debugging
//!
//! This module provides enhanced diagnostics for route conflicts, validation errors,
//! and performance analysis to improve the developer experience.

use crate::{
    bootstrap::{RouteConflict, ConflictType, RouteInfo, ValidationReport},
    routing::{HttpMethod, RouteDefinition},
};
use std::collections::HashMap;

/// Enhanced diagnostics for route analysis and debugging
#[derive(Debug)]
pub struct RouteDiagnostics {
    /// Enable colored output for terminal display
    enable_colors: bool,
    /// Enable detailed timing information
    enable_timing: bool,
    /// Maximum width for formatted output
    max_width: usize,
}

impl RouteDiagnostics {
    /// Create new route diagnostics instance
    pub fn new() -> Self {
        Self {
            enable_colors: true,
            enable_timing: false,
            max_width: 80,
        }
    }

    /// Configure color output
    pub fn with_colors(mut self, enable: bool) -> Self {
        self.enable_colors = enable;
        self
    }

    /// Configure timing output
    pub fn with_timing(mut self, enable: bool) -> Self {
        self.enable_timing = enable;
        self
    }

    /// Set maximum output width
    pub fn with_max_width(mut self, width: usize) -> Self {
        self.max_width = width;
        self
    }

    /// Generate comprehensive conflict report
    pub fn format_conflict_report(&self, conflicts: &[RouteConflict]) -> String {
        let mut report = String::new();
        
        // Header
        report.push_str(&self.format_header("Route Conflict Analysis"));
        report.push('\n');

        // Summary
        report.push_str(&format!("Found {} route conflicts that need resolution:\n\n", conflicts.len()));

        // Individual conflicts
        for (i, conflict) in conflicts.iter().enumerate() {
            if i > 0 {
                report.push_str("\n");
                report.push_str(&"─".repeat(self.max_width));
                report.push_str("\n\n");
            }

            report.push_str(&self.format_conflict(conflict, i + 1));
        }

        // Footer with recommendations
        report.push('\n');
        report.push_str(&self.format_recommendations());

        report
    }

    /// Format validation report summary
    pub fn format_validation_summary(&self, report: &ValidationReport) -> String {
        let mut output = String::new();

        output.push_str(&self.format_header("Route Validation Summary"));
        output.push('\n');

        // Statistics
        output.push_str(&format!("📊 Statistics:\n"));
        output.push_str(&format!("   Total routes: {}\n", report.total_routes));
        output.push_str(&format!("   Conflicts: {}\n", report.conflicts));
        output.push_str(&format!("   Warnings: {}\n", report.warnings));
        output.push_str(&format!("   Performance score: {}/100\n", report.performance_score));

        if !report.suggestions.is_empty() {
            output.push('\n');
            output.push_str("💡 Optimization Suggestions:\n");
            for (i, suggestion) in report.suggestions.iter().enumerate() {
                output.push_str(&format!("   {}. {}\n", i + 1, suggestion));
            }
        }

        output
    }

    /// Format route analysis for debugging
    pub fn format_route_analysis(&self, routes: &[RouteDefinition]) -> String {
        let mut analysis = String::new();

        analysis.push_str(&self.format_header("Route Analysis"));
        analysis.push('\n');

        // Group by HTTP method
        let mut method_groups: HashMap<HttpMethod, Vec<&RouteDefinition>> = HashMap::new();
        for route in routes {
            method_groups.entry(route.method.clone()).or_default().push(route);
        }

        for (method, method_routes) in method_groups.iter() {
            analysis.push_str(&format!("🔗 {} Routes ({})\n", method.as_str(), method_routes.len()));
            
            for route in method_routes {
                let complexity = self.calculate_route_complexity(route);
                let complexity_indicator = match complexity {
                    0..=2 => "🟢",
                    3..=5 => "🟡", 
                    _ => "🔴",
                };
                
                analysis.push_str(&format!("   {} {} (complexity: {})\n", 
                    complexity_indicator, route.path, complexity));
            }
            analysis.push('\n');
        }

        analysis
    }

    /// Format performance recommendations
    pub fn format_performance_recommendations(&self, routes: &[RouteDefinition]) -> String {
        let mut recommendations = String::new();
        
        recommendations.push_str(&self.format_header("Performance Recommendations"));
        recommendations.push('\n');

        // Analyze route complexity
        let complex_routes: Vec<_> = routes.iter()
            .filter(|r| self.calculate_route_complexity(r) > 5)
            .collect();

        if !complex_routes.is_empty() {
            recommendations.push_str("⚠️  Complex Routes Detected:\n");
            for route in complex_routes {
                recommendations.push_str(&format!("   • {} {} (consider simplifying)\n", 
                    route.method.as_str(), route.path));
            }
            recommendations.push('\n');
        }

        // Route count recommendations
        if routes.len() > 1000 {
            recommendations.push_str("📈 Large Route Count:\n");
            recommendations.push_str(&format!("   • {} routes detected\n", routes.len()));
            recommendations.push_str("   • Consider route grouping or lazy loading\n");
            recommendations.push_str("   • Review route organization patterns\n\n");
        }

        if recommendations.is_empty() {
            recommendations.push_str("✅ No performance issues detected\n");
        }

        recommendations
    }

    /// Format individual conflict details
    fn format_conflict(&self, conflict: &RouteConflict, index: usize) -> String {
        let mut output = String::new();

        // Conflict header
        let conflict_type_emoji = match conflict.conflict_type {
            ConflictType::Exact => "🚨",
            ConflictType::ParameterMismatch => "⚠️",
            ConflictType::Ambiguous => "❓",
            ConflictType::MiddlewareIncompatible => "🔧",
        };

        output.push_str(&format!("{} Conflict #{}: {}\n\n", 
            conflict_type_emoji, index, self.format_conflict_type(&conflict.conflict_type)));

        // Route details
        output.push_str("📍 Conflicting Routes:\n");
        output.push_str(&self.format_route_info(&conflict.route1, "1"));
        output.push_str(&self.format_route_info(&conflict.route2, "2"));

        // Resolution suggestions
        if !conflict.resolution_suggestions.is_empty() {
            output.push_str("\n💡 Resolution Options:\n");
            for (i, suggestion) in conflict.resolution_suggestions.iter().enumerate() {
                output.push_str(&format!("   {}. {}\n", 
                    i + 1, self.format_resolution_suggestion(suggestion)));
            }
        }

        output
    }

    /// Format route information for display
    fn format_route_info(&self, route: &RouteInfo, number: &str) -> String {
        let mut info = String::new();
        
        info.push_str(&format!("   {}. {} {}\n", number, route.method.as_str(), route.path));
        info.push_str(&format!("      Controller: {}::{}\n", route.controller, route.handler));
        
        if !route.middleware.is_empty() {
            info.push_str(&format!("      Middleware: {:?}\n", route.middleware));
        }
        
        if !route.parameters.is_empty() {
            info.push_str("      Parameters: ");
            let param_strs: Vec<String> = route.parameters.iter()
                .map(|p| format!("{}: {}", p.name, p.param_type))
                .collect();
            info.push_str(&param_strs.join(", "));
            info.push('\n');
        }

        info
    }

    /// Format conflict type description
    fn format_conflict_type(&self, conflict_type: &ConflictType) -> String {
        match conflict_type {
            ConflictType::Exact => "Exact Route Duplicate".to_string(),
            ConflictType::ParameterMismatch => "Parameter Type Mismatch".to_string(),
            ConflictType::Ambiguous => "Ambiguous Route Pattern".to_string(),
            ConflictType::MiddlewareIncompatible => "Middleware Incompatibility".to_string(),
        }
    }

    /// Format resolution suggestion
    fn format_resolution_suggestion(&self, suggestion: &crate::bootstrap::ConflictResolution) -> String {
        use crate::bootstrap::ConflictResolution;
        
        match suggestion {
            ConflictResolution::MergePaths { suggestion } => {
                format!("Merge paths: {}", suggestion)
            },
            ConflictResolution::RenameParameter { from, to } => {
                format!("Rename parameter '{}' to '{}'", from, to)
            },
            ConflictResolution::DifferentControllerPaths { suggestion } => {
                format!("Use different controller paths: {}", suggestion)
            },
            ConflictResolution::MiddlewareConsolidation { suggestion } => {
                format!("Consolidate middleware: {}", suggestion)
            },
            ConflictResolution::UseQueryParameters { suggestion } => {
                format!("Use query parameters: {}", suggestion)
            },
            ConflictResolution::ReorderRoutes { suggestion } => {
                format!("Reorder routes: {}", suggestion)
            },
        }
    }

    /// Format section header
    fn format_header(&self, title: &str) -> String {
        let border = "═".repeat(self.max_width);
        format!("{}\n{:^width$}\n{}", border, title, border, width = self.max_width)
    }

    /// Format general recommendations
    fn format_recommendations(&self) -> String {
        let mut recommendations = String::new();
        
        recommendations.push_str("🎯 General Recommendations:\n");
        recommendations.push_str("   • Use specific route patterns to avoid ambiguity\n");
        recommendations.push_str("   • Group related routes in the same controller\n");
        recommendations.push_str("   • Consistent parameter naming across controllers\n");
        recommendations.push_str("   • Use middleware consistently for similar routes\n");
        recommendations.push_str("   • Consider RESTful routing conventions\n");
        
        recommendations.push_str("\n📖 Documentation:\n");
        recommendations.push_str("   • Route conflict resolution: https://docs.elif.rs/routing/conflicts\n");
        recommendations.push_str("   • Best practices: https://docs.elif.rs/routing/best-practices\n");

        recommendations
    }

    /// Calculate route complexity score
    fn calculate_route_complexity(&self, route: &RouteDefinition) -> u32 {
        let mut complexity = 0;

        // Count path segments
        let segments = route.path.split('/').filter(|s| !s.is_empty()).count();
        complexity += segments as u32;

        // Count parameters
        let param_count = route.path.matches('{').count();
        complexity += param_count as u32 * 2; // Parameters add more complexity

        // Bonus complexity for catch-all parameters
        if route.path.contains("*") {
            complexity += 3;
        }

        complexity
    }
}

impl Default for RouteDiagnostics {
    fn default() -> Self {
        Self::new()
    }
}

/// CLI-friendly formatter for route diagnostics
pub struct CliDiagnosticsFormatter {
    diagnostics: RouteDiagnostics,
}

impl CliDiagnosticsFormatter {
    /// Create formatter optimized for CLI output
    pub fn new() -> Self {
        Self {
            diagnostics: RouteDiagnostics::new()
                .with_colors(true)
                .with_max_width(120),
        }
    }

    /// Create formatter for non-interactive environments
    pub fn plain() -> Self {
        Self {
            diagnostics: RouteDiagnostics::new()
                .with_colors(false)
                .with_max_width(80),
        }
    }

    /// Format conflicts for CLI display
    pub fn format_conflicts(&self, conflicts: &[RouteConflict]) -> String {
        self.diagnostics.format_conflict_report(conflicts)
    }

    /// Format validation summary for CLI
    pub fn format_summary(&self, report: &ValidationReport) -> String {
        self.diagnostics.format_validation_summary(report)
    }
}

impl Default for CliDiagnosticsFormatter {
    fn default() -> Self {
        Self::new()
    }
}