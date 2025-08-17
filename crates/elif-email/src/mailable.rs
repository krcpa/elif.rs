use crate::{Email, EmailError, templates::TemplateEngine};
use async_trait::async_trait;
use serde::Serialize;

/// Trait for objects that can be converted to emails
#[async_trait]
pub trait Mailable: Send + Sync {
    /// Build the email from this mailable object
    async fn build(&self) -> Result<Email, EmailError>;
    
    /// Get the template name (optional)
    fn template_name(&self) -> Option<&str> {
        None
    }
    
    /// Get template context data (optional)
    fn template_context(&self) -> Result<Option<serde_json::Value>, EmailError> {
        Ok(None)
    }
    
    /// Customize the email after template rendering (optional)
    async fn customize_email(&self, email: Email) -> Result<Email, EmailError> {
        Ok(email)
    }
}

/// Base mailable struct that can be extended
#[derive(Debug, Clone)]
pub struct BaseMailable {
    /// Email recipient
    pub to: String,
    /// Email sender (optional, will use default if not set)
    pub from: Option<String>,
    /// Template name
    pub template: Option<String>,
    /// Template context
    pub context: serde_json::Value,
    /// Email subject
    pub subject: Option<String>,
}

impl BaseMailable {
    /// Create new base mailable
    pub fn new(to: impl Into<String>) -> Self {
        Self {
            to: to.into(),
            from: None,
            template: None,
            context: serde_json::Value::Null,
            subject: None,
        }
    }

    /// Set sender
    pub fn from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }

    /// Set template
    pub fn template(mut self, template: impl Into<String>) -> Self {
        self.template = Some(template.into());
        self
    }

    /// Set context from serializable data
    pub fn context<T: Serialize>(mut self, data: T) -> Result<Self, EmailError> {
        self.context = serde_json::to_value(data)?;
        Ok(self)
    }

    /// Set subject
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }
}

#[async_trait]
impl Mailable for BaseMailable {
    async fn build(&self) -> Result<Email, EmailError> {
        let mut email = Email::new().to(self.to.clone());
        
        if let Some(ref from) = self.from {
            email = email.from(from.clone());
        }
        
        if let Some(ref subject) = self.subject {
            email = email.subject(subject.clone());
        }
        
        Ok(email)
    }
    
    fn template_name(&self) -> Option<&str> {
        self.template.as_deref()
    }
    
    fn template_context(&self) -> Result<Option<serde_json::Value>, EmailError> {
        if self.context.is_null() {
            Ok(None)
        } else {
            Ok(Some(self.context.clone()))
        }
    }
}

/// Mailable builder that integrates with template engine
pub struct MailableBuilder<'a> {
    mailable: Box<dyn Mailable>,
    template_engine: Option<&'a TemplateEngine>,
    default_from: Option<String>,
}

impl<'a> MailableBuilder<'a> {
    /// Create new mailable builder
    pub fn new(mailable: Box<dyn Mailable>) -> Self {
        Self {
            mailable,
            template_engine: None,
            default_from: None,
        }
    }

    /// Set template engine
    pub fn with_template_engine(mut self, engine: &'a TemplateEngine) -> Self {
        self.template_engine = Some(engine);
        self
    }

    /// Set default from address
    pub fn with_default_from(mut self, from: impl Into<String>) -> Self {
        self.default_from = Some(from.into());
        self
    }

    /// Build the email
    pub async fn build(self) -> Result<Email, EmailError> {
        let mut email = self.mailable.build().await?;
        
        // Set default from if not already set
        if email.from.is_empty() {
            if let Some(default_from) = self.default_from {
                email = email.from(default_from);
            }
        }
        
        // Apply template if available
        if let (Some(template_name), Some(engine)) = (self.mailable.template_name(), self.template_engine) {
            if let Some(context_value) = self.mailable.template_context()? {
                let context = match context_value {
                    serde_json::Value::Object(map) => map.into_iter().collect(),
                    _ => {
                        let mut ctx = std::collections::HashMap::new();
                        ctx.insert("data".to_string(), context_value);
                        ctx
                    }
                };
                
                email = email.with_template(engine, template_name, context)?;
            }
        }
        
        // Apply custom modifications
        email = self.mailable.customize_email(email).await?;
        
        Ok(email)
    }
}

/// Common mailable implementations

/// Welcome email mailable
#[derive(Debug, Clone, Serialize)]
pub struct WelcomeEmail {
    pub to: String,
    pub user_name: String,
    pub activation_link: Option<String>,
}

