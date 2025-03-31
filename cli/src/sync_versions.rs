use crate::ui;
use crate::error::Result;
use std::path::{Path, PathBuf};
use version::{Version, SemverVersion};

/// Executes the sync-versions command
pub fn execute(
    source: String,
    targets: Vec<String>,
    discover: bool,
    max_depth: usize,
    verbose: bool,
) -> Result<()> {
    let source_path = PathBuf::from(source);
    
    // Read version from the source project
    ui::status_message(&format!("Reading version from source: {}", source_path.display()));
    let source_version = Version::read_from_project(&source_path)?;
    ui::success_message(&format!("Source version: {}", source_version));
    
    // Convert target strings to paths
    let mut target_paths: Vec<PathBuf> = targets.into_iter().map(PathBuf::from).collect();
    
    // Auto-discover projects if requested
    if discover {
        ui::status_message("Auto-discovering projects...");
        
        // Determine the root directory for discovery
        let discovery_root = if target_paths.is_empty() {
            // If no targets specified, use parent of source
            source_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf()
        } else {
            // If targets specified, use the first one as the root
            target_paths[0].clone()
        };
        
        if verbose {
            println!("Discovering projects under: {}", discovery_root.display());
        }
        
        // Discover projects
        let discovered = discover_projects(&discovery_root, max_depth, verbose)?;
        
        // Add discovered projects to targets, excluding the source
        for path in discovered {
            if path != source_path {
                target_paths.push(path);
            }
        }
        
        ui::success_message(&format!("Discovered {} projects", target_paths.len()));
    }
    
    if target_paths.is_empty() {
        ui::warning_message("No target projects specified or discovered");
        return Ok(());
    }
    
    // Display target projects
    if verbose {
        println!("Target projects:");
        for path in &target_paths {
            println!("  - {}", path.display());
        }
    }
    
    // Apply version to all targets
    ui::status_message(&format!("Applying version {} to {} projects", source_version, target_paths.len()));
    
    let mut successful_updates = 0;
    let mut failed_updates = 0;
    
    for target in &target_paths {
        if verbose {
            println!("Updating: {}", target.display());
        }
        
        match update_project_version(target, &source_version, verbose) {
            Ok(_) => {
                successful_updates += 1;
            },
            Err(e) => {
                ui::error_message(&format!("Failed to update {}: {}", target.display(), e));
                failed_updates += 1;
            }
        }
    }
    
    // Show summary
    println!();
    ui::success_message(&format!("Updated {} projects successfully", successful_updates));
    if failed_updates > 0 {
        ui::warning_message(&format!("Failed to update {} projects", failed_updates));
    }
    
    Ok(())
}

/// Update a single project's version
fn update_project_version(project_path: &Path, version: &SemverVersion, verbose: bool) -> Result<()> {
    // Detect ecosystem to give more specific messages
    let ecosystem = Version::detect_ecosystem(project_path)?;
    
    if verbose {
        println!("Detected {} project at {}", ecosystem, project_path.display());
    }
    
    // Write version to project
    Version::write_to_project(project_path, version)?;
    
    if verbose {
        println!("Successfully updated {} project at {}", ecosystem, project_path.display());
    }
    
    Ok(())
}

/// Discover projects in subdirectories
fn discover_projects(root_dir: &Path, max_depth: usize, verbose: bool) -> Result<Vec<PathBuf>> {
    if verbose {
        println!("Discovering projects with max depth: {}", max_depth);
    }
    
    // Use Version::discover_projects under the hood
    let projects = Version::discover_projects(root_dir)?;
    
    // Extract just the paths
    let paths: Vec<PathBuf> = projects.into_iter()
        .map(|(path, ecosystem)| {
            if verbose {
                println!("Found {} project at {}", ecosystem, path.display());
            }
            path
        })
        .collect();
    
    Ok(paths)
}