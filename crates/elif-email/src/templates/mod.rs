pub mod engine;
pub mod registry;

pub use engine::*;
pub use registry::*;

use crate::{Email, EmailError};
use serde::Serialize;
use std::collections::HashMap;

/// Template context for rendering emails
pub type TemplateContext = HashMap<String, serde_json::Value>;

/// Email template definition
#[derive(Debug, Clone)]
pub struct EmailTemplate {
    /// Template name/identifier
    pub name: String,
    /// HTML template content or path
    pub html_template: Option<String>,
    /// Text template content or path
    pub text_template: Option<String>,
    /// Layout to use (optional)
    pub layout: Option<String>,
    /// Default subject template
    pub subject_template: Option<String>,
    /// Template metadata
    pub metadata: HashMap<String, String>,
}

/// Rendered email content
#[derive(Debug, Clone)]
pub struct RenderedEmail {
    /// Rendered HTML content
    pub html_content: Option<String>,
    /// Rendered text content
    pub text_content: Option<String>,
    /// Rendered subject
    pub subject: String,
}

/// Email template builder for fluent API
#[derive(Debug, Clone)]
pub struct EmailTemplateBuilder {
    template: EmailTemplate,
}

impl EmailTemplate {
    /// Create new email template
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            html_template: None,
            text_template: None,
            layout: None,
            subject_template: None,
            metadata: HashMap::new(),
        }
    }

    /// Create template builder
    pub fn builder(name: impl Into<String>) -> EmailTemplateBuilder {
        EmailTemplateBuilder {
            template: Self::new(name),
        }
    }

    /// Render template with context
    pub fn render(&self, engine: &TemplateEngine, context: &TemplateContext) -> Result<RenderedEmail, EmailError> {
        engine.render_template(self, context)
    }
}

impl EmailTemplateBuilder {
    /// Set HTML template
    pub fn html_template(mut self, template: impl Into<String>) -> Self {
        self.template.html_template = Some(template.into());
        self
    }

    /// Set text template
    pub fn text_template(mut self, template: impl Into<String>) -> Self {
        self.template.text_template = Some(template.into());
        self
    }

    /// Set layout
    pub fn layout(mut self, layout: impl Into<String>) -> Self {
        self.template.layout = Some(layout.into());
        self
    }

    /// Set subject template
    pub fn subject_template(mut self, template: impl Into<String>) -> Self {
        self.template.subject_template = Some(template.into());
        self
    }

    /// Add metadata
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.template.metadata.insert(key.into(), value.into());
        self
    }

    /// Build the template
    pub fn build(self) -> EmailTemplate {
        self.template
    }
}

/// Helper trait for serializable contexts
pub trait IntoTemplateContext {
    fn into_context(self) -> Result<TemplateContext, EmailError>;
}

impl<T: Serialize> IntoTemplateContext for T {
    fn into_context(self) -> Result<TemplateContext, EmailError> {
        let value = serde_json::to_value(self)?;
        match value {
            serde_json::Value::Object(map) => {
                Ok(map.into_iter().collect())
            }
            _ => {
                let mut context = TemplateContext::new();
                context.insert("data".to_string(), value);
                Ok(context)
            }
        }
    }
}

/// Helper for TemplateContext to avoid blanket implementation conflict
pub fn template_context_into_context(context: TemplateContext) -> Result<TemplateContext, EmailError> {
    Ok(context)
}

/// Template debug information for troubleshooting
#[derive(Debug, Clone)]
pub struct TemplateDebugInfo {
    /// Template name
    pub name: String,
    /// Whether template has HTML content
    pub has_html: bool,
    /// Whether template has text content
    pub has_text: bool,
    /// Whether template has subject content
    pub has_subject: bool,
    /// Layout name if any
    pub layout: Option<String>,
    /// Template metadata
    pub metadata: HashMap<String, String>,
    /// HTML template content (for debugging)
    pub html_content: Option<String>,
    /// Text template content (for debugging)
    pub text_content: Option<String>,
    /// Subject template content (for debugging)
    pub subject_content: Option<String>,
}

