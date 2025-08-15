use elif_http::{ElifRouter, ElifJson, JsonResponse};
use elif_introspect::MapGenerator;
use serde_json::Value;
use std::path::PathBuf;
use utoipa::OpenApi;

pub fn router() -> ElifRouter {
    ElifRouter::new()
        .get("/_map.json", map_handler)
        .get("/_openapi.json", openapi_handler)
        .get("/_health", health_handler)
}

async fn map_handler() -> ElifJson<Value> {
    let project_root = PathBuf::from("../../");
    let generator = MapGenerator::new(project_root);
    
    match generator.generate() {
        Ok(map) => ElifJson(serde_json::to_value(map).unwrap_or_default()),
        Err(_) => ElifJson(serde_json::json!({
            "routes": [],
            "models": [],
            "specs": []
        }))
    }
}

async fn openapi_handler() -> ElifJson<Value> {
    ElifJson(create_openapi_spec())
}

async fn health_handler() -> ElifJson<Value> {
    ElifJson(serde_json::json!({
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