use elif_core::ElifError;

pub async fn app(
    name: &str,
    _path: Option<&str>,
    _template: &str,
) -> Result<(), ElifError> {
    // Use the new simplified create_new_app function
    crate::create_new_app(name).await
}
