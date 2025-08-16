/*!
Swagger UI integration for interactive API documentation.

This module provides functionality to serve interactive Swagger UI documentation
for OpenAPI specifications.
*/

use crate::{
    error::{OpenApiError, OpenApiResult},
    specification::OpenApiSpec,
};
use std::collections::HashMap;
use tokio::net::TcpListener;
use std::sync::Arc;

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

    /// Start the Swagger UI server
    pub async fn serve(&self) -> OpenApiResult<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await
            .map_err(|e| OpenApiError::generic(format!("Failed to bind to {}: {}", addr, e)))?;

        println!("🚀 Swagger UI server running at http://{}", addr);
        println!("📖 API documentation available at http://{}/", addr);

        loop {
            let (stream, _) = listener.accept().await
                .map_err(|e| OpenApiError::generic(format!("Failed to accept connection: {}", e)))?;

            let spec = Arc::clone(&self.spec);
            let config = self.config.clone();

            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, spec, config).await {
                    eprintln!("Error handling connection: {}", e);
                }
            });
        }
    }

    /// Handle individual HTTP connection
    async fn handle_connection(
        mut stream: tokio::net::TcpStream,
        spec: Arc<OpenApiSpec>,
        config: SwaggerConfig,
    ) -> OpenApiResult<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer).await
            .map_err(|e| OpenApiError::generic(format!("Failed to read request: {}", e)))?;

        let request = String::from_utf8_lossy(&buffer[..n]);
        let lines: Vec<&str> = request.lines().collect();
        
        if let Some(request_line) = lines.first() {
            let parts: Vec<&str> = request_line.split_whitespace().collect();
            if parts.len() >= 2 {
                let method = parts[0];
                let path = parts[1];

                let response = match (method, path) {
                    ("GET", "/") => Self::serve_index(&config),
                    ("GET", "/api-spec.json") => Self::serve_spec(&spec),
                    ("GET", path) if path.starts_with("/static/") => Self::serve_static(path),
                    _ => Self::serve_404(),
                };

                let http_response = format!(
                    "HTTP/1.1 {}\r\nContent-Length: {}\r\nContent-Type: {}\r\n\r\n{}",
                    response.status,
                    response.body.len(),
                    response.content_type,
                    response.body
                );

                stream.write_all(http_response.as_bytes()).await
                    .map_err(|e| OpenApiError::generic(format!("Failed to write response: {}", e)))?;
            }
        }

        Ok(())
    }

    /// Serve the main Swagger UI index page
    fn serve_index(config: &SwaggerConfig) -> HttpResponse {
        let html = Self::generate_index_html(config);
        HttpResponse {
            status: "200 OK".to_string(),
            content_type: "text/html".to_string(),
            body: html,
        }
    }

    /// Serve the OpenAPI specification
    fn serve_spec(spec: &OpenApiSpec) -> HttpResponse {
        match serde_json::to_string_pretty(spec) {
            Ok(json) => HttpResponse {
                status: "200 OK".to_string(),
                content_type: "application/json".to_string(),
                body: json,
            },
            Err(_) => HttpResponse {
                status: "500 Internal Server Error".to_string(),
                content_type: "text/plain".to_string(),
                body: "Failed to serialize OpenAPI specification".to_string(),
            },
        }
    }

    /// Serve static assets (CSS, JS)
    fn serve_static(path: &str) -> HttpResponse {
        match path {
            "/static/swagger-ui-bundle.js" => HttpResponse {
                status: "200 OK".to_string(),
                content_type: "application/javascript".to_string(),
                body: "// Swagger UI Bundle - placeholder".to_string(),
            },
            "/static/swagger-ui-standalone-preset.js" => HttpResponse {
                status: "200 OK".to_string(),
                content_type: "application/javascript".to_string(),
                body: "// Swagger UI Preset - placeholder".to_string(),
            },
            "/static/swagger-ui.css" => HttpResponse {
                status: "200 OK".to_string(),
                content_type: "text/css".to_string(),
                body: "/* Swagger UI CSS - placeholder */".to_string(),
            },
            _ => Self::serve_404(),
        }
    }

    /// Serve 404 response
    fn serve_404() -> HttpResponse {
        HttpResponse {
            status: "404 Not Found".to_string(),
            content_type: "text/plain".to_string(),
            body: "Not Found".to_string(),
        }
    }

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

/// HTTP response structure
struct HttpResponse {
    status: String,
    content_type: String,
    body: String,
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