/// Extension methods for Email to work with templates
impl Email {
    /// Create email from template
    pub fn from_template(
        engine: &TemplateEngine,
        template_name: &str,
        context: impl IntoTemplateContext,
    ) -> Result<Self, EmailError> {
        let context = context.into_context()?;
        let template = engine.get_template(template_name)?;
        let rendered = template.render(engine, &context)?;

        let mut email = Self::new();

        if let Some(html) = rendered.html_content {
            email = email.html_body(html);
        }

        if let Some(text) = rendered.text_content {
            email = email.text_body(text);
        }

        email = email.subject(rendered.subject);

        Ok(email)
    }

    /// Update email with rendered template content
    pub fn with_template(
        mut self,
        engine: &TemplateEngine,
        template_name: &str,
        context: impl IntoTemplateContext,
    ) -> Result<Self, EmailError> {
        let context = context.into_context()?;
        let template = engine.get_template(template_name)?;
        let rendered = template.render(engine, &context)?;

        if let Some(html) = rendered.html_content {
            self.html_body = Some(html);
        }

        if let Some(text) = rendered.text_content {
            self.text_body = Some(text);
        }

        // Only update subject if it's currently empty
        if self.subject.is_empty() {
            self.subject = rendered.subject;
        }

        Ok(self)
    }
}

/// Template helper functions
pub mod helpers {
    use handlebars::{Handlebars, Helper, Context, RenderContext, Output, HelperResult, RenderError, Renderable};
    use serde_json::Value;
    use chrono::{DateTime, Utc, TimeZone, NaiveDateTime};
    use uuid::Uuid;
    use std::str::FromStr;

    /// Register built-in email template helpers
    pub fn register_email_helpers(handlebars: &mut Handlebars) {
        // Date formatting helpers
        handlebars.register_helper("format_date", Box::new(format_date_helper));
        handlebars.register_helper("format_datetime", Box::new(format_datetime_helper));
        handlebars.register_helper("now", Box::new(now_helper));
        
        // Tracking helpers
        handlebars.register_helper("tracking_pixel", Box::new(tracking_pixel_helper));
        handlebars.register_helper("tracking_link", Box::new(tracking_link_helper));
        
        // Conditional helpers
        handlebars.register_helper("if_eq", Box::new(if_eq_helper));
        handlebars.register_helper("unless", Box::new(unless_helper));
        handlebars.register_helper("if_gt", Box::new(if_gt_helper));
        handlebars.register_helper("if_lt", Box::new(if_lt_helper));
        
        // Loop helpers
        handlebars.register_helper("each_with_index", Box::new(each_with_index_helper));
        handlebars.register_helper("range", Box::new(range_helper));
        
        // Email formatting helpers
        handlebars.register_helper("format_currency", Box::new(format_currency_helper));
        handlebars.register_helper("format_phone", Box::new(format_phone_helper));
        handlebars.register_helper("format_address", Box::new(format_address_helper));
        
        // String helpers
        handlebars.register_helper("truncate", Box::new(truncate_helper));
        handlebars.register_helper("capitalize", Box::new(capitalize_helper));
        handlebars.register_helper("url_encode", Box::new(url_encode_helper));
    }

