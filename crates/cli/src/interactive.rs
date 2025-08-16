use std::io::{self, Write};

/// Interactive prompt utilities for CLI commands
pub struct Prompt;

impl Prompt {
    /// Ask a yes/no question with a default answer
    pub fn confirm(message: &str, default: bool) -> io::Result<bool> {
        let default_str = if default { "Y/n" } else { "y/N" };
        print!("{} [{}]: ", message, default_str);
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let input = input.trim().to_lowercase();
        
        match input.as_str() {
            "" => Ok(default),
            "y" | "yes" | "true" | "1" => Ok(true),
            "n" | "no" | "false" | "0" => Ok(false),
            _ => {
                println!("Please enter 'y' or 'n'");
                Self::confirm(message, default)
            }
        }
    }
    
    /// Ask for a string input with optional default value
    pub fn input(message: &str, default: Option<&str>) -> io::Result<String> {
        if let Some(def) = default {
            print!("{} [{}]: ", message, def);
        } else {
            print!("{}: ", message);
        }
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let input = input.trim();
        
        if input.is_empty() {
            if let Some(def) = default {
                Ok(def.to_string())
            } else {
                println!("Input cannot be empty");
                Self::input(message, default)
            }
        } else {
            Ok(input.to_string())
        }
    }
    
    /// Ask for a number input with validation
    pub fn number<T>(message: &str, default: Option<T>) -> io::Result<T> 
    where
        T: std::str::FromStr + std::fmt::Display + Copy,
        T::Err: std::fmt::Debug,
    {
        let prompt = if let Some(def) = default {
            format!("{} [{}]: ", message, def)
        } else {
            format!("{}: ", message)
        };
        
        print!("{}", prompt);
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let input = input.trim();
        
        if input.is_empty() {
            if let Some(def) = default {
                Ok(def)
            } else {
                println!("Input cannot be empty");
                Self::number(message, default)
            }
        } else {
            match input.parse::<T>() {
                Ok(num) => Ok(num),
                Err(_) => {
                    println!("Please enter a valid number");
                    Self::number(message, default)
                }
            }
        }
    }
    
    /// Select from a list of options
    pub fn select<T>(message: &str, options: &[(T, &str)]) -> io::Result<T>
    where
        T: Clone,
    {
        println!("{}", message);
        
        for (i, (_, desc)) in options.iter().enumerate() {
            println!("  {}. {}", i + 1, desc);
        }
        
        print!("Select an option (1-{}): ", options.len());
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let input = input.trim();
        
        match input.parse::<usize>() {
            Ok(num) if num > 0 && num <= options.len() => {
                Ok(options[num - 1].0.clone())
            }
            _ => {
                println!("Please enter a number between 1 and {}", options.len());
                Self::select(message, options)
            }
        }
    }
    
    /// Multi-select from a list of options
    pub fn multi_select<T>(message: &str, options: &[(T, &str)]) -> io::Result<Vec<T>>
    where
        T: Clone,
    {
        println!("{}", message);
        println!("Enter numbers separated by commas (e.g., 1,3,5) or 'all' for everything:");
        
        for (i, (_, desc)) in options.iter().enumerate() {
            println!("  {}. {}", i + 1, desc);
        }
        
        print!("Select options: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let input = input.trim().to_lowercase();
        
        if input == "all" {
            return Ok(options.iter().map(|(val, _)| val.clone()).collect());
        }
        
        let mut selected = Vec::new();
        
        for part in input.split(',') {
            let part = part.trim();
            if let Ok(num) = part.parse::<usize>() {
                if num > 0 && num <= options.len() {
                    selected.push(options[num - 1].0.clone());
                } else {
                    println!("Warning: {} is not a valid option", num);
                }
            } else if !part.is_empty() {
                println!("Warning: '{}' is not a valid number", part);
            }
        }
        
        if selected.is_empty() {
            println!("No valid options selected. Please try again.");
            Self::multi_select(message, options)
        } else {
            Ok(selected)
        }
    }
    
    /// Ask for a password (hidden input)
    pub fn password(message: &str) -> io::Result<String> {
        print!("{}: ", message);
        io::stdout().flush()?;
        
        // In a real implementation, you'd use a crate like `rpassword` for hidden input
        // For now, we'll use regular input with a warning
        println!("⚠️  Warning: Password will be visible on screen");
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        Ok(input.trim().to_string())
    }
    
    /// Display a progress spinner while executing a task
    pub fn with_spinner<F, T>(message: &str, task: F) -> io::Result<T>
    where
        F: FnOnce() -> T,
    {
        print!("{} ", message);
        io::stdout().flush()?;
        
        // Simple spinner animation
        let chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let mut spinner_idx = 0;
        
        // In a real implementation, you'd run this in a separate thread
        // For now, just show the message
        let result = task();
        
        // Clear the line and show completion
        print!("\r{} ✅\n", message);
        io::stdout().flush()?;
        
        Ok(result)
    }
}

/// Progress bar for showing task completion
pub struct ProgressBar {
    message: String,
    total: usize,
    current: usize,
    width: usize,
}

impl ProgressBar {
    pub fn new(message: &str, total: usize) -> Self {
        Self {
            message: message.to_string(),
            total,
            current: 0,
            width: 40,
        }
    }
    
