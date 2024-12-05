use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub struct DeployProgress {
    pb: ProgressBar,
}

impl DeployProgress {
    pub fn new() -> Self {
        let total_steps = 7; // Total number of steps in deployment
        let pb = ProgressBar::new(total_steps);
        
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-")
        );
        
        pb.enable_steady_tick(Duration::from_millis(100));
        
        Self { pb }
    }

    pub fn status_check(&self) {
        self.pb.set_message("Checking repository status...");
    }

    pub fn branch_checkout(&self, branch: &str) {
        self.pb.set_message(format!("Checking out branch: {}", branch));
        self.pb.inc(1);
    }

    pub fn pulling(&self) {
        self.pb.set_message("Pulling latest changes...");
        self.pb.inc(1);
    }

    pub fn version_increment(&self, from: &str, to: &str) {
        self.pb.set_message(format!("Incrementing version {} -> {}", from, to));
        self.pb.inc(1);
    }

    pub fn branch_creation(&self, branch: &str) {
        self.pb.set_message(format!("Creating branch: {}", branch));
        self.pb.inc(1);
    }

    pub fn branch_switch(&self, branch: &str) {
        self.pb.set_message(format!("Switching to branch: {}", branch));
        self.pb.inc(1);
    }

    pub fn updating_version(&self) {
        self.pb.set_message("Updating version in package files...");
        self.pb.inc(1);
    }

    pub fn finish(&self, success: bool) {
        if success {
            self.pb.finish_with_message("✨ Deployment preparation complete!");
        } else {
            self.pb.finish_with_message("❌ Deployment preparation failed");
        }
    }
}
