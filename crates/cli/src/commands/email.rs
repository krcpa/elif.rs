use elif_core::ElifError;
use std::collections::HashMap;
use serde_json::{Value, from_str, to_string_pretty};
use std::fs;
use std::path::PathBuf;
use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct EmailSendArgs {
    pub to: String,
    pub subject: String,
    pub template: Option<String>,
    pub body: Option<String>,
    pub html: bool,
    pub context: Option<String>,
}

#[derive(Debug)]
pub struct EmailTemplateRenderArgs {
    pub template: String,
    pub context: Option<String>,
    pub format: String,
}

#[derive(Debug)]
pub struct EmailProviderConfigureArgs {
    pub provider: String,
    pub interactive: bool,
}

#[derive(Debug)]
pub struct EmailQueueProcessArgs {
    pub limit: Option<u32>,
    pub timeout: u64,
}

#[derive(Debug)]
pub struct EmailQueueClearArgs {
    pub failed: bool,
    pub completed: bool,
}

#[derive(Debug)]
pub struct EmailTrackAnalyticsArgs {
    pub range: String,
    pub filter: Option<String>,
}

#[derive(Debug)]
pub struct EmailSetupArgs {
    pub provider: Option<String>,
    pub non_interactive: bool,
}

#[derive(Debug)]
pub struct EmailCaptureArgs {
    pub enable: bool,
    pub disable: bool,
    pub dir: Option<String>,
}

#[derive(Debug)]
pub struct EmailTestListArgs {
    pub detailed: bool,
    pub to: Option<String>,
    pub subject: Option<String>,
    pub limit: usize,
}

#[derive(Debug)]
pub struct EmailTestShowArgs {
    pub email_id: String,
    pub raw: bool,
    pub part: Option<String>,
}

#[derive(Debug)]
pub struct EmailTestClearArgs {
    pub all: bool,
    pub older_than: Option<u32>,
}

