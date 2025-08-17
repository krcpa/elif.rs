use crate::{
    config::TemplateConfig,
    error::EmailError,
    templates::{EmailTemplate, RenderedEmail, TemplateContext, TemplateDebugInfo},
};
use std::{collections::HashMap, sync::RwLock};
use tera::{Tera, Context, Value, to_value, Result as TeraResult};

/// Email template engine using Tera
pub struct TemplateEngine {
    tera: RwLock<Tera>,
    config: TemplateConfig,
    templates: RwLock<HashMap<String, EmailTemplate>>,
}

impl TemplateEngine {
    /// Create new template engine
    pub fn new(config: TemplateConfig) -> Result<Self, EmailError> {
        let mut tera = Tera::new(&format!("{}/**/*", config.templates_dir))?;
        
        // Register custom filters
        register_email_filters(&mut tera);
        
        // Enable auto-escaping for HTML but not for text
        tera.autoescape_on(vec![".html", ".htm"]);
        
        Ok(Self { 
            tera: RwLock::new(tera), 
            config,
            templates: RwLock::new(HashMap::new()),
        })
    }

    /// Render template with context
    pub fn render_template(
        &self,
        template: &EmailTemplate,
        context: &TemplateContext,
    ) -> Result<RenderedEmail, EmailError> {
        // Create tera context from template context
        let mut tera_context = Context::new();
        for (key, value) in context {
            tera_context.insert(key, value);
        }

        // Add template metadata to context
        for (key, value) in &template.metadata {
            tera_context.insert(key, value);
        }

        let mut tera = self.tera.write().map_err(|_| EmailError::template("Failed to acquire write lock"))?;

        // Render subject
        let subject = if let Some(subject_template) = &template.subject_template {
            tera.render_str(subject_template, &tera_context)?
        } else {
            // Fallback to context value or default
            tera_context.get("subject")
                .and_then(|v| v.as_str())
                .unwrap_or("No Subject")
                .to_string()
        };

        // Render HTML content
        let html_content = if let Some(html_template) = &template.html_template {
            Some(tera.render_str(html_template, &tera_context)?)
        } else {
            None
        };

        // Render text content
        let text_content = if let Some(text_template) = &template.text_template {
            Some(tera.render_str(text_template, &tera_context)?)
        } else {
            None
        };

        Ok(RenderedEmail {
            html_content,
            text_content,
            subject,
        })
    }

    /// Get template info for debugging
    pub fn get_template_info(&self, template_name: &str) -> Result<TemplateDebugInfo, EmailError> {
        let template = self.get_template(template_name)?;

        Ok(TemplateDebugInfo {
            name: template.name.clone(),
            has_html: template.html_template.is_some(),
            has_text: template.text_template.is_some(),
            has_subject: template.subject_template.is_some(),
            layout: template.layout.clone(),
            metadata: template.metadata.clone(),
            html_content: template.html_template.clone(),
            text_content: template.text_template.clone(),
            subject_content: template.subject_template.clone(),
        })
    }

    /// Register a template
    pub fn register_template(&self, template: EmailTemplate) -> Result<(), EmailError> {
        let mut templates = self.templates.write().map_err(|_| EmailError::template("Failed to acquire write lock"))?;
        templates.insert(template.name.clone(), template);
        Ok(())
    }

    /// Get template by name
    pub fn get_template(&self, name: &str) -> Result<EmailTemplate, EmailError> {
        let templates = self.templates.read().map_err(|_| EmailError::template("Failed to acquire read lock"))?;
        templates.get(name)
            .cloned()
            .ok_or_else(|| EmailError::template(format!("Template '{}' not found", name)))
    }

    /// Render template by name with context
    pub fn render_template_by_name(
        &self,
        template_name: &str,
        context: &TemplateContext,
    ) -> Result<RenderedEmail, EmailError> {
        let template = self.get_template(template_name)?;
        self.render_template(&template, context)
    }

    /// Validate template syntax
    pub fn validate_template(&self, template_str: &str, template_name: Option<&str>) -> Result<(), EmailError> {
        // Create a temporary Tera instance to test template parsing without affecting the main engine
        let mut temp_tera = Tera::default();
        let temp_name = "__validation_template__";
        
        match temp_tera.add_raw_template(temp_name, template_str) {
            Ok(_) => Ok(()),
            Err(e) => {
                let context_info = if let Some(name) = template_name {
                    format!("template '{}': {}", name, e)
                } else {
                    format!("template: {}", e)
                };
                Err(EmailError::template(format!("Invalid {}", context_info)))
            }
        }
    }

