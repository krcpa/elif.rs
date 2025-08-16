/*!
Swagger UI integration for interactive API documentation.

This module provides functionality to serve interactive Swagger UI documentation
for OpenAPI specifications.
*/

use crate::{
    error::{OpenApiError, OpenApiResult},
    specification::OpenApiSpec,
};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::{Html, Json, Response},
    routing::get,
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

/// Application state for the Swagger UI server
#[derive(Clone)]
pub struct SwaggerState {
    /// OpenAPI specification
    pub spec: Arc<OpenApiSpec>,
    /// Configuration
    pub config: SwaggerConfig,
}

/// Swagger UI server for serving interactive API documentation
pub struct SwaggerUi {
    /// OpenAPI specification
    spec: Arc<OpenApiSpec>,
    /// Configuration
    config: SwaggerConfig,
}

/// Configuration for Swagger UI
#[derive(Debug, Clone)]
pub struct SwaggerConfig {
    /// Server host
    pub host: String,
    /// Server port
    pub port: u16,
    /// Page title
    pub title: String,
    /// Custom CSS
    pub custom_css: Option<String>,
    /// Custom JavaScript
    pub custom_js: Option<String>,
    /// OAuth configuration
    pub oauth: Option<OAuthConfig>,
}

/// OAuth configuration for Swagger UI
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub realm: Option<String>,
    pub app_name: String,
    pub scopes: Vec<String>,
}

impl SwaggerUi {
    /// Create new Swagger UI server
    pub fn new(spec: OpenApiSpec, config: SwaggerConfig) -> Self {
        Self {
            spec: Arc::new(spec),
            config,
        }
    }
    
    /// Get the specification
    pub fn specification(&self) -> Option<&OpenApiSpec> {
        Some(&self.spec)
    }
    
    /// Get the configuration  
    pub fn config(&self) -> &SwaggerConfig {
        &self.config
    }

    /// Start the Swagger UI server using axum
    pub async fn serve(&self) -> OpenApiResult<()> {
        let state = SwaggerState {
            spec: Arc::clone(&self.spec),
            config: self.config.clone(),
        };

        // Build the router
        let app = Router::new()
            .route("/", get(serve_index))
            .route("/api-spec.json", get(serve_spec))
            .route("/static/*path", get(serve_static))
            .layer(CorsLayer::permissive())
            .with_state(state);

        // Bind to the configured address
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = tokio::net::TcpListener::bind(&addr).await
            .map_err(|e| OpenApiError::generic(format!("Failed to bind to {}: {}", addr, e)))?;

        println!("🚀 Swagger UI server running at http://{}", addr);
        println!("📖 API documentation available at http://{}/", addr);

        // Start the server
        axum::serve(listener, app)
            .await
            .map_err(|e| OpenApiError::generic(format!("Server error: {}", e)))?;

        Ok(())
    }
}

// Axum handlers for Swagger UI routes

/// Serve the main Swagger UI index page
async fn serve_index(State(state): State<SwaggerState>) -> Html<String> {
    let html = SwaggerUi::generate_index_html(&state.config);
    Html(html)
}

/// Serve the OpenAPI specification JSON
async fn serve_spec(State(state): State<SwaggerState>) -> Result<Json<OpenApiSpec>, (StatusCode, String)> {
    Ok(Json((*state.spec).clone()))
}

/// Serve static assets (CSS, JS)
async fn serve_static(Path(path): Path<String>) -> Result<Response<Body>, (StatusCode, &'static str)> {
    let (content_type, body) = match path.as_str() {
        "swagger-ui-bundle.js" => ("application/javascript", "// Swagger UI Bundle - placeholder for CDN content"),
        "swagger-ui-standalone-preset.js" => ("application/javascript", "// Swagger UI Preset - placeholder for CDN content"),  
        "swagger-ui.css" => ("text/css", "/* Swagger UI CSS - placeholder for CDN content */"),
        _ => return Err((StatusCode::NOT_FOUND, "Not Found")),
    };

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(body))
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response"))?;

    Ok(response)
}

impl SwaggerUi {

    /// Generate the main HTML page
    fn generate_index_html(config: &SwaggerConfig) -> String {
        let oauth_config = if let Some(oauth) = &config.oauth {
            format!(
                r#"
                ui.initOAuth({{
                    clientId: "{}",
                    realm: "{}",
                    appName: "{}",
                    scopes: [{}]
                }});
                "#,
                oauth.client_id,
                oauth.realm.as_deref().unwrap_or(""),
                oauth.app_name,
                oauth.scopes.iter().map(|s| format!(r#""{}""#, s)).collect::<Vec<_>>().join(", ")
            )
        } else {
            String::new()
        };

        let custom_css = config.custom_css.as_deref().unwrap_or("");
        let custom_js = config.custom_js.as_deref().unwrap_or("");

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <link rel="stylesheet" type="text/css" href="/static/swagger-ui.css" />
    <style>
        html {{
            box-sizing: border-box;
            overflow: -moz-scrollbars-vertical;
            overflow-y: scroll;
        }}

        *, *:before, *:after {{
            box-sizing: inherit;
        }}

        body {{
            margin:0;
            background: #fafafa;
        }}

        {}
    </style>
</head>
<body>
    <div id="swagger-ui"></div>
    
    <script src="/static/swagger-ui-bundle.js"></script>
    <script src="/static/swagger-ui-standalone-preset.js"></script>
    <script>
        window.onload = function() {{
            const ui = SwaggerUIBundle({{
                url: '/api-spec.json',
                dom_id: '#swagger-ui',
                deepLinking: true,
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIStandalonePreset
                ],
                plugins: [
                    SwaggerUIBundle.plugins.DownloadUrl
                ],
                layout: "StandaloneLayout",
                validatorUrl: null,
                tryItOutEnabled: true,
                filter: true,
                supportedSubmitMethods: ['get', 'post', 'put', 'delete', 'patch'],
                onComplete: function() {{
                    console.log("Swagger UI loaded successfully");
                }},
                onFailure: function(error) {{
                    console.error("Swagger UI failed to load:", error);
                }}
            }});

            {}

            window.ui = ui;
        }};

        {}
    </script>
</body>
</html>"#,
            config.title,
            custom_css,
            oauth_config,
            custom_js
        )
    }

