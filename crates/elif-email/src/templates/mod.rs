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
    pub fn render(
        &self,
        engine: &TemplateEngine,
        context: &TemplateContext,
    ) -> Result<RenderedEmail, EmailError> {
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
            serde_json::Value::Object(map) => Ok(map.into_iter().collect()),
            _ => {
                let mut context = TemplateContext::new();
                context.insert("data".to_string(), value);
                Ok(context)
            }
        }
    }
}

/// Helper for TemplateContext to avoid blanket implementation conflict
pub fn template_context_into_context(
    context: TemplateContext,
) -> Result<TemplateContext, EmailError> {
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
        template: &EmailTemplate,
        context: impl IntoTemplateContext,
    ) -> Result<Self, EmailError> {
        let context = context.into_context()?;
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

    /// Update email with rendered template content by name
    pub fn with_template(
        mut self,
        engine: &TemplateEngine,
        template_name: &str,
        context: impl IntoTemplateContext,
    ) -> Result<Self, EmailError> {
        let context = context.into_context()?;
        let rendered = engine.render_template_by_name(template_name, &context)?;

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

    /// Update email with rendered template content using EmailTemplate struct
    pub fn with_template_struct(
        mut self,
        engine: &TemplateEngine,
        template: &EmailTemplate,
        context: impl IntoTemplateContext,
    ) -> Result<Self, EmailError> {
        let context = context.into_context()?;
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

/// # Tera Template Engine Documentation
///
/// This module provides email templating using the Tera template engine.
/// Tera uses Jinja2-like syntax and provides powerful features for email templates.
///
/// ## Quick Start
///
/// ```rust,ignore
/// use elif_email::templates::{TemplateEngine, EmailTemplate, TemplateContext};
/// use elif_email::config::TemplateConfig;
///
/// let config = TemplateConfig {
///     templates_dir: "templates".to_string(),
///     layouts_dir: "layouts".to_string(),
///     partials_dir: "partials".to_string(),
///     enable_cache: true,
///     template_extension: ".html".to_string(),
///     cache_size: Some(100),
///     watch_files: false,
/// };
///
/// let engine = TemplateEngine::new(config)?;
/// let template = EmailTemplate::builder("welcome")
///     .html_template("<h1>Welcome {{ user.name }}!</h1>")
///     .subject_template("Welcome to {{ app_name }}")
///     .build();
/// ```
///
/// ## Available Filters
///
/// ### Date/Time Filters
/// - `{{ date_value | format_date(format="%Y-%m-%d") }}`
/// - `{{ datetime_value | format_datetime(format="%Y-%m-%d %H:%M:%S") }}`
/// - `{{ "" | now(format="%Y-%m-%d %H:%M:%S") }}`
///
/// ### Tracking Filters
/// - `{{ email_id | tracking_pixel(base_url="https://example.com") }}`
/// - `{{ email_id | tracking_link(url="https://target.com", base_url="https://example.com") }}`
///
/// ### Formatting Filters
/// - `{{ amount | currency(currency="USD") }}`
/// - `{{ phone_number | phone(country="US") }}`
/// - `{{ address_obj | address }}`
///
/// ### String Filters
/// - `{{ text | url_encode }}`
/// - `{{ text | truncate(length=100) }}` (use built-in)
/// - `{{ text | title }}` (use built-in instead of capitalize)
///
/// ## Template Syntax
///
/// ### Conditionals
/// Use Tera's native syntax instead of helpers:
/// - `{% if value1 == value2 %}...{% endif %}` (instead of if_eq)
/// - `{% if not condition %}...{% endif %}` (instead of unless)  
/// - `{% if value1 > value2 %}...{% endif %}` (instead of if_gt)
/// - `{% if value1 < value2 %}...{% endif %}` (instead of if_lt)
///
/// ### Loops
/// Use Tera's native loop syntax:
/// - `{% for item in items %}{{ loop.index0 }}: {{ item }}{% endfor %}` (instead of each_with_index)
/// - `{% for i in range(start=0, end=10) %}{{ i }}{% endfor %}` (instead of range helper)
///
/// ### Migration from Handlebars
///
/// | Handlebars | Tera Equivalent |
/// |------------|-----------------|
/// | `{{#if condition}}` | `{% if condition %}` |
/// | `{{#unless condition}}` | `{% if not condition %}` |
/// | `{{#each items}}` | `{% for item in items %}` |
/// | `{{@index}}` | `{{ loop.index0 }}` |
/// | `{{#if_eq a b}}` | `{% if a == b %}` |
/// | `{{capitalize text}}` | `{{ text \| title }}` |
///
/// All filters are registered in the TemplateEngine and available in templates.
/// For full documentation, see [Tera docs](https://tera.netlify.app/docs/).
pub struct TemplateDocumentation;
