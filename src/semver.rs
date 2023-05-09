use semver::Version;

pub fn increment(semver: &str, version: &str) -> String {
    let mut parsed_version = Version::parse(semver).expect("Failed to parse version");

    match version {
        "major" => {
            parsed_version.major += 1;
            parsed_version.minor = 0;
            parsed_version.patch = 0;
        }
        "minor" => {
            parsed_version.minor += 1;
            parsed_version.patch = 0;
        }
        "patch" => {
            parsed_version.patch += 1;
        }
        _ => panic!("Invalid version level. Only 'major', 'minor' and 'patch' are allowed."),
    }

    parsed_version.to_string()
}
