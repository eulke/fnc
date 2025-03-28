use crate::ui;
use anyhow::{Context, Result, anyhow};
use version::SemverVersion;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct PackageInfo {
    name: String,
    version: SemverVersion,
    path: PathBuf,
    dependencies: HashMap<String, String>,
}

pub fn execute(dir: Option<String>, verbose: bool) -> Result<()> {
    let working_dir = match dir {
        Some(d) => PathBuf::from(d),
        None => std::env::current_dir()?,
    };

    if verbose {
        println!("Working directory: {}", working_dir.display());
    }

    ui::status_message("Analyzing monorepo package versions...");
    
    let workspaces = get_workspaces(&working_dir)
        .with_context(|| "Failed to read workspaces from root package.json")?;
    
    if verbose {
        println!("Found workspaces: {:?}", workspaces);
    }
    
    let package_infos = find_all_packages(&working_dir, &workspaces, verbose)
        .with_context(|| "Failed to scan packages")?;
    
    if package_infos.is_empty() {
        ui::warning_message("No packages found in the monorepo");
        return Ok(());
    }
    
    if verbose {
        println!("Found {} packages", package_infos.len());
        for pkg in &package_infos {
            println!("  - {} @ {} ({})", pkg.name, pkg.version, pkg.path.display());
        }
    }
    
    let inconsistencies = find_version_inconsistencies(&package_infos);
    
    if inconsistencies.is_empty() {
        ui::success_message("All package versions are consistent!");
        return Ok(());
    }
    
    // Display inconsistencies
    ui::warning_message(&format!(
        "Found {} packages with version inconsistencies:", 
        inconsistencies.len()
    ));
    
    for (pkg_name, versions) in &inconsistencies {
        println!("  Package: {}", pkg_name);
        println!("  Versions found:");
        
        for (version, locations) in versions {
            println!("    - {} (in {} locations)", version, locations.len());
            if verbose {
                for location in locations {
                    println!("      - {}", location);
                }
            }
        }
        println!();
    }
    
    // Ask the user which version to use for each inconsistent package
    let fixes = ask_user_for_version_fixes(&inconsistencies)?;
    
    if fixes.is_empty() {
        ui::warning_message("No fixes selected. Exiting without changes.");
        return Ok(());
    }
    
    // Fix the inconsistencies
    fix_version_inconsistencies(&package_infos, &fixes, verbose)?;
    
    ui::success_message("Package versions have been successfully synchronized!");
    
    Ok(())
}

fn get_workspaces(dir: &Path) -> Result<Vec<String>> {
    let package_json_path = dir.join("package.json");
    if !package_json_path.exists() {
        return Err(anyhow!("No package.json found in {}", dir.display()));
    }
    
    let content = fs::read_to_string(&package_json_path)?;
    let root_pkg: Value = serde_json::from_str(&content)?;
    
    let workspaces = match &root_pkg["workspaces"] {
        Value::Array(arr) => {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        },
        Value::Object(obj) => {
            if let Some(pkgs) = obj.get("packages").and_then(|p| p.as_array()) {
                pkgs.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            } else {
                return Err(anyhow!("Unable to parse workspaces in package.json"));
            }
        },
        _ => {
            return Err(anyhow!("No workspaces found in package.json"));
        }
    };
    
    Ok(workspaces)
}

fn find_all_packages(dir: &Path, workspaces: &[String], verbose: bool) -> Result<Vec<PackageInfo>> {
    let mut packages = Vec::new();
    let mut glob_patterns = Vec::new();
    
    // Convert workspace globs to patterns
    for workspace in workspaces {
        let glob_pattern = format!("{}/{}/package.json", dir.display(), workspace);
        glob_patterns.push(glob_pattern);
    }
    
    // Use glob to find all package.json files
    for pattern in glob_patterns {
        for entry in glob::glob(&pattern)? {
            match entry {
                Ok(path) => {
                    if let Some(pkg_info) = read_package_info(&path, verbose)? {
                        packages.push(pkg_info);
                    }
                },
                Err(e) => {
                    if verbose {
                        println!("Error processing entry: {}", e);
                    }
                }
            }
        }
    }
    
    Ok(packages)
}

