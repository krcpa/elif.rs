use elif_core::ElifError;
use std::collections::HashMap;
use serde_json::{Value, to_string_pretty, from_str};
use std::fs;
use std::path::PathBuf;
use chrono::Utc;

use super::types::{
    EmailSendArgs, EmailCaptureArgs, EmailTestListArgs, EmailTestShowArgs, 
    EmailTestClearArgs, EmailTestExportArgs, CapturedEmail
};

/// Configure email capture to filesystem
pub async fn test_capture(args: EmailCaptureArgs) -> Result<(), ElifError> {
    if args.enable && args.disable {
        return Err(ElifError::Validation { message: "Cannot enable and disable capture at the same time".to_string() });
    }
    
    let capture_dir = get_capture_directory(args.dir)?;
    
    if args.enable {
        // Create capture directory
        fs::create_dir_all(&capture_dir)
            .map_err(|e| ElifError::Validation { message: format!("Failed to create capture directory: {}", e) })?;
        
        // Enable capture
        set_capture_enabled(true, &capture_dir).await?;
        println!("✅ Email capture enabled");
        println!("📁 Capture directory: {}", capture_dir.display());
    } else if args.disable {
        set_capture_enabled(false, &capture_dir).await?;
        println!("❌ Email capture disabled");
    } else {
        // Show current status
        let enabled = is_capture_enabled().await?;
        println!("📊 Email Capture Status: {}", if enabled { "✅ Enabled" } else { "❌ Disabled" });
        println!("📁 Capture directory: {}", capture_dir.display());
        
        if enabled {
            let emails = list_captured_emails(&capture_dir, None, None, None, 999999).await?;
            println!("📧 Captured emails: {}", emails.len());
        }
    }
    
    Ok(())
}

/// List captured emails
pub async fn test_list(args: EmailTestListArgs) -> Result<(), ElifError> {
    let capture_dir = get_capture_directory(None)?;
    
    if !capture_dir.exists() {
        println!("📂 No captured emails found. Enable capture with: elifrs email test capture --enable");
        return Ok(());
    }
    
    let emails = list_captured_emails(
        &capture_dir,
        args.to.as_deref(),
        args.subject.as_deref(),
        Some(args.limit),
        999999
    ).await?;
    
    if emails.is_empty() {
        println!("📭 No captured emails found");
        return Ok(());
    }
    
    println!("📧 Captured Emails ({} total):", emails.len());
    println!();
    
    for (i, email) in emails.iter().take(args.limit).enumerate() {
        if args.detailed {
            println!("📬 Email #{} ({})", i + 1, email.id);
            println!("  🕐 Time: {}", email.timestamp.format("%Y-%m-%d %H:%M:%S UTC"));
            println!("  📤 From: {}", email.from);
            println!("  📥 To: {}", email.to);
            println!("  📋 Subject: {}", email.subject);
            if let Some(template) = &email.template {
                println!("  🎨 Template: {}", template);
            }
            if email.body_html.is_some() {
                println!("  🌐 Format: HTML");
            } else {
                println!("  📄 Format: Text");
            }
            println!();
        } else {
            println!("{:2} | {} | {} -> {} | {}", 
                i + 1,
                email.timestamp.format("%m-%d %H:%M"),
                email.from,
                email.to,
                email.subject
            );
        }
    }
    
    Ok(())
}

/// Show specific captured email
pub async fn test_show(args: EmailTestShowArgs) -> Result<(), ElifError> {
    let capture_dir = get_capture_directory(None)?;
    
    if !capture_dir.exists() {
        return Err(ElifError::Validation { message: "No captured emails found. Enable capture first.".to_string() });
    }
    
    let email = get_captured_email(&capture_dir, &args.email_id).await?;
    
    if args.raw {
        // Show raw JSON
        println!("{}", to_string_pretty(&email)
            .map_err(|e| ElifError::Validation { message: format!("Failed to serialize email: {}", e) })?);
        return Ok(());
    }
    
    match args.part.as_deref() {
        Some("headers") => {
            println!("📋 Email Headers:");
            for (key, value) in &email.headers {
                println!("{}: {}", key, value);
            }
        }
        Some("text") => {
            if let Some(text) = &email.body_text {
                println!("📄 Text Body:");
                println!("{}", text);
            } else {
                println!("❌ No text body available");
            }
        }
        Some("html") => {
            if let Some(html) = &email.body_html {
                println!("🌐 HTML Body:");
                println!("{}", html);
            } else {
                println!("❌ No HTML body available");
            }
        }
        Some("attachments") => {
            println!("📎 Attachments: Not implemented yet");
        }
        _ => {
            // Show full email
            println!("📧 Email: {}", email.id);
            println!("🕐 Timestamp: {}", email.timestamp.format("%Y-%m-%d %H:%M:%S UTC"));
            println!("📤 From: {}", email.from);
            println!("📥 To: {}", email.to);
            println!("📋 Subject: {}", email.subject);
            
            if let Some(template) = &email.template {
                println!("🎨 Template: {}", template);
            }
            
            if !email.headers.is_empty() {
                println!("\n📋 Headers:");
                for (key, value) in &email.headers {
                    println!("  {}: {}", key, value);
                }
            }
            
            if let Some(text) = &email.body_text {
                println!("\n📄 Text Body:");
                println!("{}", text);
            }
            
            if let Some(html) = &email.body_html {
                println!("\n🌐 HTML Body:");
                println!("{}", html);
            }
            
            if let Some(context) = &email.context {
                if !context.is_empty() {
                    println!("\n🎯 Template Context:");
                    println!("{}", to_string_pretty(context)
                        .map_err(|e| ElifError::Validation { message: format!("Failed to serialize context: {}", e) })?);
                }
            }
        }
    }
    
    Ok(())
}

