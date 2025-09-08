//! Bootstrap System for Automatic Configuration
//!
//! This module provides the bootstrap system for automatic provider configuration
//! and container setup based on module declarations.

pub mod providers;

pub use providers::*;

/// Re-exports for convenience
pub use crate::container::auto_config::{
    AutoConfigBuilder, ContainerAutoConfig, ValidationIssue, ConfigurationRule,
    ValidationReport,
};