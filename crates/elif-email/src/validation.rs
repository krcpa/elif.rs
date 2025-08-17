use crate::EmailError;
use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

/// Email validation utilities
pub struct EmailValidator {
    /// Regex for basic email validation
    email_regex: Regex,
    /// Domain blocklist
    blocked_domains: HashSet<String>,
    /// Domain allowlist (if set, only these domains are allowed)
    allowed_domains: Option<HashSet<String>>,
}

impl EmailValidator {
    /// Create new email validator
    pub fn new() -> Result<Self, EmailError> {
        let email_regex = Regex::new(
            r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"
        ).map_err(|e| EmailError::configuration(format!("Invalid email regex: {}", e)))?;

        Ok(Self {
            email_regex,
            blocked_domains: HashSet::new(),
            allowed_domains: None,
        })
    }

    /// Add blocked domain
    pub fn block_domain(&mut self, domain: impl Into<String>) -> &mut Self {
        self.blocked_domains.insert(domain.into().to_lowercase());
        self
    }

    /// Add multiple blocked domains
    pub fn block_domains(&mut self, domains: Vec<String>) -> &mut Self {
        for domain in domains {
            self.blocked_domains.insert(domain.to_lowercase());
        }
        self
    }

    /// Set allowed domains (only these will be accepted)
    pub fn set_allowed_domains(&mut self, domains: Vec<String>) -> &mut Self {
        let domains: HashSet<String> = domains.into_iter().map(|d| d.to_lowercase()).collect();
        self.allowed_domains = Some(domains);
        self
    }

    /// Validate email address
    pub fn validate(&self, email: &str) -> Result<(), EmailError> {
        let email = email.trim().to_lowercase();
        
        // Basic format validation
        if !self.email_regex.is_match(&email) {
            return Err(EmailError::validation("email", "Invalid email format"));
        }

        // Extract domain
        let domain = email.split('@').nth(1)
            .ok_or_else(|| EmailError::validation("email", "No domain found in email"))?;

        // Check domain blocklist
        if self.blocked_domains.contains(domain) {
            return Err(EmailError::validation("email", format!("Domain '{}' is blocked", domain)));
        }

        // Check domain allowlist
        if let Some(ref allowed_domains) = self.allowed_domains {
            if !allowed_domains.contains(domain) {
                return Err(EmailError::validation("email", format!("Domain '{}' is not allowed", domain)));
            }
        }

        Ok(())
    }

    /// Validate multiple email addresses
    pub fn validate_many(&self, emails: &[String]) -> Result<(), EmailError> {
        for (index, email) in emails.iter().enumerate() {
            self.validate(email).map_err(|e| {
                EmailError::validation(
                    &format!("email[{}]", index),
                    format!("Email '{}': {}", email, e)
                )
            })?;
        }
        Ok(())
    }

    /// Validate email and return normalized version
    pub fn validate_and_normalize(&self, email: &str) -> Result<String, EmailError> {
        let normalized = email.trim().to_lowercase();
        self.validate(&normalized)?;
        Ok(normalized)
    }
}

impl Default for EmailValidator {
    fn default() -> Self {
        Self::new().expect("Failed to create default email validator")
    }
}

/// Global email validator instance
static GLOBAL_VALIDATOR: OnceLock<EmailValidator> = OnceLock::new();

/// Get global email validator
pub fn global_validator() -> &'static EmailValidator {
    GLOBAL_VALIDATOR.get_or_init(EmailValidator::default)
}

/// Initialize global validator with custom settings
pub fn init_global_validator(validator: EmailValidator) -> Result<(), EmailError> {
    GLOBAL_VALIDATOR.set(validator).map_err(|_| {
        EmailError::configuration("Global email validator already initialized")
    })
}

/// Quick email validation function
pub fn validate_email(email: &str) -> Result<(), EmailError> {
    global_validator().validate(email)
}

