use elif_core::ElifError;
use std::collections::HashMap;
use serde_json::{Value, from_str};

use super::types::{EmailSendArgs, EmailSetupArgs};

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
    if crate::commands::email::testing::is_capture_enabled().await? {
        crate::commands::email::testing::capture_email_to_filesystem(&args, body_text, body_html, &context_data).await?;
        println!("📁 Email captured to filesystem for testing");
    } else {
        println!("⏳ Email sending not yet implemented - would send email");
    }
    
    println!("✅ Test email processed successfully!");
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