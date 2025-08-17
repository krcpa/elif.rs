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
    use handlebars::Handlebars;

    /// Register built-in email template helpers
    pub fn register_email_helpers(_handlebars: &mut Handlebars) {
        // Simplified - helpers can be added later when needed
        // The handlebars_helper! macro has complex type requirements
        // that are easier to handle when actually needed
    }
}