use crate::cli::Version;

pub fn increment(semver: &str, version: &Version) -> String {
    let mut parsed_version = semver::Version::parse(semver).expect("Failed to parse version");

    match version {
        Version::Major => {
            parsed_version.major += 1;
            parsed_version.minor = 0;
            parsed_version.patch = 0;
        }
        Version::Minor => {
            parsed_version.minor += 1;
            parsed_version.patch = 0;
        }
        Version::Patch => {
            parsed_version.patch += 1;
        }
    }

    parsed_version.to_string()
}
