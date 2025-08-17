use elif_core::ElifError;
use std::path::Path;
use std::fs;

pub struct CodeWriter;

impl CodeWriter {
    pub fn new() -> Self {
        Self
    }
    
    pub fn write_if_changed(&self, path: &Path, content: &str) -> Result<(), ElifError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        if path.exists() {
            let existing = fs::read_to_string(path)?;
            if existing == content {
                return Ok(());
            }
        }
        
        fs::write(path, content)?;
        Ok(())
    }
    
    pub fn write_preserving_markers(&self, path: &Path, new_content: &str) -> Result<(), ElifError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        if !path.exists() {
            fs::write(path, new_content)?;
            return Ok(());
        }
        
        let existing = fs::read_to_string(path)?;
        let merged = self.merge_with_markers(&existing, new_content)?;
        
        if merged != existing {
            fs::write(path, merged)?;
        }
        
        Ok(())
    }
    
    fn merge_with_markers(&self, existing: &str, new_content: &str) -> Result<String, ElifError> {
        let marker_regex = regex::Regex::new(
            r"// <<<ELIF:BEGIN agent-editable:([^>]+)>>>(.*?)// <<<ELIF:END agent-editable:[^>]+>>>"
        ).map_err(|e| ElifError::Template { message: format!("Regex error: {}", e) })?;
        
        let mut markers = std::collections::HashMap::new();
        
        for cap in marker_regex.captures_iter(existing) {
            let id = cap.get(1).unwrap().as_str();
            let content = cap.get(2).unwrap().as_str();
            markers.insert(id, content);
        }
        
        let mut result = new_content.to_string();
        
        for (id, content) in markers {
            let begin_marker = format!("// <<<ELIF:BEGIN agent-editable:{}>>>", id);
            let end_marker = format!("// <<<ELIF:END agent-editable:{}>>>", id);
            
            if let Some(start) = result.find(&begin_marker) {
                if let Some(end_start) = result.find(&end_marker) {
                    let before = &result[..start + begin_marker.len()];
                    let after = &result[end_start..];
                    result = format!("{}{}{}", before, content, after);
                }
            }
        }
        
        Ok(result)
    }
}

impl Default for CodeWriter {
    fn default() -> Self {
        Self::new()
    }
}