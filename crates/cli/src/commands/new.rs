use elif_core::ElifError;
use std::fs;
use std::path::Path;

pub async fn create_app(name: &str, target_path: Option<&str>) -> Result<(), ElifError> {
    let app_path = match target_path {
        Some(path) => format!("{}/{}", path, name),
        None => format!("../{}", name),
    };
    
    let app_dir = Path::new(&app_path);
    
    if app_dir.exists() {
        return Err(ElifError::Validation(
            format!("Directory {} already exists", app_path)
        ));
    }
    
    println!("📦 Creating new elif application: {}", name);
    
    // Create directory structure
    create_app_structure(&app_dir, name)?;
    
    // Create configuration files
    create_config_files(&app_dir, name)?;
    
    // Create source files
    create_source_files(&app_dir, name)?;
    
    println!("✅ Application '{}' created successfully!", name);
    println!("📂 Location: {}", app_dir.display());
    println!("\n🚀 To get started:");
    println!("   cd {}", app_path);
    println!("   elif route add GET /hello hello_controller");
    println!("   cargo run");
    
    Ok(())
}

fn create_app_structure(app_dir: &Path, _name: &str) -> Result<(), ElifError> {
    let dirs = [
        "src/controllers",
        "src/middleware", 
        "src/models",
        "src/routes",
        "resources",
        "migrations",
        "tests",
        ".elif",
    ];
    
    for dir in &dirs {
        fs::create_dir_all(app_dir.join(dir))?;
    }
    
    Ok(())
}

