use crate::ui;
use anyhow::{Context, Result, anyhow};
use version::SemverVersion;
use serde_json::Value;
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

fn clean_version_req(version_req: &str) -> String {
    let version = version_req.trim();
    
    // Remove semver operators and whitespace
    let version = version
        .trim_start_matches('^')
        .trim_start_matches('~')
        .trim_start_matches('=')
        .trim_start_matches('>');
    
    // For version ranges, just take the first part
    if let Some(_idx) = version.find(|c| c == ' ' || c == '-') {
        // Only split on space for ranges, preserve prerelease identifiers
        if version.contains(' ') {
            if let Some(space_idx) = version.find(' ') {
                return version[0..space_idx].to_string();
            }
        }
    }
    
    version.to_string()
}

fn check_dependency_inconsistencies(
    packages: &[PackageInfo],
    _package_versions: &HashMap<String, HashSet<String>>
) -> PackageVersionMap {
    let mut inconsistencies: PackageVersionMap = HashMap::new();
    
    let mut actual_versions: HashMap<String, String> = HashMap::new();
    for pkg in packages {
        actual_versions.insert(pkg.name.clone(), pkg.version.to_string());
    }
    
    for pkg in packages {
        for (dep_name, dep_version_req) in &pkg.dependencies {
            if !actual_versions.contains_key(dep_name) {
                continue;
            }
            
            let cleaned_version = clean_version_req(dep_version_req);
            let actual_version = &actual_versions[dep_name];
            
            if let (Ok(cleaned_semver), Ok(actual_semver)) = (
                SemverVersion::parse(&cleaned_version),
                SemverVersion::parse(actual_version)
            ) {
                let is_inconsistent = if cleaned_semver.major != actual_semver.major ||
                                         cleaned_semver.minor != actual_semver.minor ||
                                         cleaned_semver.patch != actual_semver.patch {
                    true
                } else {
                    let has_different_prereleases = match (&cleaned_version.contains('-'), &actual_version.contains('-')) {
                        (true, true) => cleaned_version != *actual_version,
                        _ => false
                    };
                    
                    has_different_prereleases
                };
                
                if is_inconsistent {
                    let version_map = inconsistencies
                        .entry(dep_name.clone())
                        .or_insert_with(HashMap::new);
                    
                    let actual_locations = version_map
                        .entry(actual_version.clone())
                        .or_insert_with(Vec::new);
                    
                    if !actual_locations.iter().any(|loc| loc.ends_with("(package)")) {
                        actual_locations.push("actual package version".to_string());
                    }
                    
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
    }
    
    inconsistencies
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
    
    // Use regex to find and replace only the version line
    let version_regex = regex::Regex::new(r#"(\s*"version"\s*:\s*)"([^"]+)"(,?)"#)?;
    let updated_content = version_regex.replace(&content, |caps: &regex::Captures| {
        format!("{}\"{}\"{}",
            &caps[1],  // The prefix including whitespace and "version":
            version,   // The new version
            &caps[3]   // The trailing comma if present
        )
    }).to_string();
    
    if content != updated_content {
        fs::write(package_json_path, updated_content)?;
    }
    
    Ok(())
}

fn update_package_dependency(package_json_path: &Path, dep_name: &str, version: &str) -> Result<()> {
    let content = fs::read_to_string(package_json_path)?;
    
    // Create regex that only matches the specific dependency
    // This preserves all formatting and just changes the version number
    let dep_regex = regex::Regex::new(&format!(
        r#"(\s*"{}"?\s*:\s*)"[^"]+"(,?)"#,
        regex::escape(dep_name)
    ))?;
    
    // Find prefix character in original content to preserve it
    let dep_prefix_regex = regex::Regex::new(&format!(
        r#""{}"?\s*:\s*"([~^=]?)"#,
        regex::escape(dep_name)
    ))?;
    
    // Extract prefix if it exists
    let prefix = dep_prefix_regex.captures(&content)
        .map(|caps| caps.get(1).map_or("", |m| m.as_str()))
        .unwrap_or("");
    
    let updated_content = dep_regex.replace_all(&content, |caps: &regex::Captures| {
        format!("{}\"{}{}\"{}",
            &caps[1],  // The prefix including whitespace, dep name and colon
            prefix,    // The version prefix (^, ~, etc)
            version,   // The new version
            &caps[2]   // The trailing comma if present
        )
    }).to_string();
    
    if content != updated_content {
        fs::write(package_json_path, updated_content)?;
    }
    
    Ok(())
}