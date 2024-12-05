use std::{io::{self, Write}, time::Duration};
use indicatif::{ProgressBar, ProgressStyle};

pub struct DeployProgress {
    current_step: Option<ProgressBar>,
    current_emoji: Option<&'static str>,
    step_number: usize,
    total_steps: usize,
}

impl DeployProgress {
    pub fn new() -> Self {
        Self { 
            current_step: None,
            current_emoji: None,
            step_number: 0,
            total_steps: 7, // Total number of deploy steps
        }
    }

    fn create_spinner(message: String) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                .template("{spinner:.blue} {msg}")
                .unwrap()
        );
        pb.set_message(message);
        pb.enable_steady_tick(Duration::from_millis(100));
        pb
    }

    fn finish_step(&mut self, success: bool, message: Option<String>) {
        if let Some(pb) = self.current_step.take() {
            pb.finish_and_clear();
            
            let emoji = if success { 
                self.current_emoji.unwrap_or("✅")
            } else { 
                "❌" 
            };
            
            let msg = message.unwrap_or_else(|| pb.message().to_string());
            
            // Extract the actual message without the step numbers
            let msg_parts: Vec<&str> = msg.split("] ").collect();
            let actual_msg = msg_parts.get(1).map(|s| *s).unwrap_or(&msg);
            
            println!("[{}/{}] {} {}", self.step_number, self.total_steps, emoji, actual_msg);
        }
        self.current_emoji = None;
    }

    fn start_step(&mut self, message: String, completion_emoji: &'static str) {
        // Finish previous step if any
        if self.current_step.is_some() {
            self.finish_step(true, None);
        }
        
        // Increment step number for new step
        self.step_number += 1;
        
        // Store the emoji for this step
        self.current_emoji = Some(completion_emoji);
        
        // Create new step with {steps} {spinner} {message} format
        let step_message = format!("[{}/{}] {}", self.step_number, self.total_steps, message);
        self.current_step = Some(Self::create_spinner(step_message));
    }

    pub fn status_check(&mut self) {
        self.start_step("Checking repository status...".into(), "🔍");
    }

    pub fn branch_checkout(&mut self, branch: &str) {
        self.start_step(format!("Checking out branch: {}", branch), "🔄");
    }

    pub fn pulling(&mut self) {
        self.start_step("Pulling latest changes...".into(), "⬇️ ");
    }

    pub fn version_increment(&mut self, from: &str, to: &str) {
        self.start_step(format!("Incrementing version {} -> {}", from, to), "📝");
    }

    pub fn branch_creation(&mut self, branch: &str) {
        self.start_step(format!("Creating branch: {}", branch), "🌿");
    }

    pub fn branch_switch(&mut self, branch: &str) {
        self.start_step(format!("Switching to branch: {}", branch), "🔀");
    }

    pub fn updating_version(&mut self) {
        self.start_step("Updating version in manifest file...".into(), "📝");
    }

    pub fn finish(&mut self, success: bool) {
        // Finish last step with its specific emoji
        if self.current_step.is_some() {
            self.finish_step(success, None);
        }
        
        // Reset step number
        self.step_number = 0;
        
        // Final status message
        let (emoji, status) = if success {
            ("🚀", "Deploy preparation completed successfully!")
        } else {
            ("💥", "Deploy preparation failed!")
        };
        
        println!("\n{} {}\n", emoji, status);
        io::stdout().flush().unwrap();
    }
}