    /// List available templates
    pub fn list_templates(&self) -> Result<Vec<String>, EmailError> {
        let mut tera = self.tera.write().map_err(|_| EmailError::template("Failed to acquire write lock"))?;
        Ok(tera.get_template_names().map(|s| s.to_string()).collect())
    }
}

/// Register custom email filters for Tera
fn register_email_filters(tera: &mut Tera) {
    // Date formatting filters
    tera.register_filter("format_date", format_date_filter);
    tera.register_filter("format_datetime", format_datetime_filter);
    tera.register_filter("now", now_filter);
    
    // Email tracking filters
    tera.register_filter("tracking_pixel", tracking_pixel_filter);
    tera.register_filter("tracking_link", tracking_link_filter);
    
    // Formatting filters
    tera.register_filter("currency", format_currency_filter);
    tera.register_filter("phone", format_phone_filter);
    tera.register_filter("address", format_address_filter);
    
    // String filters
    tera.register_filter("url_encode", url_encode_filter);
}

// Tera filter implementations
fn format_date_filter(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
    use chrono::{DateTime, Utc, NaiveDateTime, TimeZone};
    
    let format_str = args.get("format").and_then(|v| v.as_str()).unwrap_or("%Y-%m-%d");
    
    let formatted = match value {
        Value::String(s) => {
            if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                dt.format(format_str).to_string()
            } else if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d") {
                let dt = Utc.from_utc_datetime(&dt);
                dt.format(format_str).to_string()
            } else {
                s.clone()
            }
        }
        Value::Number(n) => {
            if let Some(timestamp) = n.as_i64() {
                let dt = Utc.timestamp_opt(timestamp, 0).single().unwrap_or_else(|| Utc::now());
                dt.format(format_str).to_string()
            } else {
                "Invalid timestamp".to_string()
            }
        }
        _ => "Invalid date".to_string(),
    };
    
    Ok(to_value(formatted)?)
}

fn format_datetime_filter(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
    use chrono::{DateTime, Utc, TimeZone};
    
    let format_str = args.get("format").and_then(|v| v.as_str()).unwrap_or("%Y-%m-%d %H:%M:%S");
    
    let formatted = match value {
        Value::String(s) => {
            if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                dt.format(format_str).to_string()
            } else {
                s.clone()
            }
        }
        Value::Number(n) => {
            if let Some(timestamp) = n.as_i64() {
                let dt = Utc.timestamp_opt(timestamp, 0).single().unwrap_or_else(|| Utc::now());
                dt.format(format_str).to_string()
            } else {
                "Invalid timestamp".to_string()
            }
        }
        _ => "Invalid datetime".to_string(),
    };
    
    Ok(to_value(formatted)?)
}

fn now_filter(_value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
    use chrono::Utc;
    let format_str = args.get("format").and_then(|v| v.as_str()).unwrap_or("%Y-%m-%d %H:%M:%S");
    let now = Utc::now();
    let formatted = now.format(format_str).to_string();
    Ok(to_value(formatted)?)
}

fn tracking_pixel_filter(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
    use chrono::Utc;
    use uuid::Uuid;
    
    let email_id = value.as_str().ok_or_else(|| tera::Error::msg("tracking_pixel: email_id must be string"))?;
    let base_url = args.get("base_url").and_then(|v| v.as_str()).unwrap_or("https://tracking.example.com");
    
    // Validate UUID format
    Uuid::parse_str(email_id).map_err(|_| tera::Error::msg("tracking_pixel: invalid UUID format"))?;
    
    let timestamp = Utc::now().timestamp();
    let pixel_url = format!("{}/email/track/open?id={}&t={}", base_url, email_id, timestamp);
    let pixel_html = format!(
        r#"<img src="{}" alt="" width="1" height="1" style="display: block; width: 1px; height: 1px;" />"#,
        pixel_url
    );
    
    Ok(to_value(pixel_html)?)
}

