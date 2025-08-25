use elif_core::{ElifError, ResourceSpec};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMap {
    pub routes: Vec<RouteInfo>,
    pub models: Vec<ModelInfo>,
    pub specs: Vec<SpecInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInfo {
    pub op_id: String,
    pub method: String,
    pub path: String,
    pub file: String,
    pub marker: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecInfo {
    pub name: String,
    pub file: String,
}

pub struct MapGenerator {
    project_root: PathBuf,
}

impl MapGenerator {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    pub fn generate(&self) -> Result<ProjectMap, ElifError> {
        let mut routes = Vec::new();
        let mut models = Vec::new();
        let mut specs = Vec::new();

        // Collect resource specifications
        let resources_dir = self.project_root.join("resources");
        if resources_dir.exists() {
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

                    // Add spec info
                    specs.push(SpecInfo {
                        name: spec.name.clone(),
                        file: path.to_string_lossy().to_string(),
                    });

                    // Add model info
                    models.push(ModelInfo {
                        name: spec.name.clone(),
                        file: format!("crates/orm/src/models/{}.rs", spec.name.to_lowercase()),
                    });

                    // Add route info for each operation
                    let handler_file =
                        format!("apps/api/src/routes/{}.rs", spec.name.to_lowercase());
                    for op in &spec.api.operations {
                        routes.push(RouteInfo {
                            op_id: format!("{}.{}", spec.name, op.op),
                            method: op.method.clone(),
                            path: format!("{}{}", spec.route.trim_end_matches('/'), op.path),
                            file: handler_file.clone(),
                            marker: Some(format!("{}_{}", op.op, spec.name)),
                        });
                    }
                }
            }
        }

        Ok(ProjectMap {
            routes,
            models,
            specs,
        })
    }
}
