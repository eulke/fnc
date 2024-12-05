use dialoguer::{theme::ColorfulTheme, Select};
use crate::cli::{DeployType, Version};
use crate::language::Language;
use crate::semver;

pub struct DeployOptions {
    pub deploy_type: DeployType,
    pub version: Version,
}

impl DeployOptions {
    pub fn prompt() -> Self {
        let theme = ColorfulTheme::default();
        let language = Language::detect().expect("Unable to detect language");
        let current_version = language.current_pkg_version();

        // Select deploy type
        let deploy_types = vec!["Release", "Hotfix"];
        let deploy_type_idx = Select::with_theme(&theme)
            .with_prompt("Select deployment type")
            .items(&deploy_types)
            .default(0)
            .interact()
            .unwrap();

        let deploy_type = match deploy_types[deploy_type_idx] {
            "Release" => DeployType::Release,
            "Hotfix" => DeployType::Hotfix,
            _ => unreachable!(),
        };

        // Calculate next versions
        let next_patch = semver::increment(&current_version, &Version::Patch);
        let next_minor = semver::increment(&current_version, &Version::Minor);
        let next_major = semver::increment(&current_version, &Version::Major);

        // Select version type with preview
        let versions = vec![
            format!("Patch ({})", next_patch),
            format!("Minor ({})", next_minor),
            format!("Major ({})", next_major),
        ];
        
        let version_idx = Select::with_theme(&theme)
            .with_prompt(format!("Select version increment type (current: {})", current_version))
            .items(&versions)
            .default(0)
            .interact()
            .unwrap();

        let version = match version_idx {
            0 => Version::Patch,
            1 => Version::Minor,
            2 => Version::Major,
            _ => unreachable!(),
        };

        Self {
            deploy_type,
            version,
        }
    }
}