fn tracking_link_filter(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
    use uuid::Uuid;
    
    let email_id = value.as_str().ok_or_else(|| tera::Error::msg("tracking_link: email_id must be string"))?;
    let target_url = args.get("url").and_then(|v| v.as_str()).ok_or_else(|| tera::Error::msg("tracking_link: missing url parameter"))?;
    let base_url = args.get("base_url").and_then(|v| v.as_str()).unwrap_or("https://tracking.example.com");
    
    // Validate UUID format
    Uuid::parse_str(email_id).map_err(|_| tera::Error::msg("tracking_link: invalid UUID format"))?;
    
    let encoded_url = urlencoding::encode(target_url);
    let tracking_url = format!("{}/email/track/click?id={}&url={}", base_url, email_id, encoded_url);
    
    Ok(to_value(tracking_url)?)
}

fn format_currency_filter(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
    let amount = value.as_f64().ok_or_else(|| tera::Error::msg("currency: amount must be number"))?;
    let currency = args.get("currency").and_then(|v| v.as_str()).unwrap_or("USD");
    
    let symbol = match currency {
        "USD" => "$",
        "EUR" => "€", 
        "GBP" => "£",
        "JPY" => "¥",
        _ => currency,
    };
    
    let formatted = if currency == "JPY" {
        format!("{}{:.0}", symbol, amount)
    } else {
        format!("{}{:.2}", symbol, amount)
    };
    
    Ok(to_value(formatted)?)
}

fn format_phone_filter(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
    let phone_str = value.as_str().ok_or_else(|| tera::Error::msg("phone: must be string"))?;
    let country = args.get("country").and_then(|v| v.as_str()).unwrap_or("US");
    
    let digits: String = phone_str.chars().filter(|c| c.is_ascii_digit()).collect();
    
    let formatted = match country {
        "US" if digits.len() == 10 => {
            format!("({}) {}-{}", &digits[0..3], &digits[3..6], &digits[6..10])
        }
        "US" if digits.len() == 11 && digits.starts_with('1') => {
            format!("+1 ({}) {}-{}", &digits[1..4], &digits[4..7], &digits[7..11])
        }
        _ => phone_str.to_string(),
    };
    
    Ok(to_value(formatted)?)
}

fn format_address_filter(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let formatted = match value {
        Value::Object(obj) => {
            let street = obj.get("street").and_then(|v| v.as_str()).unwrap_or("");
            let city = obj.get("city").and_then(|v| v.as_str()).unwrap_or("");
            let state = obj.get("state").and_then(|v| v.as_str()).unwrap_or("");
            let zip = obj.get("zip").and_then(|v| v.as_str()).unwrap_or("");
            
            let mut parts = Vec::new();
            if !street.is_empty() { parts.push(street.to_string()); }
            
            let mut city_line = Vec::new();
            if !city.is_empty() { city_line.push(city.to_string()); }
            if !state.is_empty() && !zip.is_empty() {
                city_line.push(format!("{} {}", state, zip));
            } else if !state.is_empty() {
                city_line.push(state.to_string());
            } else if !zip.is_empty() {
                city_line.push(zip.to_string());
            }
            
            if !city_line.is_empty() {
                parts.push(city_line.join(" "));
            }
            
            parts.join("<br/>")
        }
        Value::String(s) => s.replace('\n', "<br/>"),
        _ => "Invalid address format".to_string(),
    };
    
    Ok(to_value(formatted)?)
}

fn url_encode_filter(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let text = value.as_str().ok_or_else(|| tera::Error::msg("url_encode: must be string"))?;
    let encoded = urlencoding::encode(text);
    Ok(to_value(encoded.to_string())?)
}