/// Quick email normalization function
pub fn normalize_email(email: &str) -> Result<String, EmailError> {
    global_validator().validate_and_normalize(email)
}

/// Email validation builder for configuration
pub struct EmailValidatorBuilder {
    validator: EmailValidator,
}

impl EmailValidatorBuilder {
    /// Create new validator builder
    pub fn new() -> Result<Self, EmailError> {
        Ok(Self {
            validator: EmailValidator::new()?,
        })
    }

    /// Block domain
    pub fn block_domain(mut self, domain: impl Into<String>) -> Self {
        self.validator.block_domain(domain);
        self
    }

    /// Block multiple domains
    pub fn block_domains(mut self, domains: Vec<String>) -> Self {
        self.validator.block_domains(domains);
        self
    }

    /// Set allowed domains
    pub fn allowed_domains(mut self, domains: Vec<String>) -> Self {
        self.validator.set_allowed_domains(domains);
        self
    }

    /// Build the validator
    pub fn build(self) -> EmailValidator {
        self.validator
    }
}

impl Default for EmailValidatorBuilder {
    fn default() -> Self {
        Self::new().expect("Failed to create email validator builder")
    }
}

/// Common domain blocklists
pub mod blocklists {
    /// Disposable email domains
    pub const DISPOSABLE_DOMAINS: &[&str] = &[
        "10minutemail.com",
        "guerrillamail.com",
        "mailinator.com",
        "tempmail.org",
        "yopmail.com",
        "throwaway.email",
        "temp-mail.org",
        "fake-mail.ml",
    ];

    /// Test domains (should be blocked in production)
    pub const TEST_DOMAINS: &[&str] = &[
        "example.com",
        "example.org",
        "test.com",
        "localhost",
    ];

    /// Get disposable email domains as Vec<String>
    pub fn disposable_domains() -> Vec<String> {
        DISPOSABLE_DOMAINS.iter().map(|s| s.to_string()).collect()
    }

    /// Get test domains as Vec<String>
    pub fn test_domains() -> Vec<String> {
        TEST_DOMAINS.iter().map(|s| s.to_string()).collect()
    }
}

/// Email content validation
pub struct EmailContentValidator;

impl EmailContentValidator {
    /// Validate email subject
    pub fn validate_subject(subject: &str) -> Result<(), EmailError> {
        if subject.is_empty() {
            return Err(EmailError::validation("subject", "Subject cannot be empty"));
        }

        if subject.len() > 998 {
            return Err(EmailError::validation("subject", "Subject too long (max 998 characters)"));
        }

        // Check for spam-like content
        let spam_keywords = ["URGENT", "FREE MONEY", "CLICK HERE NOW", "GUARANTEED"];
        let upper_subject = subject.to_uppercase();
        
        let spam_count = spam_keywords.iter()
            .filter(|&&keyword| upper_subject.contains(keyword))
            .count();
        
        if spam_count >= 2 {
            return Err(EmailError::validation("subject", "Subject contains spam-like content"));
        }

        Ok(())
    }

    /// Validate email body length
    pub fn validate_body_length(body: &str, max_length: usize) -> Result<(), EmailError> {
        if body.len() > max_length {
            return Err(EmailError::validation(
                "body", 
                format!("Body too long (max {} characters)", max_length)
            ));
        }
        Ok(())
    }

    /// Validate that email has some content
    pub fn validate_has_content(html_body: &Option<String>, text_body: &Option<String>) -> Result<(), EmailError> {
        if html_body.is_none() && text_body.is_none() {
            return Err(EmailError::validation("body", "Email must have either HTML or text body"));
        }

        if let Some(html) = html_body {
            if html.trim().is_empty() && text_body.as_ref().map_or(true, |t| t.trim().is_empty()) {
                return Err(EmailError::validation("body", "Email body cannot be empty"));
            }
        } else if let Some(text) = text_body {
            if text.trim().is_empty() {
                return Err(EmailError::validation("body", "Email body cannot be empty"));
            }
        }

        Ok(())
    }
}