    /// Format date helper: {{format_date date_value "format_string"}}
    fn format_date_helper(
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        let date_value = h.param(0)
            .ok_or_else(|| RenderError::new("format_date: missing date parameter"))?;
        
        let format_str = h.param(1)
            .and_then(|v| v.value().as_str())
            .unwrap_or("%Y-%m-%d");

        let formatted = match date_value.value() {
            Value::String(s) => {
                // Try parsing various date formats
                if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                    dt.format(format_str).to_string()
                } else if let Ok(dt) = DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
                    dt.format(format_str).to_string()
                } else if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d") {
                    let dt = Utc.from_utc_datetime(&dt);
                    dt.format(format_str).to_string()
                } else {
                    s.clone() // Return original if parsing fails
                }
            }
            Value::Number(n) => {
                if let Some(timestamp) = n.as_i64() {
                    let dt = Utc.timestamp_opt(timestamp, 0).single()
                        .unwrap_or_else(|| Utc::now());
                    dt.format(format_str).to_string()
                } else {
                    "Invalid timestamp".to_string()
                }
            }
            _ => "Invalid date".to_string(),
        };

        out.write(&formatted)?;
        Ok(())
    }

    /// Format datetime helper with time: {{format_datetime datetime_value "format_string"}}
    fn format_datetime_helper(
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        let date_value = h.param(0)
            .ok_or_else(|| RenderError::new("format_datetime: missing datetime parameter"))?;
        
        let format_str = h.param(1)
            .and_then(|v| v.value().as_str())
            .unwrap_or("%Y-%m-%d %H:%M:%S");

        let formatted = match date_value.value() {
            Value::String(s) => {
                if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                    dt.format(format_str).to_string()
                } else {
                    s.clone()
                }
            }
            Value::Number(n) => {
                if let Some(timestamp) = n.as_i64() {
                    let dt = Utc.timestamp_opt(timestamp, 0).single()
                        .unwrap_or_else(|| Utc::now());
                    dt.format(format_str).to_string()
                } else {
                    "Invalid timestamp".to_string()
                }
            }
            _ => "Invalid datetime".to_string(),
        };

        out.write(&formatted)?;
        Ok(())
    }

    /// Current timestamp helper: {{now "format_string"}}
    fn now_helper(
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        let format_str = h.param(0)
            .and_then(|v| v.value().as_str())
            .unwrap_or("%Y-%m-%d %H:%M:%S");

        let now = Utc::now();
        let formatted = now.format(format_str).to_string();
        
        out.write(&formatted)?;
        Ok(())
    }

    /// Tracking pixel helper: {{tracking_pixel email_id base_url}}
    fn tracking_pixel_helper(
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        let email_id = h.param(0)
            .ok_or_else(|| RenderError::new("tracking_pixel: missing email_id parameter"))?;
        
        let base_url = h.param(1)
            .and_then(|v| v.value().as_str())
            .unwrap_or("https://tracking.example.com");

        let email_id_str = match email_id.value() {
            Value::String(s) => s.clone(),
            _ => return Err(RenderError::new("tracking_pixel: email_id must be a string")),
        };

        // Validate UUID format
        if Uuid::from_str(&email_id_str).is_err() {
            return Err(RenderError::new("tracking_pixel: invalid UUID format"));
        }

        let timestamp = Utc::now().timestamp();
        let pixel_url = format!(
            "{}/email/track/open?id={}&t={}",
            base_url, email_id_str, timestamp
        );
        
        let pixel_html = format!(
            r#"<img src="{}" alt="" width="1" height="1" style="display: block; width: 1px; height: 1px;" />"#,
            pixel_url
        );

        out.write(&pixel_html)?;
        Ok(())
    }

    /// Tracking link helper: {{tracking_link email_id target_url base_url}}
    fn tracking_link_helper(
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        let email_id = h.param(0)
            .ok_or_else(|| RenderError::new("tracking_link: missing email_id parameter"))?;
        
        let target_url = h.param(1)
            .ok_or_else(|| RenderError::new("tracking_link: missing target_url parameter"))?;
        
        let base_url = h.param(2)
            .and_then(|v| v.value().as_str())
            .unwrap_or("https://tracking.example.com");

        let email_id_str = match email_id.value() {
            Value::String(s) => s.clone(),
            _ => return Err(RenderError::new("tracking_link: email_id must be a string")),
        };

        let target_url_str = match target_url.value() {
            Value::String(s) => s.clone(),
            _ => return Err(RenderError::new("tracking_link: target_url must be a string")),
        };

        // Validate UUID format
        if Uuid::from_str(&email_id_str).is_err() {
            return Err(RenderError::new("tracking_link: invalid UUID format"));
        }

        let encoded_url = urlencoding::encode(&target_url_str);
        let tracking_url = format!(
            "{}/email/track/click?id={}&url={}",
            base_url, email_id_str, encoded_url
        );

        out.write(&tracking_url)?;
        Ok(())
    }

    /// If equals helper: {{if_eq value1 value2 "true_output" "false_output"}}
    fn if_eq_helper(
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        let value1 = h.param(0)
            .ok_or_else(|| RenderError::new("if_eq: missing first parameter"))?;
        let value2 = h.param(1)
            .ok_or_else(|| RenderError::new("if_eq: missing second parameter"))?;
        let true_output = h.param(2)
            .and_then(|v| v.value().as_str())
            .unwrap_or("true");
        let false_output = h.param(3)
            .and_then(|v| v.value().as_str())
            .unwrap_or("false");

        if value1.value() == value2.value() {
            out.write(true_output)?;
        } else {
            out.write(false_output)?;
        }

        Ok(())
    }

    /// Unless helper: {{unless condition "output_if_false" "output_if_true"}}
    fn unless_helper(
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        let condition = h.param(0)
            .ok_or_else(|| RenderError::new("unless: missing condition parameter"))?;
        let false_output = h.param(1)
            .and_then(|v| v.value().as_str())
            .unwrap_or("");
        let true_output = h.param(2)
            .and_then(|v| v.value().as_str())
            .unwrap_or("");

        let is_truthy = match condition.value() {
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Object(o) => !o.is_empty(),
            Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
        };

        if !is_truthy {
            out.write(false_output)?;
        } else {
            out.write(true_output)?;
        }

        Ok(())
    }

    /// If greater than helper: {{if_gt value1 value2 "true_output" "false_output"}}
    fn if_gt_helper(
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        let value1 = h.param(0)
            .ok_or_else(|| RenderError::new("if_gt: missing first parameter"))?;
        let value2 = h.param(1)
            .ok_or_else(|| RenderError::new("if_gt: missing second parameter"))?;
        let true_output = h.param(2)
            .and_then(|v| v.value().as_str())
            .unwrap_or("true");
        let false_output = h.param(3)
            .and_then(|v| v.value().as_str())
            .unwrap_or("false");

        let num1 = value1.value().as_f64().unwrap_or(0.0);
        let num2 = value2.value().as_f64().unwrap_or(0.0);

        if num1 > num2 {
            out.write(true_output)?;
        } else {
            out.write(false_output)?;
        }

        Ok(())
    }

    /// If less than helper: {{if_lt value1 value2 "true_output" "false_output"}}
    fn if_lt_helper(
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        let value1 = h.param(0)
            .ok_or_else(|| RenderError::new("if_lt: missing first parameter"))?;
        let value2 = h.param(1)
            .ok_or_else(|| RenderError::new("if_lt: missing second parameter"))?;
        let true_output = h.param(2)
            .and_then(|v| v.value().as_str())
            .unwrap_or("true");
        let false_output = h.param(3)
            .and_then(|v| v.value().as_str())
            .unwrap_or("false");

        let num1 = value1.value().as_f64().unwrap_or(0.0);
        let num2 = value2.value().as_f64().unwrap_or(0.0);

        if num1 < num2 {
            out.write(true_output)?;
        } else {
            out.write(false_output)?;
        }

        Ok(())
    }

    /// Each with index helper: {{each_with_index items}}
    /// Note: Simplified to avoid complex block helper issues - use standard {{#each}} instead
    fn each_with_index_helper(
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        let items = h.param(0)
            .ok_or_else(|| RenderError::new("each_with_index: missing items parameter"))?;

        let result = match items.value() {
            Value::Array(arr) => {
                let formatted: Vec<String> = arr.iter().enumerate()
                    .map(|(i, item)| format!("{}: {}", i, item))
                    .collect();
                formatted.join(", ")
            }
            _ => "Invalid array".to_string(),
        };

        out.write(&result)?;
        Ok(())
    }

    /// Range helper: {{range start end}}
    fn range_helper(
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        let start = h.param(0)
            .ok_or_else(|| RenderError::new("range: missing start parameter"))?;
        let end = h.param(1)
            .ok_or_else(|| RenderError::new("range: missing end parameter"))?;

        let start_num = start.value().as_i64().unwrap_or(0);
        let end_num = end.value().as_i64().unwrap_or(0);

        let range: Vec<String> = (start_num..end_num)
            .map(|i| i.to_string())
            .collect();

        out.write(&range.join(", "))?;
        Ok(())
    }

    /// Format currency helper: {{format_currency amount "USD"}}
    fn format_currency_helper(
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        let amount = h.param(0)
            .ok_or_else(|| RenderError::new("format_currency: missing amount parameter"))?;
        
        let currency = h.param(1)
            .and_then(|v| v.value().as_str())
            .unwrap_or("USD");

        let amount_num = amount.value().as_f64().unwrap_or(0.0);
        let symbol = match currency {
            "USD" => "$",
            "EUR" => "€",
            "GBP" => "£",
            "JPY" => "¥",
            _ => currency,
        };

        let formatted = if currency == "JPY" {
            format!("{}{:.0}", symbol, amount_num)
        } else {
            format!("{}{:.2}", symbol, amount_num)
        };

        out.write(&formatted)?;
        Ok(())
    }

    /// Format phone helper: {{format_phone phone_number "US"}}
    fn format_phone_helper(
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        let phone = h.param(0)
            .ok_or_else(|| RenderError::new("format_phone: missing phone parameter"))?;
        
        let country = h.param(1)
            .and_then(|v| v.value().as_str())
            .unwrap_or("US");

        let phone_str = match phone.value() {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            _ => return Err(RenderError::new("format_phone: phone must be string or number")),
        };

        // Simple US phone formatting
        let digits: String = phone_str.chars().filter(|c| c.is_ascii_digit()).collect();
        
        let formatted = match country {
            "US" if digits.len() == 10 => {
                format!("({}) {}-{}", &digits[0..3], &digits[3..6], &digits[6..10])
            }
            "US" if digits.len() == 11 && digits.starts_with('1') => {
                format!("+1 ({}) {}-{}", &digits[1..4], &digits[4..7], &digits[7..11])
            }
            _ => phone_str, // Return original if we can't format
        };

        out.write(&formatted)?;
        Ok(())
    }

    /// Format address helper: {{format_address address}}
    fn format_address_helper(
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        let address = h.param(0)
            .ok_or_else(|| RenderError::new("format_address: missing address parameter"))?;

        let formatted = match address.value() {
            Value::Object(obj) => {
                let street = obj.get("street").and_then(|v| v.as_str()).unwrap_or("");
                let city = obj.get("city").and_then(|v| v.as_str()).unwrap_or("");
                let state = obj.get("state").and_then(|v| v.as_str()).unwrap_or("");
                let zip = obj.get("zip").and_then(|v| v.as_str()).unwrap_or("");
                
                let mut parts = Vec::new();
                if !street.is_empty() { 
                    parts.push(street.to_string()); 
                }
                
                // Build city, state, zip line
                let mut city_line_parts = Vec::new();
                if !city.is_empty() { 
                    city_line_parts.push(city.to_string()); 
                }
                if !state.is_empty() && !zip.is_empty() {
                    city_line_parts.push(format!("{} {}", state, zip));
                } else if !state.is_empty() {
                    city_line_parts.push(state.to_string());
                } else if !zip.is_empty() {
                    city_line_parts.push(zip.to_string());
                }
                
                if !city_line_parts.is_empty() {
                    parts.push(city_line_parts.join(" "));
                }
                
                parts.join("<br/>")
            }
            Value::String(s) => s.replace('\n', "<br/>"),
            _ => "Invalid address format".to_string(),
        };

        out.write(&formatted)?;
        Ok(())
    }

    /// Truncate text helper: {{truncate text 100 "..."}}
    fn truncate_helper(
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        let text = h.param(0)
            .ok_or_else(|| RenderError::new("truncate: missing text parameter"))?;
        
        let length = h.param(1)
            .and_then(|v| v.value().as_u64())
            .unwrap_or(100) as usize;
        
        let suffix = h.param(2)
            .and_then(|v| v.value().as_str())
            .unwrap_or("...");

        let text_str = match text.value() {
            Value::String(s) => s.clone(),
            _ => return Err(RenderError::new("truncate: text must be a string")),
        };

        let truncated = if text_str.len() > length {
            format!("{}{}", &text_str[0..length], suffix)
        } else {
            text_str
        };

        out.write(&truncated)?;
        Ok(())
    }

    /// Capitalize helper: {{capitalize text}}
    fn capitalize_helper(
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        let text = h.param(0)
            .ok_or_else(|| RenderError::new("capitalize: missing text parameter"))?;

        let text_str = match text.value() {
            Value::String(s) => s.clone(),
            _ => return Err(RenderError::new("capitalize: text must be a string")),
        };

        let capitalized = text_str
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars.as_str().to_lowercase().chars()).collect(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        out.write(&capitalized)?;
        Ok(())
    }

    /// URL encode helper: {{url_encode text}}
    fn url_encode_helper(
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        let text = h.param(0)
            .ok_or_else(|| RenderError::new("url_encode: missing text parameter"))?;

        let text_str = match text.value() {
            Value::String(s) => s.clone(),
            _ => return Err(RenderError::new("url_encode: text must be a string")),
        };

        let encoded = urlencoding::encode(&text_str);
        out.write(&encoded)?;
        Ok(())
    }
}