    pub fn update(&mut self, current: usize) -> io::Result<()> {
        self.current = current;
        self.draw()
    }
    
    pub fn increment(&mut self) -> io::Result<()> {
        self.current += 1;
        self.draw()
    }
    
    pub fn finish(&mut self) -> io::Result<()> {
        self.current = self.total;
        self.draw()?;
        println!();
        Ok(())
    }
    
    pub fn finish_with_message(&mut self, message: &str) -> io::Result<()> {
        self.current = self.total;
        self.draw()?;
        println!(" - {}", message);
        Ok(())
    }
    
    fn draw(&self) -> io::Result<()> {
        let percentage = if self.total > 0 {
            (self.current * 100) / self.total
        } else {
            100
        };
        
        let filled = if self.total > 0 {
            (self.current * self.width) / self.total
        } else {
            self.width
        };
        
        let bar = "█".repeat(filled) + &"░".repeat(self.width - filled);
        
        print!("\r{} [{}] {}% ({}/{})", 
            self.message, bar, percentage, self.current, self.total);
        io::stdout().flush()
    }
}

/// Output formatting utilities
pub struct Format;

impl Format {
    /// Print a success message
    pub fn success(message: &str) {
        println!("✅ {}", message);
    }
    
    /// Print an error message
    pub fn error(message: &str) {
        println!("❌ {}", message);
    }
    
    /// Print a warning message
    pub fn warning(message: &str) {
        println!("⚠️  {}", message);
    }
    
    /// Print an info message
    pub fn info(message: &str) {
        println!("ℹ️  {}", message);
    }
    
    /// Print a section header
    pub fn header(title: &str) {
        println!("\n{}", title);
        println!("{}", "=".repeat(title.len()));
    }
    
    /// Print a subsection header
    pub fn subheader(title: &str) {
        println!("\n{}", title);
        println!("{}", "-".repeat(title.len()));
    }
    
    /// Print a table with headers and rows
    pub fn table(headers: &[&str], rows: &[Vec<String>]) {
        if headers.is_empty() || rows.is_empty() {
            return;
        }
        
        // Calculate column widths
        let mut widths = headers.iter().map(|h| h.len()).collect::<Vec<_>>();
        
        for row in rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    widths[i] = widths[i].max(cell.len());
                }
            }
        }
        
        // Print header
        print!("┌");
        for (i, width) in widths.iter().enumerate() {
            print!("{}", "─".repeat(width + 2));
            if i < widths.len() - 1 {
                print!("┬");
            }
        }
        println!("┐");
        
        print!("│");
        for (i, header) in headers.iter().enumerate() {
            print!(" {:<width$} ", header, width = widths[i]);
            print!("│");
        }
        println!();
        
        // Print separator
        print!("├");
        for (i, width) in widths.iter().enumerate() {
            print!("{}", "─".repeat(width + 2));
            if i < widths.len() - 1 {
                print!("┼");
            }
        }
        println!("┤");
        
        // Print rows
        for row in rows {
            print!("│");
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    print!(" {:<width$} ", cell, width = widths[i]);
                    print!("│");
                }
            }
            println!();
        }
        
        // Print footer
        print!("└");
        for (i, width) in widths.iter().enumerate() {
            print!("{}", "─".repeat(width + 2));
            if i < widths.len() - 1 {
                print!("┴");
            }
        }
        println!("┘");
    }
    
    /// Print a list with bullets
    pub fn list(items: &[&str]) {
        for item in items {
            println!("• {}", item);
        }
    }
    
    /// Print numbered list
    pub fn numbered_list(items: &[&str]) {
        for (i, item) in items.iter().enumerate() {
            println!("{}. {}", i + 1, item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_progress_bar_creation() {
        let pb = ProgressBar::new("Testing", 100);
        assert_eq!(pb.message, "Testing");
        assert_eq!(pb.total, 100);
        assert_eq!(pb.current, 0);
    }
    
    #[test]
    fn test_format_functions() {
        // These functions print to stdout, so we can't easily test their output
        // In a real implementation, you might want to make them return strings
        Format::success("Test success");
        Format::error("Test error");
        Format::warning("Test warning");
        Format::info("Test info");
    }
}