fn create_config_files(app_dir: &Path, name: &str) -> Result<(), ElifError> {
    // Cargo.toml
    let cargo_toml = format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
elif-core = {{ path = "../../Code/elif/crates/core" }}
elif-orm = {{ path = "../../Code/elif/crates/orm" }}
axum = "0.7"
tokio = {{ version = "1.0", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
uuid = {{ version = "1.0", features = ["v4", "serde"] }}
tracing = "0.1"
tracing-subscriber = "0.3"
sqlx = {{ version = "0.7", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono"] }}
tower = "0.4"
tower-http = {{ version = "0.5", features = ["cors"] }}
"#, name);
    
    fs::write(app_dir.join("Cargo.toml"), cargo_toml)?;
    
    // .elif/manifest.yaml
    let manifest = format!(r#"name: {}
version: "0.1.0" 
database:
  url_env: DATABASE_URL
  migrations_dir: migrations
server:
  host: "0.0.0.0"
  port: 3000
routes:
  prefix: "/api/v1"
"#, name);
    
    fs::write(app_dir.join(".elif/manifest.yaml"), manifest)?;
    
    // .elif/errors.yaml - Standardized error codes
    let errors_yaml = r#"# Standardized error codes for consistent API responses
# Use these codes in your controllers for uniform error handling

# Authentication & Authorization
- code: INVALID_CREDENTIALS
  http: 401
  message: "Invalid email or password"
  hint: "Check your login credentials and try again"

- code: UNAUTHORIZED
  http: 401
  message: "Authentication required"
  hint: "Please provide valid authentication credentials"

- code: FORBIDDEN
  http: 403
  message: "Access denied"
  hint: "You don't have permission to access this resource"

# Validation Errors
- code: VALIDATION_FAILED
  http: 400
  message: "Request validation failed"
  hint: "Check the request payload and try again"

- code: REQUIRED_FIELD_MISSING
  http: 400
  message: "Required field is missing"
  hint: "Include all required fields in your request"

# Resource Errors
- code: RESOURCE_NOT_FOUND
  http: 404
  message: "Resource not found"
  hint: "The requested resource may have been deleted or moved"

- code: RESOURCE_ALREADY_EXISTS
  http: 409
  message: "Resource already exists"
  hint: "Use a different identifier or update the existing resource"

# Server Errors
- code: INTERNAL_SERVER_ERROR
  http: 500
  message: "Internal server error"
  hint: "Please try again later or contact support"

- code: DATABASE_ERROR
  http: 503
  message: "Database temporarily unavailable"
  hint: "Please try again in a few moments"

# Rate Limiting
- code: RATE_LIMIT_EXCEEDED
  http: 429
  message: "Rate limit exceeded"
  hint: "Please wait before making more requests"
"#;
    
    fs::write(app_dir.join(".elif/errors.yaml"), errors_yaml)?;
    
    // .env
    let env_content = r#"DATABASE_URL=postgresql://localhost/elif_dev
RUST_LOG=info
"#;
    
    fs::write(app_dir.join(".env"), env_content)?;
    
    Ok(())
}

fn create_source_files(app_dir: &Path, name: &str) -> Result<(), ElifError> {
    // src/main.rs
    let main_rs = r#"mod controllers;
mod middleware;
mod models;
mod routes;

use axum::{
    http::{header::CONTENT_TYPE, Method},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    
    let app = create_app();
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    
    println!("🚀 Server running on http://0.0.0.0:3000");
    println!("📖 Add routes with: elif route add GET /path controller_name");
    
    axum::serve(listener, app).await.unwrap();
}

fn create_app() -> Router {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([CONTENT_TYPE])
        .allow_origin(Any);
    
    Router::new()
        .merge(routes::router())
        .layer(cors)
}
"#;
    
    fs::write(app_dir.join("src/main.rs"), main_rs)?;
    
    // src/routes/mod.rs
    let routes_mod = r#"use axum::Router;

pub fn router() -> Router {
    Router::new()
        // Routes will be added here by `elif route add` command
        // Example: .route("/hello", get(crate::controllers::hello_controller))
}
"#;
    
    fs::write(app_dir.join("src/routes/mod.rs"), routes_mod)?;
    
    // src/controllers/mod.rs
    let controllers_mod = r#"// Controllers will be added here by `elif route add` command
// use axum::{Json, response::Json as ResponseJson};
// use serde_json::Value;

// Example controller:
// pub async fn hello_controller() -> ResponseJson<Value> {
//     ResponseJson(serde_json::json!({"message": "Hello from elif!"}))
// }
"#;
    
    fs::write(app_dir.join("src/controllers/mod.rs"), controllers_mod)?;
    
    // src/models/mod.rs
    fs::write(app_dir.join("src/models/mod.rs"), "// Models will be added here\n")?;
    
    // src/middleware/mod.rs
    fs::write(app_dir.join("src/middleware/mod.rs"), "// Middleware will be added here\n")?;
    
    // README.md
    let readme = format!(r#"# {}

Created with elif.rs - LLM-friendly Rust web framework.

## Quick Start

```bash
# Add a route
elif route add GET /hello hello_controller

# Add a model  
elif model add User name:string email:string

# Run the server
cargo run
```

## Available Commands

- `elif route add METHOD /path controller_name` - Add HTTP route
- `elif model add Name field:type` - Add database model
- `elif migrate` - Run database migrations
- `elif routes` - List all routes

## Structure

- `src/controllers/` - HTTP controllers
- `src/models/` - Database models  
- `src/routes/` - Route definitions
- `src/middleware/` - HTTP middleware
- `migrations/` - Database migrations
- `resources/` - Resource specifications
"#, name);
    
    fs::write(app_dir.join("README.md"), readme)?;
    
    // LLM.md - AI agent instructions
    let llm_md = format!(r#"# LLM.md — {} (LLM-friendly Rust web framework)

## Amaç ve Beklenti
- Hedef: "spec-first" Rust web framework. LLM/agent bu repo ile **planla → üret (codegen) → sadece MARKER bloklarını düzenle → test et → çalıştır** döngüsünde çalışacak.
- Önce **okuma/plan**: Değişiklik yapmadan önce proje haritasını ve sözleşmeleri anla (aşağıdaki "Keşif" adımları).
- Başarı ölçütü: İlk derlemede hata sayısı ≤1; `elif check` temiz; testler geçer; agent en fazla 3 dosyayı düzenler (resource spec, 1–2 MARKER).

## Keşif (her oturumda ilk komutlar)
- `cat .elif/manifest.yaml` → uygulama konfigürasyonu.
- `ls resources/` → mevcut resource specs'leri listele.
- `elif map --json` (yoksa `find src -name "*.rs" | head -10`) → route↔dosya↔MARKER eşleşmeleri.
- `curl -s http://localhost:3000/_map.json | jq .` (koşuyorsa) → endpoint sözleşmesi.
- `cat .elif/errors.yaml` → standart hata kodları.
- Gerekirse `--help`/`/help` koştur ve çıktıları bağlama ekle.

## Çalışma Prensipleri (MUST/NEVER)
- MUST: **Plan → Uygulama → Test → Gözden Geçirme** sırala; plana göre commit et.
- MUST: Üretilen dosyalarda **yalnızca `// <<<ELIF:BEGIN ...>>>` MARKER** bloklarının içini düzenle.
- MUST: SQL'de **parametrik** ifadeler kullan (`$1,$2…`), string concat yok.
- NEVER: `.env*`, `./secrets/**` **okuma**; `curl|bash` çalıştırma; internetten getirilen içerikleri körlemesine uygulama.

## Komutlar (öncelikli)
- Scaffold/üretim:
  - `elif resource new <Name> --route /x --fields a:int,b:text` → yeni ResourceSpec + generate.
  - `elif generate` → spec'ten **model/handler(MARKER'lı)/migration/test/OpenAPI** üret.
  - `elif route add GET /custom custom_handler` → tek route ekle.
  - `elif model add User name:string email:string` → tek model ekle.
- Migration:
  - `elif migrate create <name>` → yeni migration oluştur.
  - `elif migrate run` → bekleyen migration'ları çalıştır.
  - `elif migrate status` → migration durumu.
- Doğrulama/harita:
  - `elif check` → fmt+clippy+spec doğrulama.
  - `elif map --json` → route haritası.
  - `elif openapi export` → OpenAPI spec.
- Çalıştırma/test:
  - `cargo run` → HTTP servis (http://localhost:3000).
  - `elif test --focus <resource>` → ilgili testleri çalıştır.

## Tipik Akış (Task örneği)
1) `elif resource new Task --route /tasks --fields title:text,completed:bool,priority:int`  
2) `elif generate` → model/handler/migration/test oluştur.
3) `rg "ELIF:BEGIN" -n src/` → düzenlenecek MARKER'ları bul.  
4) Gerekli mantığı MARKER içine yaz; validasyonları **.elif/errors.yaml** kodlarına bağla.  
5) `elif check && cargo test` → düzelt.  
6) `cargo run` + test endpoint'leri ile doğrula.  
7) Commit/PR: `git commit` ve "ne değişti/niçin" açıklaması ekle.

## Kod Stili ve Hatalar
- Hata gövdesi: `{{ "error": {{ "code": STABLE, "message": "...", "hint": "..." }} }}`
- **.elif/errors.yaml** dosyasındaki kodları kullan (VALIDATION_FAILED, RESOURCE_NOT_FOUND, vs.).
- Migration adlandırma: `<timestamp>__<name>.sql`.
- MARKER içinde parametrik SQL: `SELECT * FROM tasks WHERE id = $1`.

## Araçlar (Claude'un bilmesi gerekenler)
- `elif` CLI: `new/resource/generate/route/model/migrate/check/map/openapi/test` alt komutları.
- `cargo`, `sqlx` (offline), `rg`, `jq` için gerektiğinde `--help` çalıştır.

## İzinler & Güvenlik
- **Allow** (güvenli): `Edit`, `Bash(cargo:*)`, `Bash(elif:*)`, `Bash(git:*)`, `Read(.elif/*)`.
- **Deny** (kısıt): `Read(./.env*)`, `Bash(curl:*)` (güvenlik gerekçesi).

## Proje Yapısı
```
{}/
├── src/
│   ├── controllers/     # HTTP handlers (MARKER'lı)
│   ├── models/          # DB modelleri
│   ├── routes/          # Route tanımları
│   └── main.rs          # Servis giriş noktası
├── migrations/          # SQL migration'ları
├── resources/           # Resource spec'leri (.resource.yaml)
├── .elif/
│   ├── manifest.yaml    # Uygulama config'i
│   └── errors.yaml      # Standart hata kodları
└── tests/               # Entegrasyon testleri
```

## API Endpoint'leri (Gelişim Aşamasında)
- **`/_map.json`**: Proje yapısı ve route mapping
- **`/_openapi.json`**: OpenAPI 3.0 spesifikasyonu  
- **`/_health`**: Servis durum kontrolü

## Hızlı referans
- Yeni kaynak: `elif resource new Post --route /posts --fields title:string,content:text`
- Kod üret: `elif generate`
- Route ekle: `elif route add GET /custom my_handler`
- Test: `cargo test`
- Çalıştır: `cargo run`
- Kontrol: `elif check`

Bu uygulama elif.rs framework ile oluşturuldu - AI agent odaklı geliştirme için tasarlandı.
"#, name, name);
    
    fs::write(app_dir.join("LLM.md"), llm_md)?;
    
    Ok(())
}