use std::time::{Duration, Instant};
use crate::ui;

/// A simple progress tracker for CLI operations
pub struct ProgressTracker {
    operation_name: String,
    start_time: Instant,
    steps: Vec<String>,
    current_step: usize,
}

impl ProgressTracker {
    /// Create a new progress tracker with the given operation name
    pub fn new(operation_name: &str) -> Self {
        ui::section_header(operation_name);
        ProgressTracker {
            operation_name: operation_name.to_string(),
            start_time: Instant::now(),
            steps: Vec::new(),
            current_step: 0,
        }
    }
    
    /// Add steps to the tracker
    pub fn with_steps(mut self, steps: Vec<String>) -> Self {
        self.steps = steps;
        self
    }
    
    /// Start the next step
    pub fn start_step(&mut self) -> &str {
        if self.current_step < self.steps.len() {
            let step = &self.steps[self.current_step];
            ui::status_message(step);
            step
        } else {
            ""
        }
    }
    
    /// Complete the current step
    pub fn complete_step(&mut self) {
        if self.current_step < self.steps.len() {
            ui::success_message(&self.steps[self.current_step]);
            self.current_step += 1;
        }
    }
    
    /// Skip the current step
    pub fn skip_step(&mut self, reason: &str) {
        if self.current_step < self.steps.len() {
            ui::warning_message(&format!("Skipped: {} ({})", self.steps[self.current_step], reason));
            self.current_step += 1;
        }
    }
    
    /// Complete the operation
    pub fn complete(&self) {
        let elapsed = self.start_time.elapsed();
        ui::success_message(&format!("{} completed in {}", 
            self.operation_name, 
            Self::format_duration(elapsed)));
    }
    
    /// Format a duration in a human-readable way
    fn format_duration(duration: Duration) -> String {
        let seconds = duration.as_secs();
        if seconds < 60 {
            format!("{} seconds", seconds)
        } else if seconds < 3600 {
            format!("{} minutes {} seconds", seconds / 60, seconds % 60)
        } else {
            format!("{} hours {} minutes", seconds / 3600, (seconds % 3600) / 60)
        }
    }
}