/// Clear captured emails
pub async fn test_clear(args: EmailTestClearArgs) -> Result<(), ElifError> {
    let capture_dir = get_capture_directory(None)?;
    
    if !capture_dir.exists() {
        println!("📂 No captured emails to clear");
        return Ok(());
    }
    
    let entries = fs::read_dir(&capture_dir)
        .map_err(|e| ElifError::Validation { message: format!("Failed to read capture directory: {}", e) })?;
    
    let mut cleared_count = 0;
    
    for entry in entries {
        let entry = entry.map_err(|e| ElifError::Validation { message: format!("Failed to read directory entry: {}", e) })?;
        let path = entry.path();
        
        if !path.extension().map_or(false, |ext| ext == "json") {
            continue;
        }
        
        if args.all {
            fs::remove_file(&path)
                .map_err(|e| ElifError::Validation { message: format!("Failed to remove file: {}", e) })?;
            cleared_count += 1;
        } else if let Some(days) = args.older_than {
            // Check file age
            let metadata = fs::metadata(&path)
                .map_err(|e| ElifError::Validation { message: format!("Failed to get file metadata: {}", e) })?;
            
            // Try creation time first, fall back to modification time for cross-platform compatibility
            let file_time = match metadata.created() {
                Ok(created) => created,
                Err(_) => {
                    // Log warning and use modification time as fallback
                    eprintln!("Warning: File creation time not available on this platform, using modification time for {}", path.display());
                    metadata.modified()
                        .map_err(|e| ElifError::Validation { message: format!("Failed to get file modification time: {}", e) })?
                }
            };
            
            let age = file_time.elapsed()
                .map_err(|_| ElifError::Validation { message: "Failed to calculate file age".to_string() })?;
            
            if age.as_secs() > (days as u64 * 24 * 60 * 60) {
                fs::remove_file(&path)
                    .map_err(|e| ElifError::Validation { message: format!("Failed to remove file: {}", e) })?;
                cleared_count += 1;
            }
        }
    }
    
    println!("🧹 Cleared {} captured emails", cleared_count);
    Ok(())
}

/// Export captured emails
pub async fn test_export(args: EmailTestExportArgs) -> Result<(), ElifError> {
    let capture_dir = get_capture_directory(None)?;
    
    if !capture_dir.exists() {
        return Err(ElifError::Validation { message: "No captured emails found".to_string() });
    }
    
    let emails = list_captured_emails(&capture_dir, None, None, None, 999999).await?;
    
    if emails.is_empty() {
        println!("📭 No emails to export");
        return Ok(());
    }
    
    let output_path = args.output.unwrap_or_else(|| {
        format!("emails_export_{}.{}", 
            Utc::now().format("%Y%m%d_%H%M%S"),
            match args.format.as_str() {
                "csv" => "csv",
                "mbox" => "mbox",
                _ => "json",
            }
        )
    });
    
    match args.format.as_str() {
        "json" => export_as_json(&emails, &output_path, args.include_body)?,
        "csv" => export_as_csv(&emails, &output_path, args.include_body)?,
        "mbox" => export_as_mbox(&emails, &output_path)?,
        _ => return Err(ElifError::Validation { message: format!("Unsupported export format: {}", args.format) }),
    }
    
    println!("📤 Exported {} emails to: {}", emails.len(), output_path);
    Ok(())
}

// Helper functions

pub fn get_capture_directory(custom_dir: Option<String>) -> Result<PathBuf, ElifError> {
    if let Some(dir) = custom_dir {
        Ok(PathBuf::from(dir))
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(".elif").join("email_capture"))
            .map_err(|e| ElifError::Validation { message: format!("Failed to get current directory: {}", e) })
    }
}

pub async fn is_capture_enabled() -> Result<bool, ElifError> {
    let config_path = get_capture_directory(None)?.join(".config");
    Ok(config_path.exists())
}

