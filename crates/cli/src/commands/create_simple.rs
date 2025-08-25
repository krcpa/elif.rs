use elif_core::ElifError;

pub async fn app(
    name: &str,
    path: Option<&str>,
    template: &str,
    modules: bool,
) -> Result<(), ElifError> {
    println!(
        "Creating app: {} at {:?} with template: {}, modules: {}",
        name, path, template, modules
    );
    Ok(())
}
