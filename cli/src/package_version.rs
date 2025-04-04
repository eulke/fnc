use crate::{
    error::{CliError, Result},
    ui,
};
use dialoguer::{Select, theme::ColorfulTheme};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use version::SemverVersion;

#[derive(Debug, Clone, PartialEq, Eq)]
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
        .map_err(|e| e.with_context("Failed to read workspaces from root package.json"))?;

    if verbose {
        println!("Found workspaces: {workspaces:?}");
    }

    let package_infos = find_all_packages(&working_dir, &workspaces, verbose)
        .map_err(|e| e.with_context("Failed to scan packages"))?;

    if package_infos.is_empty() {
        ui::warning_message("No packages found in the monorepo");
        return Ok(());
    }

    if verbose {
        println!("Found {} packages", package_infos.len());
        for pkg in &package_infos {
            println!(
                "  - {name} @ {version} ({path})",
                name = pkg.name,
                version = pkg.version,
                path = pkg.path.display()
            );
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
        println!("  Package: {pkg_name}");
        println!("  Versions found:");

        for (version, locations) in versions {
            println!("    - {version} (in {} locations)", locations.len());
            if verbose {
                for location in locations {
                    println!("      - {location}");
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
        return Err(CliError::PackageNotFound(dir.to_path_buf()));
    }

    let content = fs::read_to_string(&package_json_path)?;
    let root_pkg: Value = serde_json::from_str(&content)?;

    let workspaces = match &root_pkg["workspaces"] {
        Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        Value::Object(obj) => {
            if let Some(pkgs) = obj.get("packages").and_then(|p| p.as_array()) {
                pkgs.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            } else {
                return Err(CliError::Other(
                    "Unable to parse workspaces in package.json".to_string(),
                ));
            }
        }
        _ => {
            return Err(CliError::NoWorkspaces);
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
                }
                Err(e) => {
                    if verbose {
                        println!("Error processing entry: {e}");
                    }
                }
            }
        }
    }

    Ok(packages)
}

fn read_package_info(path: &Path, verbose: bool) -> Result<Option<PackageInfo>> {
    let content = fs::read_to_string(path).map_err(|e| {
        CliError::Io(e).with_context(format!("Failed to read package.json at {}", path.display()))
    })?;

    let pkg_data: Value = serde_json::from_str(&content).map_err(|e| {
        CliError::JsonParseError(e)
            .with_context(format!("Failed to parse JSON in {}", path.display()))
    })?;

    // Extract the package name
    let Some(Value::String(name)) = pkg_data.get("name") else {
        if verbose {
            println!("Skipping package at {} - no name field", path.display());
        }
        return Ok(None);
    };
    let name = name.clone();

    // Extract the package version
    let Some(Value::String(version_str)) = pkg_data.get("version") else {
        if verbose {
            println!(
                "Skipping package '{}' at {} - no version field",
                name,
                path.display()
            );
        }
        return Ok(None);
    };

    let version = SemverVersion::parse(version_str).map_err(|e| {
        CliError::SemverError(e)
            .with_context(format!("Invalid version '{version_str}' in package {name}"))
    })?;

    // Get parent directory as package directory
    let pkg_dir = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    // Read dependencies
    let mut dependencies = HashMap::new();

    // Helper closure to extract dependencies from a section
    let mut extract_deps = |section: &str| {
        if let Some(deps) = pkg_data.get(section).and_then(|d| d.as_object()) {
            for (dep_name, dep_version) in deps {
                if let Some(ver) = dep_version.as_str() {
                    dependencies.insert(dep_name.clone(), ver.to_string());
                }
            }
        }
    };

    // Extract all dependency types
    extract_deps("dependencies");
    extract_deps("devDependencies");
    extract_deps("peerDependencies");

    Ok(Some(PackageInfo {
        name,
        version,
        path: pkg_dir,
        dependencies,
    }))
}

type PackageVersionMap = HashMap<String, HashMap<String, Vec<String>>>;

fn find_version_inconsistencies(packages: &[PackageInfo]) -> PackageVersionMap {
    let mut inconsistencies = HashMap::new();
    let mut package_versions = HashMap::new();
    let mut all_versions = HashSet::new();

    for pkg in packages {
        package_versions
            .entry(pkg.name.clone())
            .or_insert_with(HashSet::new)
            .insert(pkg.version.to_string());

        all_versions.insert(pkg.version.to_string());
    }

    let sync_all_versions = all_versions.len() > 1;

    let packages_with_inconsistent_versions: Vec<String> = if sync_all_versions {
        packages.iter().map(|pkg| pkg.name.clone()).collect()
    } else {
        package_versions
            .iter()
            .filter(|(_, versions)| versions.len() > 1)
            .map(|(name, _)| name.clone())
            .collect()
    };

    if packages_with_inconsistent_versions.is_empty() && !sync_all_versions {
        // Check dependencies for inconsistencies with actual package versions
        return check_dependency_inconsistencies(packages, &package_versions);
    }

    // Second pass: build inconsistency map
    for pkg in packages {
        if packages_with_inconsistent_versions.contains(&pkg.name) {
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
        match inconsistencies.entry(pkg_name) {
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(versions);
            }
            std::collections::hash_map::Entry::Occupied(mut e) => {
                let existing_versions = e.get_mut();
                for (version, locations) in versions {
                    let existing_locations = existing_versions.entry(version).or_default();

                    existing_locations.extend(locations);
                }
            }
        }
    }

    inconsistencies
}

fn clean_version_req(version_req: &str) -> String {
    let version = version_req.trim().trim_start_matches(['^', '~', '=', '>']);

    // For version ranges, just take the first part
    version
        .split_once(' ')
        .map_or(version, |(first, _)| first)
        .to_string()
}

fn check_dependency_inconsistencies(
    packages: &[PackageInfo],
    _package_versions: &HashMap<String, HashSet<String>>,
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
                SemverVersion::parse(actual_version),
            ) {
                let is_inconsistent = if cleaned_semver.major != actual_semver.major
                    || cleaned_semver.minor != actual_semver.minor
                    || cleaned_semver.patch != actual_semver.patch
                {
                    true
                } else {
                    match (
                        &cleaned_version.contains('-'),
                        &actual_version.contains('-'),
                    ) {
                        (true, true) => cleaned_version != *actual_version,
                        _ => false,
                    }
                };

                if is_inconsistent {
                    // Create map entry for package's inconsistent versions
                    let version_map = inconsistencies.entry(dep_name.clone()).or_default();

                    // Add actual version location
                    let actual_locations = version_map.entry(actual_version.clone()).or_default();

                    if !actual_locations
                        .iter()
                        .any(|loc| loc.ends_with("(package)"))
                    {
                        actual_locations.push("actual package version".to_string());
                    }

                    // Add dependency version location
                    let dep_locations = version_map.entry(cleaned_version.clone()).or_default();

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

    // First, collect all unique versions across all packages
    let mut all_versions: HashMap<String, Vec<String>> = HashMap::new();
    for pkg_name in inconsistencies.keys() {
        let version_map = inconsistencies.get(pkg_name).unwrap();

        for version in version_map.keys() {
            let entry = all_versions.entry(version.clone()).or_default();
            entry.push(pkg_name.clone());
        }
    }

    // If we have more than one unique version, ask user to select one for all packages
    if all_versions.len() > 1 {
        println!(
            "\nMultiple versions found across packages. Choose one version to apply to all packages:"
        );

        let mut version_list: Vec<(String, usize, Vec<String>)> = all_versions
            .iter()
            .map(|(version, pkgs)| (version.clone(), pkgs.len(), pkgs.clone()))
            .collect();

        // Sort by frequency (most common first)
        version_list.sort_by(|a, b| b.1.cmp(&a.1));

        let mut select_items = Vec::new();
        for (version, count, pkgs) in &version_list {
            if count > &3 {
                select_items.push(format!("{version} (used in {count} packages)"));
            } else {
                select_items.push(format!("{} (used in {})", version, pkgs.join(", ")));
            }
        }

        select_items.push("Skip global version selection (proceed package by package)".to_string());

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a version to apply to all packages")
            .default(0)
            .items(&select_items)
            .interact()?;

        // Apply the selected version to all packages
        if selection < version_list.len() {
            let selected_version = &version_list[selection].0;
            println!("Applying version {selected_version} to all packages with inconsistencies.");

            // Apply the selected version to all packages
            for pkg_name in inconsistencies.keys() {
                fixes.insert(pkg_name.clone(), selected_version.clone());
            }

            return Ok(fixes);
        }
    }

    // If user skipped global selection or there's only one unique version,
    // proceed with package-by-package selection
    for pkg_name in inconsistencies.keys() {
        println!("\nPackage: {pkg_name}");

        let versions = inconsistencies.get(pkg_name).unwrap();
        let mut version_list: Vec<(String, usize)> = versions
            .iter()
            .map(|(version, locations)| (version.clone(), locations.len()))
            .collect();

        // Sort by number of locations (most frequent first)
        version_list.sort_by(|a, b| b.1.cmp(&a.1));

        let mut select_items = Vec::new();
        for (version, count) in &version_list {
            select_items.push(format!("{version} (used in {count} locations)"));
        }

        select_items.push("Skip this package".to_string());

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Choose the correct version for {pkg_name}"))
            .default(0)
            .items(&select_items)
            .interact()?;

        if selection < version_list.len() {
            let selected_version = &version_list[selection].0;
            fixes.insert(pkg_name.clone(), selected_version.clone());

            println!("Will use version {selected_version} for {pkg_name}");
        } else {
            println!("Skipping {pkg_name}");
        }
    }

    Ok(fixes)
}

fn fix_version_inconsistencies(
    packages: &[PackageInfo],
    fixes: &HashMap<String, String>,
    verbose: bool,
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
        for dep_name in pkg.dependencies.keys() {
            if let Some(target_version) = fixes.get(dep_name) {
                if verbose {
                    println!(
                        "Updating dependency {} in {} to version {}",
                        dep_name,
                        pkg.path.display(),
                        target_version
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
    let updated_content = version_regex
        .replace(&content, |caps: &regex::Captures| {
            format!(
                "{}\"{}\"{}",
                &caps[1], // The prefix including whitespace and "version":
                version,  // The new version
                &caps[3]  // The trailing comma if present
            )
        })
        .to_string();

    if content != updated_content {
        fs::write(package_json_path, updated_content)?;
    }

    Ok(())
}

fn update_package_dependency(
    package_json_path: &Path,
    dep_name: &str,
    version: &str,
) -> Result<()> {
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
    let prefix = dep_prefix_regex
        .captures(&content)
        .map_or("", |caps| caps.get(1).map_or("", |m| m.as_str()));

    let updated_content = dep_regex
        .replace_all(&content, |caps: &regex::Captures| {
            format!(
                "{}\"{}{}\"{}",
                &caps[1], // The prefix including whitespace, dep name and colon
                prefix,   // The version prefix (^, ~, etc)
                version,  // The new version
                &caps[2]  // The trailing comma if present
            )
        })
        .to_string();

    if content != updated_content {
        fs::write(package_json_path, updated_content)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_version_req() {
        assert_eq!(clean_version_req("^1.2.3"), "1.2.3");
        assert_eq!(clean_version_req("~1.2.3"), "1.2.3");
        assert_eq!(clean_version_req(">=1.2.3"), "1.2.3");
        assert_eq!(clean_version_req("=1.2.3"), "1.2.3");
        assert_eq!(clean_version_req("1.2.3"), "1.2.3");
        assert_eq!(clean_version_req("^1.2.3 || ^2.0.0"), "1.2.3");
        assert_eq!(clean_version_req(">1.0.0 <2.0.0"), "1.0.0");
        assert_eq!(clean_version_req("  ^1.2.3  "), "1.2.3");
    }
}