fn read_package_info(path: &Path, verbose: bool) -> Result<Option<PackageInfo>> {
    let content = fs::read_to_string(path)?;
    let pkg_data: Value = serde_json::from_str(&content)?;
    
    let name = match pkg_data.get("name") {
        Some(Value::String(name)) => name.clone(),
        _ => {
            if verbose {
                println!("Skipping package at {} - no name field", path.display());
            }
            return Ok(None);
        }
    };
    
    let version_str = match pkg_data.get("version") {
        Some(Value::String(version)) => version,
        _ => {
            if verbose {
                println!("Skipping package '{}' at {} - no version field", name, path.display());
            }
            return Ok(None);
        }
    };
    
    let version = match SemverVersion::parse(version_str) {
        Ok(v) => v,
        Err(e) => {
            if verbose {
                println!(
                    "Skipping package '{}' at {} - invalid version '{}': {}", 
                    name, path.display(), version_str, e
                );
            }
            return Ok(None);
        }
    };
    
    // Extract dependencies
    let mut dependencies = HashMap::new();
    
    // Check regular dependencies
    if let Some(deps) = pkg_data.get("dependencies").and_then(|d| d.as_object()) {
        for (dep_name, dep_version) in deps {
            if let Some(Value::String(ver)) = dep_version.as_str().map(|s| Value::String(s.to_string())) {
                dependencies.insert(dep_name.clone(), ver);
            }
        }
    }
    
    // Check dev dependencies
    if let Some(deps) = pkg_data.get("devDependencies").and_then(|d| d.as_object()) {
        for (dep_name, dep_version) in deps {
            if let Some(Value::String(ver)) = dep_version.as_str().map(|s| Value::String(s.to_string())) {
                dependencies.insert(dep_name.clone(), ver);
            }
        }
    }
    
    // Check peer dependencies
    if let Some(deps) = pkg_data.get("peerDependencies").and_then(|d| d.as_object()) {
        for (dep_name, dep_version) in deps {
            if let Some(Value::String(ver)) = dep_version.as_str().map(|s| Value::String(s.to_string())) {
                dependencies.insert(dep_name.clone(), ver);
            }
        }
    }
    
    let pkg_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    
    Ok(Some(PackageInfo {
        name,
        version,
        path: pkg_dir,
        dependencies,
    }))
}

type PackageVersionMap = HashMap<String, HashMap<String, Vec<String>>>;

fn find_version_inconsistencies(packages: &[PackageInfo]) -> PackageVersionMap {
    let mut inconsistencies: PackageVersionMap = HashMap::new();
    let mut package_versions: HashMap<String, HashSet<String>> = HashMap::new();
    
    // First pass: collect all versions for each package
    for pkg in packages {
        let entry = package_versions
            .entry(pkg.name.clone())
            .or_insert_with(HashSet::new);
        
        entry.insert(pkg.version.to_string());
    }
    
    // Filter out packages with only one version
    let packages_with_multiple_versions: HashSet<String> = package_versions
        .iter()
        .filter(|(_, versions)| versions.len() > 1)
        .map(|(name, _)| name.clone())
        .collect();
    
    if packages_with_multiple_versions.is_empty() {
        // Check dependencies for inconsistencies with actual package versions
        return check_dependency_inconsistencies(packages, &package_versions);
    }
    
    // Second pass: build inconsistency map for packages with multiple versions
    for pkg in packages {
        if packages_with_multiple_versions.contains(&pkg.name) {
            let version_map = inconsistencies
                .entry(pkg.name.clone())
                .or_insert_with(HashMap::new);
            
            let locations = version_map
                .entry(pkg.version.to_string())
                .or_insert_with(Vec::new);
            
            locations.push(format!("{} (package)", pkg.path.display()));
        }
    }
    
    // Check dependencies for inconsistencies
    let dep_inconsistencies = check_dependency_inconsistencies(packages, &package_versions);
    
    // Merge the two inconsistency maps
    for (pkg_name, versions) in dep_inconsistencies {
        if inconsistencies.contains_key(&pkg_name) {
            let existing_versions = inconsistencies.get_mut(&pkg_name).unwrap();
            for (version, locations) in versions {
                let existing_locations = existing_versions
                    .entry(version)
                    .or_insert_with(Vec::new);
                
                existing_locations.extend(locations);
            }
        } else {
            inconsistencies.insert(pkg_name, versions);
        }
    }
    
    inconsistencies
}