/// Extension methods for Email validation
impl crate::Email {
    /// Validate this email
    pub fn validate(&self) -> Result<(), EmailError> {
        // Validate sender
        if !self.from.is_empty() {
            validate_email(&self.from)?;
        }

        // Validate recipients
        if self.to.is_empty() {
            return Err(EmailError::validation("to", "Email must have at least one recipient"));
        }
        
        for (index, to_addr) in self.to.iter().enumerate() {
            validate_email(to_addr).map_err(|e| {
                EmailError::validation(&format!("to[{}]", index), e.to_string())
            })?;
        }

        // Validate CC recipients
        if let Some(ref cc_list) = self.cc {
            for (index, cc_addr) in cc_list.iter().enumerate() {
                validate_email(cc_addr).map_err(|e| {
                    EmailError::validation(&format!("cc[{}]", index), e.to_string())
                })?;
            }
        }

        // Validate BCC recipients
        if let Some(ref bcc_list) = self.bcc {
            for (index, bcc_addr) in bcc_list.iter().enumerate() {
                validate_email(bcc_addr).map_err(|e| {
                    EmailError::validation(&format!("bcc[{}]", index), e.to_string())
                })?;
            }
        }

        // Validate reply-to
        if let Some(ref reply_to) = self.reply_to {
            validate_email(reply_to)?;
        }

        // Validate subject
        EmailContentValidator::validate_subject(&self.subject)?;

        // Validate body content
        EmailContentValidator::validate_has_content(&self.html_body, &self.text_body)?;

        // Validate body lengths
        if let Some(ref html) = self.html_body {
            EmailContentValidator::validate_body_length(html, 102_400)?; // 100KB
        }
        if let Some(ref text) = self.text_body {
            EmailContentValidator::validate_body_length(text, 102_400)?; // 100KB
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation() {
        let validator = EmailValidator::new().unwrap();

        // Valid emails
        assert!(validator.validate("user@example.com").is_ok());
        assert!(validator.validate("test.email+tag@domain.co.uk").is_ok());
        
        // Invalid emails
        assert!(validator.validate("invalid-email").is_err());
        assert!(validator.validate("@domain.com").is_err());
        assert!(validator.validate("user@").is_err());
    }

    #[test]
    fn test_domain_blocking() {
        let mut validator = EmailValidator::new().unwrap();
        validator.block_domain("spam.com");

        assert!(validator.validate("user@example.com").is_ok());
        assert!(validator.validate("user@spam.com").is_err());
    }

    #[test]
    fn test_domain_allowlist() {
        let mut validator = EmailValidator::new().unwrap();
        validator.set_allowed_domains(vec!["allowed.com".to_string()]);

        assert!(validator.validate("user@allowed.com").is_ok());
        assert!(validator.validate("user@other.com").is_err());
    }

    #[test]
    fn test_email_normalization() {
        let validator = EmailValidator::new().unwrap();
        
        let normalized = validator.validate_and_normalize("  User@Example.COM  ").unwrap();
        assert_eq!(normalized, "user@example.com");
    }

    #[test]
    fn test_subject_validation() {
        assert!(EmailContentValidator::validate_subject("Valid Subject").is_ok());
        assert!(EmailContentValidator::validate_subject("").is_err());
        
        let long_subject = "a".repeat(1000);
        assert!(EmailContentValidator::validate_subject(&long_subject).is_err());
        
        assert!(EmailContentValidator::validate_subject("URGENT FREE MONEY NOW").is_err());
    }

    #[test]
    fn test_email_struct_validation() {
        let mut email = crate::Email::new()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Test Subject")
            .text_body("Hello World");

        assert!(email.validate().is_ok());

        // Remove body to make it invalid
        email.text_body = None;
        assert!(email.validate().is_err());
    }
}