#[derive(Debug)]
pub struct EmailTestExportArgs {
    pub format: String,
    pub output: Option<String>,
    pub include_body: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CapturedEmail {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub to: String,
    pub from: String,
    pub subject: String,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub headers: HashMap<String, String>,
    pub template: Option<String>,
    pub context: Option<HashMap<String, Value>>,
}

/// Send a test email
pub async fn send(args: EmailSendArgs) -> Result<(), ElifError> {
    println!("📧 Sending test email to: {}", args.to);
    
    // Parse context data if provided
    let context_data: HashMap<String, Value> = if let Some(context_str) = &args.context {
        from_str(context_str)
            .map_err(|e| ElifError::Validation(format!("Invalid JSON context: {}", e)))?
    } else {
        HashMap::new()
    };
    
    let (body_text, body_html) = if let Some(template) = &args.template {
        println!("📄 Using template: {}", template);
        if !context_data.is_empty() {
            println!("🎯 Context variables: {}", context_data.len());
        }
        // TODO: Load and render template
        let rendered_body = format!("Template '{}' rendered with {} context variables", template, context_data.len());
        if args.html {
            (None, Some(format!("<html><body>{}</body></html>", rendered_body)))
        } else {
            (Some(rendered_body), None)
        }
    } else if let Some(body) = &args.body {
        println!("📝 Email body length: {} characters", body.len());
        if args.html {
            println!("🌐 Sending as HTML email");
            (None, Some(body.clone()))
        } else {
            println!("📄 Sending as plain text email");
            (Some(body.clone()), None)
        }
    } else {
        return Err(ElifError::Validation("Either --template or --body must be provided".to_string()));
    };

    // Check if email capture is enabled
    if is_capture_enabled().await? {
        capture_email_to_filesystem(&args, body_text, body_html, &context_data).await?;
        println!("📁 Email captured to filesystem for testing");
    } else {
        println!("⏳ Email sending not yet implemented - would send email");
    }
    
    println!("✅ Test email processed successfully!");
    Ok(())
}

/// List all available email templates
pub async fn template_list() -> Result<(), ElifError> {
    println!("📄 Available Email Templates:");
    println!("⏳ Template discovery not yet implemented");
    // TODO: Scan templates directory and list available templates
    Ok(())
}

/// Validate email template syntax
pub async fn template_validate(template: &str) -> Result<(), ElifError> {
    println!("🔍 Validating template: {}", template);
    println!("⏳ Template validation not yet implemented");
    // TODO: Load template and validate Tera syntax
    Ok(())
}

/// Render email template with context data
pub async fn template_render(args: EmailTemplateRenderArgs) -> Result<(), ElifError> {
    println!("🎨 Rendering template: {}", args.template);
    
    // Parse context data if provided
    let _context_data: HashMap<String, Value> = if let Some(context_str) = &args.context {
        from_str(context_str)
            .map_err(|e| ElifError::Validation(format!("Invalid JSON context: {}", e)))?
    } else {
        HashMap::new()
    };
    
    println!("📋 Output format: {}", args.format);
    println!("⏳ Template rendering not yet implemented");
    // TODO: Load template, render with context, and display output
    Ok(())
}

/// Test email provider connection
pub async fn provider_test(provider: Option<String>) -> Result<(), ElifError> {
    let provider_name = provider.unwrap_or_else(|| "default".to_string());
    println!("🔌 Testing email provider: {}", provider_name);
    println!("⏳ Provider testing not yet implemented");
    // TODO: Load provider config and test connection
    Ok(())
}

/// Configure email provider
pub async fn provider_configure(args: EmailProviderConfigureArgs) -> Result<(), ElifError> {
    println!("⚙️  Configuring email provider: {}", args.provider);
    if args.interactive {
        println!("🎯 Interactive configuration mode");
        // TODO: Launch interactive configuration wizard
    } else {
        println!("📋 Non-interactive configuration");
        // TODO: Use environment variables or config files
    }
    println!("⏳ Provider configuration not yet implemented");
    Ok(())
}

/// Switch active email provider
pub async fn provider_switch(provider: &str) -> Result<(), ElifError> {
    println!("🔄 Switching to email provider: {}", provider);
    println!("⏳ Provider switching not yet implemented");
    // TODO: Update active provider in config
    Ok(())
}

/// Show email queue status
pub async fn queue_status(detailed: bool) -> Result<(), ElifError> {
    println!("📊 Email Queue Status:");
    if detailed {
        println!("📋 Detailed information enabled");
    }
    println!("⏳ Queue status not yet implemented");
    // TODO: Connect to queue backend and show status
    Ok(())
}

/// Process queued emails
pub async fn queue_process(args: EmailQueueProcessArgs) -> Result<(), ElifError> {
    println!("⚡ Processing email queue");
    if let Some(limit) = args.limit {
        println!("📏 Processing up to {} emails", limit);
    }
    println!("⏰ Timeout: {} seconds per email", args.timeout);
    println!("⏳ Queue processing not yet implemented");
    // TODO: Connect to queue backend and process emails
    Ok(())
}

/// Clear email queue
pub async fn queue_clear(args: EmailQueueClearArgs) -> Result<(), ElifError> {
    println!("🧹 Clearing email queue");
    if args.failed {
        println!("❌ Clearing failed jobs");
    }
    if args.completed {
        println!("✅ Clearing completed jobs");
    }
    if !args.failed && !args.completed {
        return Err(ElifError::Validation("Must specify either --failed or --completed".to_string()));
    }
    println!("⏳ Queue clearing not yet implemented");
    // TODO: Connect to queue backend and clear specified jobs
    Ok(())
}

/// Show email tracking analytics
pub async fn track_analytics(args: EmailTrackAnalyticsArgs) -> Result<(), ElifError> {
    println!("📊 Email Analytics - Range: {}", args.range);
    if let Some(filter) = &args.filter {
        println!("🎯 Filter: {}", filter);
    }
    println!("⏳ Analytics not yet implemented");
    // TODO: Connect to analytics backend and show data
    Ok(())
}

/// Show email delivery statistics
pub async fn track_stats(group_by: &str) -> Result<(), ElifError> {
    println!("📈 Email Statistics - Grouped by: {}", group_by);
    println!("⏳ Statistics not yet implemented");
    // TODO: Connect to analytics backend and show stats
    Ok(())
}

/// Setup email system configuration
pub async fn setup(args: EmailSetupArgs) -> Result<(), ElifError> {
    println!("🔧 Email System Setup");
    if let Some(provider) = &args.provider {
        println!("📮 Provider: {}", provider);
    }
    if args.non_interactive {
        println!("🤖 Non-interactive mode");
    } else {
        println!("🎯 Interactive configuration wizard");
    }
    println!("⏳ Email setup not yet implemented");
    // TODO: Launch configuration wizard or use defaults
    Ok(())
}

// Email testing and filesystem capture implementation

/// Configure email capture to filesystem
pub async fn test_capture(args: EmailCaptureArgs) -> Result<(), ElifError> {
    if args.enable && args.disable {
        return Err(ElifError::Validation("Cannot enable and disable capture at the same time".to_string()));
    }
    
    let capture_dir = get_capture_directory(args.dir)?;
    
    if args.enable {
        // Create capture directory
        fs::create_dir_all(&capture_dir)
            .map_err(|e| ElifError::Validation(format!("Failed to create capture directory: {}", e)))?;
        
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
        return Err(ElifError::Validation("No captured emails found. Enable capture first.".to_string()));
    }
    
    let email = get_captured_email(&capture_dir, &args.email_id).await?;
    
    if args.raw {
        // Show raw JSON
        println!("{}", to_string_pretty(&email)
            .map_err(|e| ElifError::Validation(format!("Failed to serialize email: {}", e)))?);
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
                        .map_err(|e| ElifError::Validation(format!("Failed to serialize context: {}", e)))?);
                }
            }
        }
    }
    
    Ok(())
}

