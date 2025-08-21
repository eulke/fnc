use std::time::Instant;

/// Progress tracking for concurrent and chained execution
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProgressTracker {
    pub total_requests: usize,
    pub completed_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub skipped_routes: usize,
    #[serde(skip, default = "Instant::now")]
    pub start_time: Instant,
    /// Additional fields for chain execution tracking
    pub current_batch: usize,
    pub total_batches: usize,
    pub extraction_completed: usize,
    pub dependency_wait_count: usize,
}

impl ProgressTracker {
    pub fn new(total_requests: usize) -> Self {
        Self {
            total_requests,
            completed_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            skipped_routes: 0,
            start_time: Instant::now(),
            current_batch: 0,
            total_batches: 0,
            extraction_completed: 0,
            dependency_wait_count: 0,
        }
    }
    
    /// Create a new progress tracker for chained execution
    pub fn new_for_chains(total_requests: usize, total_batches: usize) -> Self {
        Self {
            total_requests,
            completed_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            skipped_routes: 0,
            start_time: Instant::now(),
            current_batch: 0,
            total_batches,
            extraction_completed: 0,
            dependency_wait_count: 0,
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

    pub fn route_skipped(&mut self) {
        self.skipped_routes += 1;
    }
    
    /// Mark batch as completed
    pub fn batch_completed(&mut self) {
        self.current_batch += 1;
    }
    
    /// Mark value extraction as completed
    pub fn extraction_completed(&mut self) {
        self.extraction_completed += 1;
    }
    
    /// Update dependency wait count
    pub fn set_dependency_wait_count(&mut self, count: usize) {
        self.dependency_wait_count = count;
    }
    
    /// Get batch progress percentage
    pub fn batch_progress_percentage(&self) -> f64 {
        if self.total_batches == 0 {
            0.0
        } else {
            (self.current_batch as f64 / self.total_batches as f64) * 100.0
        }
    }
    
    /// Get extraction completion percentage
    pub fn extraction_progress_percentage(&self) -> f64 {
        if self.completed_requests == 0 {
            0.0
        } else {
            (self.extraction_completed as f64 / self.completed_requests as f64) * 100.0
        }
    }
    
    /// Check if this is chain execution mode
    pub fn is_chain_execution(&self) -> bool {
        self.total_batches > 0
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
