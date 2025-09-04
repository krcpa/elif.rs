use elif_core::ElifError;
use crate::commands::new::simple_interactive::ProjectConfig;
use crate::commands::new::template_generator;

pub async fn app(
    name: &str,
    path: Option<&str>,
    template: &str,
) -> Result<(), ElifError> {
    // Create project config from the provided parameters
    let project_type = match template {
        "api" => "API Server",
        "web" => "Full-Stack Web App",
        "minimal" => "Minimal Setup",
        _ => "API Server", // Default fallback
    };

    let config = ProjectConfig {
        name: name.to_string(),
        project_type: project_type.to_string(),
        database_enabled: template != "minimal", // Enable database unless minimal
        database_type: if template != "minimal" { "postgresql".to_string() } else { "none".to_string() },
        database_name: if template != "minimal" { Some(format!("{}_development", name)) } else { None },
        include_seeders: false,
        auth_enabled: false,
        jwt_enabled: false,
        features: vec![],
        directory: path.map(|s| s.to_string()),
    };

    // Use the template generator with git initialization
    template_generator::generate_project_from_template(&config).await?;
    
    Ok(())
}
