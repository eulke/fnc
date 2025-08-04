use std::time::Instant;

/// Progress tracking for concurrent execution
#[derive(Debug, Clone)]
pub struct ProgressTracker {
    pub total_requests: usize,
    pub completed_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub start_time: Instant,
}

impl ProgressTracker {
    pub fn new(total_requests: usize) -> Self {
        Self {
            total_requests,
            completed_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            start_time: Instant::now(),
        }
    }

    pub fn request_completed(&mut self, success: bool) {
        self.completed_requests += 1;
        if success {
            self.successful_requests += 1;
        } else {
            self.failed_requests += 1;
        }
    }

    pub fn progress_percentage(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.completed_requests as f64 / self.total_requests as f64) * 100.0
        }
    }

    pub fn elapsed_time(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    pub fn estimated_remaining(&self) -> Option<std::time::Duration> {
        if self.completed_requests == 0 || self.completed_requests >= self.total_requests {
            return None;
        }

        let elapsed = self.elapsed_time();
        let avg_time_per_request = elapsed.as_secs_f64() / self.completed_requests as f64;
        let remaining_requests = self.total_requests - self.completed_requests;
        let estimated_seconds = avg_time_per_request * remaining_requests as f64;

        Some(std::time::Duration::from_secs_f64(estimated_seconds))
    }
}

/// Alias for progress callback to reduce type complexity lint
pub type ProgressCallback = dyn Fn(&ProgressTracker) + Send + Sync;