impl From<tera::Error> for EmailError {
    fn from(err: tera::Error) -> Self {
        EmailError::template(format!("Template error: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    fn create_test_config(temp_dir: &TempDir) -> TemplateConfig {
        TemplateConfig {
            templates_dir: temp_dir.path().join("templates").to_string_lossy().to_string(),
            layouts_dir: temp_dir.path().join("layouts").to_string_lossy().to_string(),
            partials_dir: temp_dir.path().join("partials").to_string_lossy().to_string(),
            enable_cache: false,
            template_extension: ".html".to_string(),
            cache_size: None,
            watch_files: false,
        }
    }

    #[test]
    fn test_template_engine_creation() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("templates")).unwrap();
        
        let config = create_test_config(&temp_dir);
        let engine = TemplateEngine::new(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_basic_template_rendering() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("templates")).unwrap();
        
        let config = create_test_config(&temp_dir);
        let engine = TemplateEngine::new(config).unwrap();

        let template = EmailTemplate {
            name: "test".to_string(),
            html_template: Some("<h1>Hello {{ name }}</h1>".to_string()),
            text_template: Some("Hello {{ name }}".to_string()),
            subject_template: Some("Welcome {{ name }}".to_string()),
            layout: None,
            metadata: HashMap::new(),
        };

        let mut context = TemplateContext::new();
        context.insert("name".to_string(), serde_json::Value::String("World".to_string()));

        let rendered = engine.render_template(&template, &context).unwrap();

        assert_eq!(rendered.subject, "Welcome World");
        assert_eq!(rendered.html_content, Some("<h1>Hello World</h1>".to_string()));
        assert_eq!(rendered.text_content, Some("Hello World".to_string()));
    }

