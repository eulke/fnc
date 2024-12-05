use dialoguer::{theme::ColorfulTheme, Select};
use crate::cli::{DeployType, Version};

pub struct DeployOptions {
    pub deploy_type: DeployType,
    pub version: Version,
}

impl DeployOptions {
    pub fn prompt() -> Self {
        let theme = ColorfulTheme::default();

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

        // Select version type
        let versions = vec!["Patch", "Minor", "Major"];
        let version_idx = Select::with_theme(&theme)
            .with_prompt("Select version increment type")
            .items(&versions)
            .default(0)
            .interact()
            .unwrap();

        let version = match versions[version_idx] {
            "Patch" => Version::Patch,
            "Minor" => Version::Minor,
            "Major" => Version::Major,
            _ => unreachable!(),
        };

        Self {
            deploy_type,
            version,
        }
    }
}
