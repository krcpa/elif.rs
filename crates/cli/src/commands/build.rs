use elif_core::ElifError;
use std::process::Command;
use std::env;
use std::path::Path;

pub async fn run(release: bool, target: &str, optimizations: Vec<String>) -> Result<(), ElifError> {
    println!("🔨 Building elif.rs application...");
    
    // Validate target
    let valid_targets = ["native", "docker", "wasm"];
    if !valid_targets.contains(&target) {
        return Err(ElifError::Validation {
            message: format!("Invalid build target: {}. Valid targets are: {}", target, valid_targets.join(", ")),
        });
    }

    // Check if we're in an elif.rs project
    if !Path::new("Cargo.toml").exists() {
        return Err(ElifError::Validation {
            message: "Not in a Rust project directory. Run this command from the root of your elif.rs project".to_string(),
        });
    }

    println!("   📦 Target: {}", target);
    println!("   🚀 Release mode: {}", if release { "enabled" } else { "disabled" });
    
    if !optimizations.is_empty() {
        println!("   ⚡ Optimizations: {}", optimizations.join(", "));
    }

    // Apply optimizations as environment variables
    apply_optimizations(&optimizations)?;

    match target {
        "native" => build_native(release).await,
        "docker" => build_docker(release).await,
        "wasm" => build_wasm(release).await,
        _ => unreachable!("Target validation should prevent this"),
    }
}

fn apply_optimizations(optimizations: &[String]) -> Result<(), ElifError> {
    for opt in optimizations {
        match opt.as_str() {
            "lto" => {
                env::set_var("CARGO_PROFILE_RELEASE_LTO", "true");
                println!("   ✅ Enabled Link-Time Optimization");
            }
            "strip" => {
                env::set_var("CARGO_PROFILE_RELEASE_STRIP", "true");
                println!("   ✅ Enabled binary stripping");
            }
            "size" => {
                env::set_var("CARGO_PROFILE_RELEASE_OPT_LEVEL", "s");
                println!("   ✅ Optimizing for size");
            }
            "speed" => {
                env::set_var("CARGO_PROFILE_RELEASE_OPT_LEVEL", "3");
                println!("   ✅ Optimizing for speed");
            }
            _ => {
                return Err(ElifError::Validation {
                    message: format!("Unknown optimization: {}. Valid optimizations: lto, strip, size, speed", opt),
                });
            }
        }
    }
    Ok(())
}

async fn build_native(release: bool) -> Result<(), ElifError> {
    println!("🏗️ Building native binary...");
    
    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    
    if release {
        cmd.arg("--release");
    }
    
    println!("   Running: cargo build{}", if release { " --release" } else { "" });
    
    let output = cmd.output().map_err(|e| ElifError::Io(e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ElifError::SystemError {
            message: format!("Build failed:\n{}", stderr),
            source: None,
        });
    }
    
    let target_dir = if release { "target/release" } else { "target/debug" };
    println!("✅ Build completed successfully!");
    println!("   Binary location: {}/", target_dir);
    
    Ok(())
}

async fn build_docker(release: bool) -> Result<(), ElifError> {
    println!("🐳 Building Docker image...");
    
    if !Path::new("Dockerfile").exists() {
        println!("   📝 Creating optimized Dockerfile...");
        create_dockerfile(release).await?;
    }
    
    let image_tag = format!("elifrs-app:{}", if release { "latest" } else { "debug" });
    
    let mut cmd = Command::new("docker");
    cmd.args(&["build", "-t", &image_tag, "."]);
    
    if release {
        cmd.args(&["--build-arg", "RUST_PROFILE=release"]);
    } else {
        cmd.args(&["--build-arg", "RUST_PROFILE=debug"]);
    }
    
    println!("   Running: docker build -t {} .", image_tag);
    
    let output = cmd.output().map_err(|e| ElifError::Io(e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ElifError::SystemError {
            message: format!("Docker build failed:\n{}", stderr),
            source: None,
        });
    }
    
    println!("✅ Docker image built successfully!");
    println!("   Image tag: {}", image_tag);
    
    Ok(())
}

async fn build_wasm(release: bool) -> Result<(), ElifError> {
    println!("🕸️ Building WebAssembly target...");
    
    // Check if wasm32-unknown-unknown target is installed
    let check_target = Command::new("rustup")
        .args(&["target", "list", "--installed"])
        .output()
        .map_err(|e| ElifError::Io(e))?;
    
    let targets = String::from_utf8_lossy(&check_target.stdout);
    if !targets.contains("wasm32-unknown-unknown") {
        println!("   📦 Installing wasm32-unknown-unknown target...");
        let install = Command::new("rustup")
            .args(&["target", "add", "wasm32-unknown-unknown"])
            .output()
            .map_err(|e| ElifError::Io(e))?;
        
        if !install.status.success() {
            return Err(ElifError::SystemError {
                message: "Failed to install WASM target".to_string(),
                source: None,
            });
        }
    }
    
    let mut cmd = Command::new("cargo");
    cmd.args(&["build", "--target", "wasm32-unknown-unknown"]);
    
    if release {
        cmd.arg("--release");
    }
    
    let output = cmd.output().map_err(|e| ElifError::Io(e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ElifError::SystemError {
            message: format!("WASM build failed:\n{}", stderr),
            source: None,
        });
    }
    
    let target_dir = if release { 
        "target/wasm32-unknown-unknown/release" 
    } else { 
        "target/wasm32-unknown-unknown/debug" 
    };
    
    println!("✅ WASM build completed successfully!");
    println!("   Binary location: {}/", target_dir);
    
    Ok(())
}

async fn create_dockerfile(release: bool) -> Result<(), ElifError> {
    let dockerfile_content = format!(r#"# Multi-stage build for elif.rs application
FROM rust:1.75-slim as builder

WORKDIR /app
COPY . .

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Build the application
ARG RUST_PROFILE={}
RUN cargo build --profile ${{RUST_PROFILE}}

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary
COPY --from=builder /app/target/{}/elifrs /usr/local/bin/

# Create non-root user
RUN useradd -r -s /bin/false elifrs

# Switch to non-root user
USER elifrs

EXPOSE 3000

CMD ["elifrs", "serve"]
"#, 
    if release { "release" } else { "debug" },
    if release { "release" } else { "debug" }
    );
    
    tokio::fs::write("Dockerfile", dockerfile_content)
        .await
        .map_err(|e| ElifError::Io(e))?;
    
    println!("   ✅ Generated optimized Dockerfile");
    
    Ok(())
}
