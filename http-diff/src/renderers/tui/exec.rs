use super::msg::ExecMsg;

use std::thread::JoinHandle;

/// Spawn the HTTP tests execution in a background thread and send ExecMsg updates
pub fn spawn(
    tx: std::sync::mpsc::Sender<ExecMsg>,
    config_path: String,
    users_file: String,
    selected_environments: Vec<String>,
    selected_routes: Vec<String>,
    include_headers: bool,
    include_errors: bool,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                let _ = tx.send(ExecMsg::Failed(format!("Failed to create runtime: {}", e)));
                return;
            }
        };

        rt.block_on(async {
            let timeout_duration = std::time::Duration::from_secs(300);
            match tokio::time::timeout(
                timeout_duration,
                execute_http_tests_async(
                    tx.clone(),
                    config_path,
                    users_file,
                    selected_environments,
                    selected_routes,
                    include_headers,
                    include_errors,
                ),
            )
            .await
            {
                Ok(_) => {}
                Err(_) => {
                    let _ = tx.send(ExecMsg::Failed(
                        "Execution timed out after 5 minutes".to_string(),
                    ));
                }
            }
        });
    })
}

/// Execute HTTP tests asynchronously and send progress updates
pub async fn execute_http_tests_async(
    tx: std::sync::mpsc::Sender<ExecMsg>,
    config_path: String,
    users_file: String,
    selected_environments: Vec<String>,
    selected_routes: Vec<String>,
    _include_headers: bool,
    _include_errors: bool,
) {
    use crate::{
        config::{load_user_data, HttpDiffConfig},
        create_default_test_runner, ProgressCallback, TestRunner,
    };

    // Send initial progress with empty tracker
    let _ = tx.send(ExecMsg::Progress {
        tracker: crate::execution::progress::ProgressTracker::new(0),
        op: "Loading configuration...".to_string(),
    });

    // Load configuration
    let config_path = std::path::Path::new(&config_path);
    let config = match HttpDiffConfig::load_from_file(config_path) {
        Ok(config) => config,
        Err(e) => {
            let _ = tx.send(ExecMsg::Failed(format!(
                "Failed to load configuration: {}",
                e
            )));
            return;
        }
    };

    // Load user data
    let users_path = std::path::Path::new(&users_file);
    let user_data = match load_user_data(users_path) {
        Ok(data) => data,
        Err(e) => {
            let _ = tx.send(ExecMsg::Failed(format!("Failed to load user data: {}", e)));
            return;
        }
    };

    // Send progress update
    let _ = tx.send(ExecMsg::Progress {
        tracker: crate::execution::progress::ProgressTracker::new(0),
        op: "Creating test runner...".to_string(),
    });

    // Create test runner
    let runner = match create_default_test_runner(config) {
        Ok(runner) => runner,
        Err(e) => {
            let _ = tx.send(ExecMsg::Failed(format!(
                "Failed to create test runner: {}",
                e
            )));
            return;
        }
    };

    // Set up progress callback using ProgressTracker as single source of truth
    let tx_clone = tx.clone();
    let last_state =
        std::sync::Arc::new(std::sync::Mutex::new((std::time::Instant::now(), 0usize)));

    let progress_callback: ProgressCallback = Box::new(move |progress_tracker| {
        // Rate limit progress updates to prevent UI spam (max every 50ms)
        let now = std::time::Instant::now();
        let should_update = if let Ok(mut state) = last_state.lock() {
            let (last_update, last_completed) = &mut *state;
            let update_needed = progress_tracker.completed_requests != *last_completed
                || now.duration_since(*last_update).as_millis() >= 50;

            if update_needed {
                *last_update = now;
                *last_completed = progress_tracker.completed_requests;
            }
            update_needed
        } else {
            true // If lock fails, send update anyway
        };

        if should_update {
            let operation = format!(
                "Completed {}/{} requests ({} successful, {} failed)",
                progress_tracker.completed_requests,
                progress_tracker.total_requests,
                progress_tracker.successful_requests,
                progress_tracker.failed_requests
            );

            let _ = tx_clone.send(ExecMsg::Progress {
                tracker: progress_tracker.clone(),
                op: operation,
            });
        }
    });

    // Calculate total requests and send initial progress with correct total
    let total_requests = selected_environments.len() * selected_routes.len() * user_data.len();
    let initial_tracker = crate::execution::progress::ProgressTracker::new(total_requests);

    let _ = tx.send(ExecMsg::Progress {
        tracker: initial_tracker,
        op: format!("Starting {} HTTP tests...", total_requests),
    });

    match runner
        .execute_with_data(
            &user_data,
            Some(selected_environments),
            Some(selected_routes),
            Some(progress_callback),
        )
        .await
    {
        Ok(execution_result) => {
            let _ = tx.send(ExecMsg::Completed(execution_result.comparisons));
        }
        Err(e) => {
            let _ = tx.send(ExecMsg::Failed(format!("Test execution failed: {}", e)));
        }
    }
}