    #[test]
    fn test_tera_filters_compatibility() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("templates")).unwrap();
        
        let config = create_test_config(&temp_dir);
        let engine = TemplateEngine::new(config).unwrap();

        // Test currency filter
        let template = EmailTemplate {
            name: "currency_test".to_string(),
            html_template: Some("Price: {{ amount | currency(currency=\"USD\") }}".to_string()),
            text_template: None,
            subject_template: None,
            layout: None,
            metadata: HashMap::new(),
        };

        let mut context = TemplateContext::new();
        context.insert("amount".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(99.99).unwrap()));

        let rendered = engine.render_template(&template, &context).unwrap();
        assert_eq!(rendered.html_content, Some("Price: $99.99".to_string()));
    }

    #[test]
    fn test_phone_filter() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("templates")).unwrap();
        
        let config = create_test_config(&temp_dir);
        let engine = TemplateEngine::new(config).unwrap();

        let template = EmailTemplate {
            name: "phone_test".to_string(),
            html_template: Some("Phone: {{ phone_number | phone(country=\"US\") }}".to_string()),
            text_template: None,
            subject_template: None,
            layout: None,
            metadata: HashMap::new(),
        };

        let mut context = TemplateContext::new();
        context.insert("phone_number".to_string(), serde_json::Value::String("5551234567".to_string()));

        let rendered = engine.render_template(&template, &context).unwrap();
        assert_eq!(rendered.html_content, Some("Phone: (555) 123-4567".to_string()));
    }

    #[test]
    fn test_address_filter() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("templates")).unwrap();
        
        let config = create_test_config(&temp_dir);
        let engine = TemplateEngine::new(config).unwrap();

        let template = EmailTemplate {
            name: "address_test".to_string(),
            html_template: Some("Address: {{ address | address }}".to_string()),
            text_template: None,
            subject_template: None,
            layout: None,
            metadata: HashMap::new(),
        };

        let mut context = TemplateContext::new();
        let address_obj = serde_json::json!({
            "street": "123 Main St",
            "city": "Anytown",
            "state": "CA",
            "zip": "90210"
        });
        context.insert("address".to_string(), address_obj);

        let rendered = engine.render_template(&template, &context).unwrap();
        assert_eq!(rendered.html_content, Some("Address: 123 Main St<br/>Anytown CA 90210".to_string()));
    }

    #[test]
    fn test_url_encode_filter() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("templates")).unwrap();
        
        let config = create_test_config(&temp_dir);
        let engine = TemplateEngine::new(config).unwrap();

        let template = EmailTemplate {
            name: "url_test".to_string(),
            html_template: Some("URL: {{ url | url_encode }}".to_string()),
            text_template: None,
            subject_template: None,
            layout: None,
            metadata: HashMap::new(),
        };

        let mut context = TemplateContext::new();
        context.insert("url".to_string(), serde_json::Value::String("hello world & more".to_string()));

        let rendered = engine.render_template(&template, &context).unwrap();
        assert_eq!(rendered.html_content, Some("URL: hello%20world%20%26%20more".to_string()));
    }

    #[test]
    fn test_tracking_pixel_filter() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("templates")).unwrap();
        
        let config = create_test_config(&temp_dir);
        let engine = TemplateEngine::new(config).unwrap();

        let template = EmailTemplate {
            name: "tracking_test".to_string(),
            html_template: Some("{{ email_id | tracking_pixel(base_url=\"https://test.com\") }}".to_string()),
            text_template: None,
            subject_template: None,
            layout: None,
            metadata: HashMap::new(),
        };

        let mut context = TemplateContext::new();
        context.insert("email_id".to_string(), serde_json::Value::String("550e8400-e29b-41d4-a716-446655440000".to_string()));

        let rendered = engine.render_template(&template, &context).unwrap();
        let html_content = rendered.html_content.as_ref().unwrap();
        assert!(html_content.contains("https://test.com/email/track/open"));
        assert!(html_content.contains("550e8400-e29b-41d4-a716-446655440000"));
    }

    #[test]
    fn test_tracking_link_filter() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("templates")).unwrap();
        
        let config = create_test_config(&temp_dir);
        let engine = TemplateEngine::new(config).unwrap();

        let template = EmailTemplate {
            name: "link_test".to_string(),
            html_template: Some("{{ email_id | tracking_link(url=\"https://example.com\", base_url=\"https://track.com\") }}".to_string()),
            text_template: None,
            subject_template: None,
            layout: None,
            metadata: HashMap::new(),
        };

        let mut context = TemplateContext::new();
        context.insert("email_id".to_string(), serde_json::Value::String("550e8400-e29b-41d4-a716-446655440000".to_string()));

        let rendered = engine.render_template(&template, &context).unwrap();
        let expected = "https://track.com/email/track/click?id=550e8400-e29b-41d4-a716-446655440000&url=https%3A%2F%2Fexample.com";
        assert_eq!(rendered.html_content, Some(expected.to_string()));
    }

    #[test]
    fn test_format_date_filter() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("templates")).unwrap();
        
        let config = create_test_config(&temp_dir);
        let engine = TemplateEngine::new(config).unwrap();

        let template = EmailTemplate {
            name: "date_test".to_string(),
            html_template: Some("Date: {{ date_value | format_date(format=\"%B %d, %Y\") }}".to_string()),
            text_template: None,
            subject_template: None,
            layout: None,
            metadata: HashMap::new(),
        };

        let mut context = TemplateContext::new();
        context.insert("date_value".to_string(), serde_json::Value::String("2023-12-25T10:00:00Z".to_string()));

        let rendered = engine.render_template(&template, &context).unwrap();
        assert_eq!(rendered.html_content, Some("Date: December 25, 2023".to_string()));
    }

    #[test]
    fn test_now_filter() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("templates")).unwrap();
        
        let config = create_test_config(&temp_dir);
        let engine = TemplateEngine::new(config).unwrap();

        let template = EmailTemplate {
            name: "now_test".to_string(),
            html_template: Some("Current year: {{ \"\" | now(format=\"%Y\") }}".to_string()),
            text_template: None,
            subject_template: None,
            layout: None,
            metadata: HashMap::new(),
        };

        let context = TemplateContext::new();
        let rendered = engine.render_template(&template, &context).unwrap();
        
        // Should contain the current year
        let current_year = chrono::Utc::now().format("%Y").to_string();
        assert_eq!(rendered.html_content, Some(format!("Current year: {}", current_year)));
    }

    #[test]
    fn test_template_registration_and_retrieval() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("templates")).unwrap();
        
        let config = create_test_config(&temp_dir);
        let engine = TemplateEngine::new(config).unwrap();

        let template = EmailTemplate {
            name: "registered_template".to_string(),
            html_template: Some("<p>Hello {{ user }}</p>".to_string()),
            text_template: Some("Hello {{ user }}".to_string()),
            subject_template: Some("Welcome {{ user }}".to_string()),
            layout: None,
            metadata: HashMap::new(),
        };

        // Register template
        engine.register_template(template.clone()).unwrap();

        // Retrieve and verify
        let retrieved = engine.get_template("registered_template").unwrap();
        assert_eq!(retrieved.name, "registered_template");
        assert_eq!(retrieved.html_template, template.html_template);

        // Render by name
        let mut context = TemplateContext::new();
        context.insert("user".to_string(), serde_json::Value::String("Alice".to_string()));

        let rendered = engine.render_template_by_name("registered_template", &context).unwrap();
        assert_eq!(rendered.subject, "Welcome Alice");
        assert_eq!(rendered.html_content, Some("<p>Hello Alice</p>".to_string()));
        assert_eq!(rendered.text_content, Some("Hello Alice".to_string()));
    }

    #[test]
    fn test_template_validation() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("templates")).unwrap();
        
        let config = create_test_config(&temp_dir);
        let engine = TemplateEngine::new(config).unwrap();

        // Valid template with no variables should validate
        assert!(engine.validate_template("Hello world!", Some("valid")).is_ok());

        // Valid template with variables should validate (syntax check only)
        assert!(engine.validate_template("Hello {{ name }}!", Some("valid_with_vars")).is_ok());

        // Valid template with conditionals should validate
        assert!(engine.validate_template("{% if user %}Hello {{ user.name }}{% endif %}", Some("valid_conditional")).is_ok());

        // Invalid template syntax should fail
        let result = engine.validate_template("Hello {{ invalid_syntax", Some("invalid"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid template 'invalid'"));

        // Invalid template with bad syntax should fail  
        let result = engine.validate_template("{% if unclosed", Some("invalid2"));
        assert!(result.is_err());
    }

    #[test]
    fn test_template_metadata_in_context() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("templates")).unwrap();
        
        let config = create_test_config(&temp_dir);
        let engine = TemplateEngine::new(config).unwrap();

        let mut metadata = HashMap::new();
        metadata.insert("brand_color".to_string(), "#ff0000".to_string());

        let template = EmailTemplate {
            name: "metadata_test".to_string(),
            html_template: Some("<div style=\"color: {{ brand_color }}\">Hello {{ name }}</div>".to_string()),
            text_template: None,
            subject_template: None,
            layout: None,
            metadata,
        };

        let mut context = TemplateContext::new();
        context.insert("name".to_string(), serde_json::Value::String("User".to_string()));

        let rendered = engine.render_template(&template, &context).unwrap();
        assert_eq!(rendered.html_content, Some("<div style=\"color: #ff0000\">Hello User</div>".to_string()));
    }

    #[test]
    fn test_template_debug_info() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("templates")).unwrap();
        
        let config = create_test_config(&temp_dir);
        let engine = TemplateEngine::new(config).unwrap();

        let mut metadata = HashMap::new();
        metadata.insert("version".to_string(), "1.0".to_string());
        metadata.insert("author".to_string(), "test".to_string());

        let template = EmailTemplate {
            name: "debug_test".to_string(),
            html_template: Some("<h1>Debug Test</h1>".to_string()),
            text_template: Some("Debug Test".to_string()),
            subject_template: Some("Debug: {{ type }}".to_string()),
            layout: Some("base".to_string()),
            metadata,
        };

        // Register template
        engine.register_template(template.clone()).unwrap();

        // Get debug info
        let debug_info = engine.get_template_info("debug_test").unwrap();

        // Verify accurate debug information
        assert_eq!(debug_info.name, "debug_test");
        assert_eq!(debug_info.has_html, true);
        assert_eq!(debug_info.has_text, true);
        assert_eq!(debug_info.has_subject, true);
        assert_eq!(debug_info.layout, Some("base".to_string()));
        assert_eq!(debug_info.metadata.get("version"), Some(&"1.0".to_string()));
        assert_eq!(debug_info.metadata.get("author"), Some(&"test".to_string()));
        assert_eq!(debug_info.html_content, Some("<h1>Debug Test</h1>".to_string()));
        assert_eq!(debug_info.text_content, Some("Debug Test".to_string()));
        assert_eq!(debug_info.subject_content, Some("Debug: {{ type }}".to_string()));

        // Test with template that has no text content
        let minimal_template = EmailTemplate {
            name: "minimal".to_string(),
            html_template: Some("<p>Minimal</p>".to_string()),
            text_template: None,
            subject_template: None,
            layout: None,
            metadata: HashMap::new(),
        };

        engine.register_template(minimal_template).unwrap();
        let minimal_debug = engine.get_template_info("minimal").unwrap();

        assert_eq!(minimal_debug.has_html, true);
        assert_eq!(minimal_debug.has_text, false);
        assert_eq!(minimal_debug.has_subject, false);
        assert_eq!(minimal_debug.layout, None);
        assert!(minimal_debug.metadata.is_empty());

        // Test error case for non-existent template
        let result = engine.get_template_info("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Template 'nonexistent' not found"));
    }
}