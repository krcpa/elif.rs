use axum::{
    response::Json,
    routing::get,
    Router,
};
use elif_introspect::MapGenerator;
use serde_json::Value;
use std::path::PathBuf;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub fn router() -> Router {
    Router::new()
        .route("/_map.json", get(map_handler))
        .route("/_openapi.json", get(openapi_handler))
        .route("/_health", get(health_handler))
        .merge(SwaggerUi::new("/_ui").url("/_openapi.json", create_openapi_doc()))
}

async fn map_handler() -> Json<Value> {
    let project_root = PathBuf::from("../../");
    let generator = MapGenerator::new(project_root);
    
    match generator.generate() {
        Ok(map) => Json(serde_json::to_value(map).unwrap_or_default()),
        Err(_) => Json(serde_json::json!({
            "routes": [],
            "models": [],
            "specs": []
        }))
    }
}

async fn openapi_handler() -> Json<Value> {
    Json(create_openapi_spec())
}

async fn health_handler() -> Json<Value> {
    Json(serde_json::json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

#[derive(OpenApi)]
#[openapi(
    paths(),
    components()
)]
struct ApiDoc;

fn create_openapi_doc() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

fn create_openapi_spec() -> Value {
    serde_json::json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Elif API",
            "version": "0.1.0",
            "description": "LLM-friendly Rust web framework API"
        },
        "servers": [
            {
                "url": "http://localhost:8080",
                "description": "Development server"
            }
        ],
        "paths": {
            "/_map.json": {
                "get": {
                    "summary": "Get project map",
                    "responses": {
                        "200": {
                            "description": "Project introspection data",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object"
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/_health": {
                "get": {
                    "summary": "Health check",
                    "responses": {
                        "200": {
                            "description": "Service is healthy",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "status": {
                                                "type": "string"
                                            },
                                            "timestamp": {
                                                "type": "string",
                                                "format": "date-time"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
        "components": {
            "schemas": {}
        }
    })
}