fn check_dependency_inconsistencies(
    packages: &[PackageInfo],
    _package_versions: &HashMap<String, HashSet<String>>
) -> PackageVersionMap {
    let mut inconsistencies: PackageVersionMap = HashMap::new();
    
    // Build a map of package name -> actual version
    let mut actual_versions: HashMap<String, String> = HashMap::new();
    for pkg in packages {
        actual_versions.insert(pkg.name.clone(), pkg.version.to_string());
    }
    
    // Check dependencies for inconsistencies
    for pkg in packages {
        for (dep_name, dep_version_req) in &pkg.dependencies {
            // Only check internal dependencies (packages in the monorepo)
            if !actual_versions.contains_key(dep_name) {
                continue;
            }
            
            // Clean the version requirement (remove ^, ~, etc)
            let cleaned_version = clean_version_req(dep_version_req);
            let actual_version = &actual_versions[dep_name];
            
            if cleaned_version != *actual_version {
                let version_map = inconsistencies
                    .entry(dep_name.clone())
                    .or_insert_with(HashMap::new);
                
                // Add the actual version
                let actual_locations = version_map
                    .entry(actual_version.clone())
                    .or_insert_with(Vec::new);
                
                if !actual_locations.iter().any(|loc| loc.ends_with("(package)")) {
                    actual_locations.push("actual package version".to_string());
                }
                
                // Add the dependency version
                let dep_locations = version_map
                    .entry(cleaned_version.clone())
                    .or_insert_with(Vec::new);
                
                dep_locations.push(format!(
                    "{} (dependency in {})", 
                    dep_version_req, 
                    pkg.path.display()
                ));
            }
        }
    }
    
    inconsistencies
}

fn clean_version_req(version_req: &str) -> String {
    let version = version_req.trim();
    
    // Remove semver operators and whitespace
    let version = version
        .trim_start_matches('^')
        .trim_start_matches('~')
        .trim_start_matches('=')
        .trim_start_matches('>');
    
    // For version ranges, just take the first part
    if let Some(idx) = version.find(|c| c == ' ' || c == '-') {
        return version[0..idx].to_string();
    }
    
    version.to_string()
}

fn ask_user_for_version_fixes(
    inconsistencies: &PackageVersionMap,
) -> Result<HashMap<String, String>> {
    let mut fixes = HashMap::new();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let stdin = io::stdin();
    let mut buffer = String::new();
    
    for (pkg_name, versions) in inconsistencies {
        println!("\nPackage: {}", pkg_name);
        println!("Choose the correct version:");
        
        let mut version_list: Vec<(String, usize)> = versions
            .iter()
            .map(|(version, locations)| (version.clone(), locations.len()))
            .collect();
        
        // Sort by number of locations (most frequent first)
        version_list.sort_by(|a, b| b.1.cmp(&a.1));
        
        for (idx, (version, count)) in version_list.iter().enumerate() {
            println!("  {}. {} (used in {} locations)", idx + 1, version, count);
        }
        
        println!("  0. Skip this package");
        
        write!(stdout, "Enter choice [1-{}, default=1]: ", version_list.len())?;
        stdout.flush()?;
        
        buffer.clear();
        stdin.read_line(&mut buffer)?;
        let choice = buffer.trim();
        
        let idx = if choice.is_empty() {
            0 // Default to the first option
        } else {
            match choice.parse::<usize>() {
                Ok(num) if num == 0 => {
                    println!("Skipping {}", pkg_name);
                    continue;
                }
                Ok(num) if num <= version_list.len() => num - 1,
                _ => {
                    println!("Invalid choice, using default (1)");
                    0
                }
            }
        };
        
        let selected_version = &version_list[idx].0;
        fixes.insert(pkg_name.clone(), selected_version.clone());
        
        println!("Will use version {} for {}", selected_version, pkg_name);
    }
    
    Ok(fixes)
}