async fn set_capture_enabled(enabled: bool, capture_dir: &PathBuf) -> Result<(), ElifError> {
    let config_path = capture_dir.join(".config");
    
    if enabled {
        fs::write(&config_path, "enabled")
            .map_err(|e| ElifError::Validation { message: format!("Failed to write config: {}", e) })?;
    } else {
        if config_path.exists() {
            fs::remove_file(&config_path)
                .map_err(|e| ElifError::Validation { message: format!("Failed to remove config: {}", e) })?;
        }
    }
    
    Ok(())
}

pub async fn capture_email_to_filesystem(
    args: &EmailSendArgs,
    body_text: Option<String>,
    body_html: Option<String>,
    context_data: &HashMap<String, Value>
) -> Result<(), ElifError> {
    let capture_dir = get_capture_directory(None)?;
    
    let now = Utc::now();
    let email_id = format!("email_{}", now.timestamp_micros());
    let mut headers = HashMap::new();
    headers.insert("To".to_string(), args.to.clone());
    headers.insert("Subject".to_string(), args.subject.clone());
    headers.insert("From".to_string(), "test@elif.rs".to_string());
    
    let captured_email = CapturedEmail {
        id: email_id.clone(),
        timestamp: now,
        to: args.to.clone(),
        from: "test@elif.rs".to_string(),
        subject: args.subject.clone(),
        body_text,
        body_html,
        headers,
        template: args.template.clone(),
        context: if context_data.is_empty() { None } else { Some(context_data.clone()) },
    };
    
    let file_path = capture_dir.join(format!("{}.json", email_id));
    let json_content = to_string_pretty(&captured_email)
        .map_err(|e| ElifError::Validation { message: format!("Failed to serialize email: {}", e) })?;
    
    fs::write(&file_path, json_content)
        .map_err(|e| ElifError::Validation { message: format!("Failed to write email file: {}", e) })?;
    
    Ok(())
}

async fn list_captured_emails(
    capture_dir: &PathBuf,
    to_filter: Option<&str>,
    subject_filter: Option<&str>,
    limit: Option<usize>,
    _max_results: usize
) -> Result<Vec<CapturedEmail>, ElifError> {
    let entries = fs::read_dir(capture_dir)
        .map_err(|e| ElifError::Validation { message: format!("Failed to read capture directory: {}", e) })?;
    
    // Collect and sort file paths first (without reading contents)
    let mut file_paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| ElifError::Validation { message: format!("Failed to read directory entry: {}", e) })?;
        let path = entry.path();
        
        if path.extension().map_or(false, |ext| ext == "json") {
            file_paths.push(path);
        }
    }
    
    // Sort paths by modification time (newest first) - this is fast as it doesn't read file contents
    file_paths.sort_by(|a, b| {
        let a_meta = a.metadata().ok();
        let b_meta = b.metadata().ok();
        match (a_meta, b_meta) {
            (Some(a_meta), Some(b_meta)) => {
                b_meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                    .cmp(&a_meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH))
            }
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });
    
    // Only read the files we need based on limit
    let files_to_read = if let Some(limit) = limit {
        std::cmp::min(limit * 2, file_paths.len()) // Read a bit more to account for filtering
    } else {
        file_paths.len()
    };
    
    let mut emails = Vec::new();
    for path in file_paths.into_iter().take(files_to_read) {
        let content = fs::read_to_string(&path)
            .map_err(|e| ElifError::Validation { message: format!("Failed to read email file: {}", e) })?;
        
        let email: CapturedEmail = from_str(&content)
            .map_err(|e| ElifError::Validation { message: format!("Failed to parse email JSON: {}", e) })?;
        
        // Apply filters
        if let Some(to_filter) = to_filter {
            if !email.to.contains(to_filter) {
                continue;
            }
        }
        
        if let Some(subject_filter) = subject_filter {
            if !email.subject.contains(subject_filter) {
                continue;
            }
        }
        
        emails.push(email);
        
        // Early exit if we have enough emails after filtering
        if let Some(limit) = limit {
            if emails.len() >= limit {
                break;
            }
        }
    }
    
    // Sort by timestamp (newest first)
    emails.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    
    // Apply final limit
    if let Some(limit) = limit {
        emails.truncate(limit);
    }
    
    Ok(emails)
}