impl WelcomeEmail {
    pub fn new(to: impl Into<String>, user_name: impl Into<String>) -> Self {
        Self {
            to: to.into(),
            user_name: user_name.into(),
            activation_link: None,
        }
    }

    pub fn with_activation_link(mut self, link: impl Into<String>) -> Self {
        self.activation_link = Some(link.into());
        self
    }
}

#[async_trait]
impl Mailable for WelcomeEmail {
    async fn build(&self) -> Result<Email, EmailError> {
        Ok(Email::new()
            .to(self.to.clone())
            .subject(format!("Welcome {}!", self.user_name)))
    }
    
    fn template_name(&self) -> Option<&str> {
        Some("welcome")
    }
    
    fn template_context(&self) -> Result<Option<serde_json::Value>, EmailError> {
        Ok(Some(serde_json::to_value(self)?))
    }
}

/// Password reset email mailable
#[derive(Debug, Clone, Serialize)]
pub struct PasswordResetEmail {
    pub to: String,
    pub user_name: String,
    pub reset_link: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl PasswordResetEmail {
    pub fn new(
        to: impl Into<String>,
        user_name: impl Into<String>,
        reset_link: impl Into<String>,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            to: to.into(),
            user_name: user_name.into(),
            reset_link: reset_link.into(),
            expires_at,
        }
    }
}

#[async_trait]
impl Mailable for PasswordResetEmail {
    async fn build(&self) -> Result<Email, EmailError> {
        Ok(Email::new()
            .to(self.to.clone())
            .subject("Password Reset Request"))
    }
    
    fn template_name(&self) -> Option<&str> {
        Some("password_reset")
    }
    
    fn template_context(&self) -> Result<Option<serde_json::Value>, EmailError> {
        Ok(Some(serde_json::to_value(self)?))
    }
}

/// Invoice email mailable
#[derive(Debug, Clone, Serialize)]
pub struct InvoiceEmail {
    pub to: String,
    pub customer_name: String,
    pub invoice_number: String,
    pub amount: f64,
    pub due_date: chrono::DateTime<chrono::Utc>,
    pub pdf_attachment: Option<Vec<u8>>,
}

impl InvoiceEmail {
    pub fn new(
        to: impl Into<String>,
        customer_name: impl Into<String>,
        invoice_number: impl Into<String>,
        amount: f64,
        due_date: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            to: to.into(),
            customer_name: customer_name.into(),
            invoice_number: invoice_number.into(),
            amount,
            due_date,
            pdf_attachment: None,
        }
    }

    pub fn with_pdf_attachment(mut self, pdf_data: Vec<u8>) -> Self {
        self.pdf_attachment = Some(pdf_data);
        self
    }
}

#[async_trait]
impl Mailable for InvoiceEmail {
    async fn build(&self) -> Result<Email, EmailError> {
        let mut email = Email::new()
            .to(self.to.clone())
            .subject(format!("Invoice {} - ${:.2}", self.invoice_number, self.amount));

        // Add PDF attachment if provided
        if let Some(ref pdf_data) = self.pdf_attachment {
            let attachment = crate::Attachment::new(
                format!("invoice_{}.pdf", self.invoice_number),
                pdf_data.clone()
            ).with_content_type("application/pdf");
            email = email.attach(attachment);
        }

        Ok(email)
    }
    
    fn template_name(&self) -> Option<&str> {
        Some("invoice")
    }
    
    fn template_context(&self) -> Result<Option<serde_json::Value>, EmailError> {
        Ok(Some(serde_json::to_value(self)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_base_mailable() {
        let mailable = BaseMailable::new("test@example.com")
            .from("sender@example.com")
            .subject("Test Subject");

        let email = mailable.build().await.unwrap();
        assert_eq!(email.to, vec!["test@example.com"]);
        assert_eq!(email.from, "sender@example.com");
        assert_eq!(email.subject, "Test Subject");
    }

    #[tokio::test]
    async fn test_welcome_email() {
        let mailable = WelcomeEmail::new("user@example.com", "John Doe")
            .with_activation_link("https://example.com/activate");

        let email = mailable.build().await.unwrap();
        assert_eq!(email.to, vec!["user@example.com"]);
        assert_eq!(email.subject, "Welcome John Doe!");
        assert_eq!(mailable.template_name(), Some("welcome"));
    }

    #[tokio::test]
    async fn test_mailable_builder() {
        let mailable = Box::new(BaseMailable::new("test@example.com"));
        let builder = MailableBuilder::new(mailable)
            .with_default_from("default@example.com");

        let email = builder.build().await.unwrap();
        assert_eq!(email.from, "default@example.com");
    }
}