pub mod generator;
pub mod templates;
pub mod writer;

pub use generator::*;
pub use writer::*;

use elif_core::{ElifError, ResourceSpec};
use std::path::PathBuf;

pub struct CodeGenerator {
    pub project_root: PathBuf,
}

impl CodeGenerator {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    pub fn generate_all(&self) -> Result<(), ElifError> {
        let resources_dir = self.project_root.join("resources");
        if !resources_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(&resources_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml")
                && path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.ends_with(".resource"))
            {
                let content = std::fs::read_to_string(&path)?;
                let spec = ResourceSpec::from_yaml(&content)?;
                self.generate_resource(&spec)?;
            }
        }

        Ok(())
    }

    pub fn generate_resource(&self, spec: &ResourceSpec) -> Result<(), ElifError> {
        let generator = ResourceGenerator::new(&self.project_root, spec);

        generator.generate_model()?;
        generator.generate_handler()?;
        generator.generate_migration()?;
        generator.generate_test()?;

        Ok(())
    }
}