    /// Generate static Swagger UI HTML file
    pub fn generate_static_html(spec: &OpenApiSpec, config: &SwaggerConfig) -> OpenApiResult<String> {
        let spec_json = serde_json::to_string(spec)
            .map_err(|e| OpenApiError::export_error(format!("Failed to serialize spec: {}", e)))?;

        let oauth_config = if let Some(oauth) = &config.oauth {
            format!(
                r#"
                ui.initOAuth({{
                    clientId: "{}",
                    realm: "{}",
                    appName: "{}",
                    scopes: [{}]
                }});
                "#,
                oauth.client_id,
                oauth.realm.as_deref().unwrap_or(""),
                oauth.app_name,
                oauth.scopes.iter().map(|s| format!(r#""{}""#, s)).collect::<Vec<_>>().join(", ")
            )
        } else {
            String::new()
        };

        let custom_css = config.custom_css.as_deref().unwrap_or("");
        let custom_js = config.custom_js.as_deref().unwrap_or("");

        Ok(format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@5.9.0/swagger-ui.css" />
    <style>
        html {{
            box-sizing: border-box;
            overflow: -moz-scrollbars-vertical;
            overflow-y: scroll;
        }}

        *, *:before, *:after {{
            box-sizing: inherit;
        }}

        body {{
            margin:0;
            background: #fafafa;
        }}

        {}
    </style>
</head>
<body>
    <div id="swagger-ui"></div>
    
    <script src="https://unpkg.com/swagger-ui-dist@5.9.0/swagger-ui-bundle.js"></script>
    <script src="https://unpkg.com/swagger-ui-dist@5.9.0/swagger-ui-standalone-preset.js"></script>
    <script>
        const spec = {};

        window.onload = function() {{
            const ui = SwaggerUIBundle({{
                spec: spec,
                dom_id: '#swagger-ui',
                deepLinking: true,
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIStandalonePreset
                ],
                plugins: [
                    SwaggerUIBundle.plugins.DownloadUrl
                ],
                layout: "StandaloneLayout",
                validatorUrl: null,
                tryItOutEnabled: true,
                filter: true,
                supportedSubmitMethods: ['get', 'post', 'put', 'delete', 'patch'],
                onComplete: function() {{
                    console.log("Swagger UI loaded successfully");
                }},
                onFailure: function(error) {{
                    console.error("Swagger UI failed to load:", error);
                }}
            }});

            {}

            window.ui = ui;
        }};

        {}
    </script>
</body>
</html>"#,
            config.title,
            custom_css,
            spec_json,
            oauth_config,
            custom_js
        ))
    }
}


impl Default for SwaggerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            title: "API Documentation".to_string(),
            custom_css: None,
            custom_js: None,
            oauth: None,
        }
    }
}

impl SwaggerConfig {
    /// Create new configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set server host and port
    pub fn with_server(mut self, host: &str, port: u16) -> Self {
        self.host = host.to_string();
        self.port = port;
        self
    }

    /// Set page title
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    /// Add custom CSS
    pub fn with_custom_css(mut self, css: &str) -> Self {
        self.custom_css = Some(css.to_string());
        self
    }

    /// Add custom JavaScript
    pub fn with_custom_js(mut self, js: &str) -> Self {
        self.custom_js = Some(js.to_string());
        self
    }

    /// Configure OAuth
    pub fn with_oauth(mut self, oauth: OAuthConfig) -> Self {
        self.oauth = Some(oauth);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specification::OpenApiSpec;

    #[test]
    fn test_swagger_config_creation() {
        let config = SwaggerConfig::new()
            .with_server("localhost", 3000)
            .with_title("My API");

        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 3000);
        assert_eq!(config.title, "My API");
    }

    #[test]
    fn test_static_html_generation() {
        let spec = OpenApiSpec::new("Test API", "1.0.0");
        let config = SwaggerConfig::new().with_title("Test API Documentation");

        let html = SwaggerUi::generate_static_html(&spec, &config).unwrap();
        
        assert!(html.contains("Test API Documentation"));
        assert!(html.contains("swagger-ui"));
        assert!(html.contains("SwaggerUIBundle"));
    }

    #[test]
    fn test_swagger_ui_creation() {
        let spec = OpenApiSpec::new("Test API", "1.0.0");
        let config = SwaggerConfig::default();
        
        let swagger_ui = SwaggerUi::new(spec, config);
        assert_eq!(swagger_ui.config.host, "127.0.0.1");
        assert_eq!(swagger_ui.config.port, 8080);
    }
}