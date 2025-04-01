use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::io;
use std::os::unix::fs::PermissionsExt;

use crate::error::Result;
use crate::ui;

pub fn execute(force: bool, verbose: bool) -> Result<()> {
    if verbose {
        ui::info_message("Starting upgrade process for FNC CLI");
    }

    // Check if running on macOS
    if env::consts::OS != "macos" {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Upgrade command only supports macOS currently"
        ).into());
    }

    // Check if we're in a development environment
    if !force {
        let current_exe = env::current_exe()?;
        let home = env::var("HOME").map_err(|e| io::Error::new(
            io::ErrorKind::NotFound,
            format!("HOME environment variable not found: {}", e)
        ))?;
        let local_bin = PathBuf::from(home).join(".local/bin/fnc");
        
        // If the running binary is not from ~/.local/bin
        if current_exe != local_bin {
            if verbose {
                ui::warning_message(&format!(
                    "Running from development environment: {:?}\nUse --force to upgrade anyway.",
                    current_exe
                ));
            } else {
                ui::warning_message("Running from development environment. Use --force to upgrade anyway.");
            }
            return Ok(());
        }
    }

    // Get the installer script
    ui::status_message("Downloading installer script");
    let temp_dir = tempfile::tempdir()?;
    let script_path = temp_dir.path().join("install.sh");
    
    let output = Command::new("curl")
        .args([
            "-s", 
            "-L", 
            "-o", 
            script_path.to_str().unwrap(),
            "https://raw.githubusercontent.com/eulke/fnc/main/install.sh"
        ])
        .output()?;
    
    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Failed to download installer: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        ).into());
    }
    
    // Make script executable
    fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))?;
    
    // Run the installer script
    ui::status_message("Running installer");
    let status = Command::new(script_path)
        .spawn()?
        .wait()?;
    
    if status.success() {
        ui::success_message("FNC CLI has been successfully upgraded!");
    } else {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Upgrade failed. See error messages above."
        ).into());
    }

    Ok(())
}