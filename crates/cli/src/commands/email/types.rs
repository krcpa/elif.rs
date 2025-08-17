use std::collections::HashMap;
use serde_json::Value;
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