/// Clear captured emails
pub async fn test_clear(args: EmailTestClearArgs) -> Result<(), ElifError> {
    if !args.all && args.older_than.is_none() {
        return Err(ElifError::Validation("Must specify either --all or --older-than".to_string()));
    }
    
    let capture_dir = get_capture_directory(None)?;
    
    if !capture_dir.exists() {
        println!("📂 No captured emails to clear");
        return Ok(());
    }
    
    let entries = fs::read_dir(&capture_dir)
        .map_err(|e| ElifError::Validation(format!("Failed to read capture directory: {}", e)))?;
    
    let mut cleared_count = 0;
    
    for entry in entries {
        let entry = entry.map_err(|e| ElifError::Validation(format!("Failed to read directory entry: {}", e)))?;
        let path = entry.path();
        
        if !path.extension().map_or(false, |ext| ext == "json") {
            continue;
        }
        
        if args.all {
            fs::remove_file(&path)
                .map_err(|e| ElifError::Validation(format!("Failed to remove file: {}", e)))?;
            cleared_count += 1;
        } else if let Some(days) = args.older_than {
            // Check file age
            let metadata = fs::metadata(&path)
                .map_err(|e| ElifError::Validation(format!("Failed to get file metadata: {}", e)))?;
            
            if let Ok(created) = metadata.created() {
                let age = created.elapsed().map_err(|_| ElifError::Validation("Failed to calculate file age".to_string()))?;
                if age.as_secs() > (days as u64 * 24 * 60 * 60) {
                    fs::remove_file(&path)
                        .map_err(|e| ElifError::Validation(format!("Failed to remove file: {}", e)))?;
                    cleared_count += 1;
                }
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
        return Err(ElifError::Validation("No captured emails found".to_string()));
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
        _ => return Err(ElifError::Validation(format!("Unsupported export format: {}", args.format))),
    }
    
    println!("📤 Exported {} emails to: {}", emails.len(), output_path);
    Ok(())
}

// Helper functions

fn get_capture_directory(custom_dir: Option<String>) -> Result<PathBuf, ElifError> {
    if let Some(dir) = custom_dir {
        Ok(PathBuf::from(dir))
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(".elif").join("email_capture"))
            .map_err(|e| ElifError::Validation(format!("Failed to get current directory: {}", e)))
    }
}

async fn is_capture_enabled() -> Result<bool, ElifError> {
    let config_path = get_capture_directory(None)?.join(".config");
    Ok(config_path.exists())
}

async fn set_capture_enabled(enabled: bool, capture_dir: &PathBuf) -> Result<(), ElifError> {
    let config_path = capture_dir.join(".config");
    
    if enabled {
        fs::write(&config_path, "enabled")
            .map_err(|e| ElifError::Validation(format!("Failed to write config: {}", e)))?;
    } else {
        if config_path.exists() {
            fs::remove_file(&config_path)
                .map_err(|e| ElifError::Validation(format!("Failed to remove config: {}", e)))?;
        }
    }
    
    Ok(())
}

async fn capture_email_to_filesystem(
    args: &EmailSendArgs,
    body_text: Option<String>,
    body_html: Option<String>,
    context_data: &HashMap<String, Value>
) -> Result<(), ElifError> {
    let capture_dir = get_capture_directory(None)?;
    
    let email_id = format!("email_{}", Utc::now().timestamp_micros());
    let mut headers = HashMap::new();
    headers.insert("To".to_string(), args.to.clone());
    headers.insert("Subject".to_string(), args.subject.clone());
    headers.insert("From".to_string(), "test@elif.rs".to_string());
    
    let captured_email = CapturedEmail {
        id: email_id.clone(),
        timestamp: Utc::now(),
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
        .map_err(|e| ElifError::Validation(format!("Failed to serialize email: {}", e)))?;
    
    fs::write(&file_path, json_content)
        .map_err(|e| ElifError::Validation(format!("Failed to write email file: {}", e)))?;
    
    Ok(())
}

async fn list_captured_emails(
    capture_dir: &PathBuf,
    to_filter: Option<&str>,
    subject_filter: Option<&str>,
    limit: Option<usize>,
    _max_results: usize
) -> Result<Vec<CapturedEmail>, ElifError> {
    let mut emails = Vec::new();
    
    let entries = fs::read_dir(capture_dir)
        .map_err(|e| ElifError::Validation(format!("Failed to read capture directory: {}", e)))?;
    
    for entry in entries {
        let entry = entry.map_err(|e| ElifError::Validation(format!("Failed to read directory entry: {}", e)))?;
        let path = entry.path();
        
        if !path.extension().map_or(false, |ext| ext == "json") {
            continue;
        }
        
        let content = fs::read_to_string(&path)
            .map_err(|e| ElifError::Validation(format!("Failed to read email file: {}", e)))?;
        
        let email: CapturedEmail = from_str(&content)
            .map_err(|e| ElifError::Validation(format!("Failed to parse email JSON: {}", e)))?;
        
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
    }
    
    // Sort by timestamp (newest first)
    emails.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    
    if let Some(limit) = limit {
        emails.truncate(limit);
    }
    
    Ok(emails)
}

async fn get_captured_email(capture_dir: &PathBuf, email_id: &str) -> Result<CapturedEmail, ElifError> {
    // Try direct ID match first
    let file_path = capture_dir.join(format!("{}.json", email_id));
    
    if file_path.exists() {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| ElifError::Validation(format!("Failed to read email file: {}", e)))?;
        
        return from_str(&content)
            .map_err(|e| ElifError::Validation(format!("Failed to parse email JSON: {}", e)));
    }
    
    // Try index-based lookup (1-based)
    if let Ok(index) = email_id.parse::<usize>() {
        let emails = list_captured_emails(capture_dir, None, None, None, 999999).await?;
        if index > 0 && index <= emails.len() {
            return Ok(emails[index - 1].clone());
        }
    }
    
    Err(ElifError::Validation(format!("Email not found: {}", email_id)))
}

fn export_as_json(emails: &[CapturedEmail], output_path: &str, include_body: bool) -> Result<(), ElifError> {
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
    
    let json_content = to_string_pretty(&export_data)
        .map_err(|e| ElifError::Validation(format!("Failed to serialize export data: {}", e)))?;
    
    fs::write(output_path, json_content)
        .map_err(|e| ElifError::Validation(format!("Failed to write export file: {}", e)))?;
    
    Ok(())
}

fn export_as_csv(emails: &[CapturedEmail], output_path: &str, include_body: bool) -> Result<(), ElifError> {
    let mut csv_content = if include_body {
        "timestamp,from,to,subject,template,body_text,body_html\n".to_string()
    } else {
        "timestamp,from,to,subject,template\n".to_string()
    };
    
    for email in emails {
        let template = email.template.as_deref().unwrap_or("");
        let row = if include_body {
            format!("{},{},{},{},{},{},{}\n",
                email.timestamp.format("%Y-%m-%d %H:%M:%S"),
                csv_escape(&email.from),
                csv_escape(&email.to),
                csv_escape(&email.subject),
                csv_escape(template),
                csv_escape(&email.body_text.as_deref().unwrap_or("")),
                csv_escape(&email.body_html.as_deref().unwrap_or(""))
            )
        } else {
            format!("{},{},{},{},{}\n",
                email.timestamp.format("%Y-%m-%d %H:%M:%S"),
                csv_escape(&email.from),
                csv_escape(&email.to),
                csv_escape(&email.subject),
                csv_escape(template)
            )
        };
        csv_content.push_str(&row);
    }
    
    fs::write(output_path, csv_content)
        .map_err(|e| ElifError::Validation(format!("Failed to write CSV file: {}", e)))?;
    
    Ok(())
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn export_as_mbox(_emails: &[CapturedEmail], _output_path: &str) -> Result<(), ElifError> {
    // TODO: Implement mbox format export
    Err(ElifError::Validation("MBOX export not yet implemented".to_string()))
}