fn fix_version_inconsistencies(
    packages: &[PackageInfo], 
    fixes: &HashMap<String, String>,
    verbose: bool
) -> Result<()> {
    // Track which packages have been updated
    let mut updated_packages = HashSet::new();
    
    // First update actual package versions
    for pkg in packages {
        if let Some(target_version) = fixes.get(&pkg.name) {
            let target_semver = SemverVersion::parse(target_version)?;
            
            if pkg.version != target_semver {
                if verbose {
                    println!(
                        "Updating package {} from version {} to {}",
                        pkg.name, pkg.version, target_version
                    );
                }
                
                let package_json_path = pkg.path.join("package.json");
                update_package_version(&package_json_path, target_version)?;
                
                updated_packages.insert(pkg.name.clone());
            }
        }
    }
    
    // Then update all dependencies
    for pkg in packages {
        let package_json_path = pkg.path.join("package.json");
        let mut any_dep_updated = false;
        
        // Check if we need to update any dependencies
        for (dep_name, _) in &pkg.dependencies {
            if let Some(target_version) = fixes.get(dep_name) {
                if verbose {
                    println!(
                        "Updating dependency {} in {} to version {}", 
                        dep_name, pkg.path.display(), target_version
                    );
                }
                
                update_package_dependency(&package_json_path, dep_name, target_version)?;
                any_dep_updated = true;
            }
        }
        
        if any_dep_updated && !updated_packages.contains(&pkg.name) {
            updated_packages.insert(pkg.name.clone());
        }
    }
    
    if verbose {
        println!("Updated {} packages", updated_packages.len());
    }
    
    Ok(())
}

fn update_package_version(package_json_path: &Path, version: &str) -> Result<()> {
    let content = fs::read_to_string(package_json_path)?;
    let mut package_json: Value = serde_json::from_str(&content)?;
    
    if let Some(obj) = package_json.as_object_mut() {
        obj.insert("version".to_string(), json!(version));
    }
    
    let updated_content = serde_json::to_string_pretty(&package_json)?;
    fs::write(package_json_path, updated_content)?;
    
    Ok(())
}

fn update_package_dependency(package_json_path: &Path, dep_name: &str, version: &str) -> Result<()> {
    let content = fs::read_to_string(package_json_path)?;
    let mut package_json: Value = serde_json::from_str(&content)?;
    
    // Function to update a specific dependency section
    let update_dep_section = |section: &mut Value, dep_name: &str, version: &str| {
        if let Some(deps) = section.as_object_mut() {
            if deps.contains_key(dep_name) {
                let current_ver = &deps[dep_name];
                if let Some(current_str) = current_ver.as_str() {
                    // Preserve version prefix (^, ~, etc)
                    let prefix = if current_str.starts_with('^') {
                        "^"
                    } else if current_str.starts_with('~') {
                        "~"
                    } else {
                        ""
                    };
                    
                    deps.insert(dep_name.to_string(), json!(format!("{}{}", prefix, version)));
                }
            }
        }
    };
    
    // Update in dependencies
    if let Some(deps) = package_json.get_mut("dependencies") {
        update_dep_section(deps, dep_name, version);
    }
    
    // Update in devDependencies
    if let Some(deps) = package_json.get_mut("devDependencies") {
        update_dep_section(deps, dep_name, version);
    }
    
    // Update in peerDependencies
    if let Some(deps) = package_json.get_mut("peerDependencies") {
        update_dep_section(deps, dep_name, version);
    }
    
    let updated_content = serde_json::to_string_pretty(&package_json)?;
    fs::write(package_json_path, updated_content)?;
    
    Ok(())
}