async fn get_captured_email_by_index(capture_dir: &PathBuf, index: usize) -> Result<CapturedEmail, ElifError> {
    let entries = fs::read_dir(capture_dir)
        .map_err(|e| ElifError::Validation { message: format!("Failed to read capture directory: {}", e) })?;
    
    // Collect and sort file paths first (without reading contents)
    let mut file_paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| ElifError::Validation { message: format!("Failed to read directory entry: {}", e) })?;
        let path = entry.path();
        
        if path.extension().map_or(false, |ext| ext == "json") {
            file_paths.push(path);
        }
    }
    
    // Sort paths by modification time (newest first)
    file_paths.sort_by(|a, b| {
        let a_meta = a.metadata().ok();
        let b_meta = b.metadata().ok();
        match (a_meta, b_meta) {
            (Some(a_meta), Some(b_meta)) => {
                b_meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                    .cmp(&a_meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH))
            }
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });
    
    // Check if index is valid
    if index >= file_paths.len() {
        return Err(ElifError::Validation { message: format!("Email index {} not found (only {} emails available)", index + 1, file_paths.len()) });
    }
    
    // Read only the single file at the specified index
    let file_path = &file_paths[index];
    let content = fs::read_to_string(file_path)
        .map_err(|e| ElifError::Validation { message: format!("Failed to read email file: {}", e) })?;
    
    from_str(&content)
        .map_err(|e| ElifError::Validation { message: format!("Failed to parse email JSON: {}", e) })
}

async fn get_captured_email(capture_dir: &PathBuf, email_id: &str) -> Result<CapturedEmail, ElifError> {
    // Try direct ID match first
    let file_path = capture_dir.join(format!("{}.json", email_id));
    
    if file_path.exists() {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| ElifError::Validation { message: format!("Failed to read email file: {}", e) })?;
        
        return from_str(&content)
            .map_err(|e| ElifError::Validation { message: format!("Failed to parse email JSON: {}", e) });
    }
    
    // Try index-based lookup (1-based)
    if let Ok(index) = email_id.parse::<usize>() {
        if index > 0 {
            return get_captured_email_by_index(capture_dir, index - 1).await;
        }
    }
    
    Err(ElifError::Validation { message: format!("Email not found: {}", email_id) })
}

fn export_as_json(emails: &[CapturedEmail], output_path: &str, include_body: bool) -> Result<(), ElifError> {
    let file = std::fs::File::create(output_path)
        .map_err(|e| ElifError::Validation { message: format!("Failed to create export file: {}", e) })?;
    
    let export_data: Vec<_> = if include_body {
        emails.iter().cloned().collect()
    } else {
        emails.iter().map(|e| CapturedEmail {
            id: e.id.clone(),
            timestamp: e.timestamp,
            to: e.to.clone(),
            from: e.from.clone(),
            subject: e.subject.clone(),
            body_text: None,
            body_html: None,
            headers: e.headers.clone(),
            template: e.template.clone(),
            context: None,
        }).collect()
    };
    
    // Stream JSON directly to file instead of building in memory
    serde_json::to_writer_pretty(file, &export_data)
        .map_err(|e| ElifError::Validation { message: format!("Failed to write JSON export: {}", e) })?;
    
    Ok(())
}

fn export_as_csv(emails: &[CapturedEmail], output_path: &str, include_body: bool) -> Result<(), ElifError> {
    let file = std::fs::File::create(output_path)
        .map_err(|e| ElifError::Validation { message: format!("Failed to create CSV file: {}", e) })?;
    
    let mut writer = csv::Writer::from_writer(file);
    
    // Write header
    if include_body {
        writer.write_record(&["timestamp", "from", "to", "subject", "template", "body_text", "body_html"])
            .map_err(|e| ElifError::Validation { message: format!("Failed to write CSV header: {}", e) })?;
    } else {
        writer.write_record(&["timestamp", "from", "to", "subject", "template"])
            .map_err(|e| ElifError::Validation { message: format!("Failed to write CSV header: {}", e) })?;
    }
    
    // Write rows directly to file stream
    for email in emails {
        let timestamp = email.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
        let template = email.template.as_deref().unwrap_or("");
        
        if include_body {
            let body_text = email.body_text.as_deref().unwrap_or("");
            let body_html = email.body_html.as_deref().unwrap_or("");
            
            writer.write_record(&[
                &timestamp,
                &email.from,
                &email.to,
                &email.subject,
                template,
                body_text,
                body_html
            ]).map_err(|e| ElifError::Validation { message: format!("Failed to write CSV row: {}", e) })?;
        } else {
            writer.write_record(&[
                &timestamp,
                &email.from,
                &email.to,
                &email.subject,
                template
            ]).map_err(|e| ElifError::Validation { message: format!("Failed to write CSV row: {}", e) })?;
        }
    }
    
    writer.flush()
        .map_err(|e| ElifError::Validation { message: format!("Failed to flush CSV writer: {}", e) })?;
    
    Ok(())
}

fn export_as_mbox(_emails: &[CapturedEmail], _output_path: &str) -> Result<(), ElifError> {
    // TODO: Implement mbox format export
    Err(ElifError::Validation { message: "MBOX export not yet implemented".to_string() })
}