use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;

use crate::{PackageOrExample, Target, TargetBuildSettings};
use camino::Utf8PathBuf;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct DexterousConfig {
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub asset_folders: Vec<camino::Utf8PathBuf>,
    #[serde(default)]
    pub code_watch_folders: Vec<camino::Utf8PathBuf>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub targets: HashMap<Target, ReloadTargetConfig>,
    #[serde(default)]
    pub packages: HashMap<String, ReloadTargetConfig>,
    #[serde(default)]
    pub examples: HashMap<String, ReloadTargetConfig>,
    #[serde(default)]
    pub default_package: Option<ReloadTargetConfig>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ReloadTargetConfig {
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub asset_folders: Vec<camino::Utf8PathBuf>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
}

impl DexterousConfig {
    pub async fn load_toml(path: &camino::Utf8Path) -> Result<Self, LoadConfigError> {
        let path = path.canonicalize_utf8()?;
        let path = if path.is_file() {
            path
        } else {
            path.join("Dexterous.toml")
        };

        if !path.exists() {
            info!("No config found at {path}, using a default config");
            return Ok(Default::default());
        }

        let file = tokio::fs::read_to_string(path).await?;

        let config = toml::from_str(&file)?;

        Ok(config)
    }

    pub fn load_toml_from_str(toml: &str) -> Result<Self, LoadConfigError> {
        let config = toml::from_str(toml)?;
        Ok(config)
    }
}

#[derive(Error, Debug)]
pub enum LoadConfigError {
    #[error("Couldn't read config file {0}")]
    IoError(#[from] std::io::Error),
    #[error("Couldn't parse config file {0}")]
    ParseError(#[from] toml::de::Error),
}

impl DexterousConfig {
    pub fn generate_build_settings(
        &self,
        package_or_example: Option<PackageOrExample>,
        features: &[String],
    ) -> Result<Vec<(Target, TargetBuildSettings)>, BuildSettingsGenerationError> {
        let package_or_example = package_or_example.unwrap_or_default();

        let package_specific_config = match &package_or_example {
            PackageOrExample::DefaulPackage => {
                self.default_package.as_ref().cloned().unwrap_or_default()
            }
            PackageOrExample::Package(package) => {
                self.packages.get(package).cloned().unwrap_or_default()
            }
            PackageOrExample::Example(example) => {
                self.examples.get(example).cloned().unwrap_or_default()
            }
        };

        let global_features = features
            .iter()
            .chain(package_specific_config.features.iter())
            .chain(self.features.iter())
            .map(|s| s.as_str())
            .collect::<Vec<_>>();
        let global_asset_folders = package_specific_config
            .asset_folders
            .iter()
            .chain(self.asset_folders.iter())
            .map(|s| s.as_str())
            .collect::<Vec<_>>();
        let global_environment_variables = package_specific_config
            .environment
            .iter()
            .chain(self.environment.iter())
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect::<HashMap<_, _>>();

        let mut targets = self
            .targets
            .iter()
            .map(|(target, settings)| {
                (
                    *target,
                    settings.features.clone(),
                    settings.asset_folders.clone(),
                    settings.environment.clone(),
                )
            })
            .collect::<Vec<_>>();

        if targets.is_empty() {
            let default_target =
                Target::current().ok_or(BuildSettingsGenerationError::NoDefaultTarget)?;
            targets.push((default_target, vec![], vec![], HashMap::new()))
        }

        Ok(targets
            .into_iter()
            .map(
                move |(target, mut features, mut asset_folders, mut environment)| {
                    for f in global_features.iter() {
                        features.push(f.to_string());
                    }
                    for a in global_asset_folders.iter() {
                        asset_folders.push(Utf8PathBuf::from(*a));
                    }
                    for (key, value) in global_environment_variables.iter() {
                        environment.insert(key.to_owned(), value.to_owned());
                    }
                    (
                        target,
                        TargetBuildSettings {
                            working_dir: Default::default(),
                            package_or_example: package_or_example.clone(),
                            features,
                            asset_folders,
                            code_watch_folders: self.code_watch_folders.clone(),
                            environment,
                        },
                    )
                },
            )
            .collect::<Vec<_>>())
    }
}

#[derive(Error, Debug)]
pub enum BuildSettingsGenerationError {
    #[error("No Default Target for this Platform")]
    NoDefaultTarget,
}

#[cfg(test)]
mod test {
    use crate::{PackageOrExample, Target};
    use camino::Utf8PathBuf;

    use super::{DexterousConfig, ReloadTargetConfig};

    #[test]
    fn given_a_manifest_with_no_metadata_provides_default_target() {
        let default_target = Target::current().expect("No default target for this platform");

        let toml = r#"
        "#;

        let config = DexterousConfig::load_toml_from_str(toml).expect("Couldn't load toml");
        let build_settings = config
            .generate_build_settings(None, &[])
            .expect("Couldn't generate build settings");

        assert_eq!(build_settings.len(), 1);

        let (target, settings) = build_settings.first().expect("No Targets Set Up");

        assert_eq!(target, &default_target);

        assert!(matches!(
            settings.package_or_example,
            PackageOrExample::DefaulPackage
        ));
        assert_eq!(settings.features.len(), 0);
        assert_eq!(settings.asset_folders.len(), 0);
    }

    #[test]
    fn given_a_manifest_with_a_target_provides_that_target() {
        let config = DexterousConfig {
            targets: ([(
                Target::Windows,
                ReloadTargetConfig {
                    features: vec!["my-feature".to_string()],
                    asset_folders: vec![Utf8PathBuf::from("/asset")],
                    environment: [("env".to_string(), "value".to_string())]
                        .into_iter()
                        .collect(),
                },
            )])
            .into_iter()
            .collect(),
            ..Default::default()
        };

        let build_settings = config
            .generate_build_settings(None, &[])
            .expect("Couldn't generate build settings");

        assert_eq!(build_settings.len(), 1);

        let (target, settings) = build_settings.first().expect("No Targets Set Up");

        assert_eq!(target, &Target::Windows);

        assert!(matches!(
            settings.package_or_example,
            PackageOrExample::DefaulPackage
        ));
        assert_eq!(settings.features.len(), 1);
        assert_eq!(settings.features.first().unwrap(), "my-feature");
        assert_eq!(settings.asset_folders.len(), 1);
        assert_eq!(
            settings.asset_folders.first().unwrap().to_string(),
            "/asset"
        );
        assert_eq!(settings.environment.get("env").unwrap(), "value");
    }

    #[test]
    fn given_a_manifest_with_features_and_assets_provides_them() {
        let default_target = Target::current().expect("No default target for this platform");

        let config = DexterousConfig {
            features: vec!["my-feature".to_string()],
            asset_folders: vec![Utf8PathBuf::from("/asset")],
            ..Default::default()
        };

        let build_settings = config
            .generate_build_settings(None, &[])
            .expect("Couldn't generate build settings");

        assert_eq!(build_settings.len(), 1);

        let (target, settings) = build_settings.first().expect("No Targets Set Up");

        assert_eq!(target, &default_target);

        assert!(matches!(
            settings.package_or_example,
            PackageOrExample::DefaulPackage
        ));
        assert_eq!(settings.features.len(), 1);
        assert_eq!(settings.features.first().unwrap(), "my-feature");
        assert_eq!(settings.asset_folders.len(), 1);
        assert_eq!(
            settings.asset_folders.first().unwrap().to_string(),
            "/asset"
        );
    }

    #[test]
    fn given_a_manifest_with_a_package_provides_the_correct_package() {
        let default_target = Target::current().expect("No default target for this platform");

        let config = DexterousConfig {
            packages: ([
                (
                    "My-Package".to_owned(),
                    ReloadTargetConfig {
                        features: vec!["my-feature".to_string()],
                        asset_folders: vec![Utf8PathBuf::from("/asset")],
                        ..Default::default()
                    },
                ),
                (
                    "My-Other-Package".to_owned(),
                    ReloadTargetConfig {
                        features: vec!["shouldnt-load".to_string()],
                        asset_folders: vec![],
                        ..Default::default()
                    },
                ),
            ])
            .into_iter()
            .collect(),
            ..Default::default()
        };

        let build_settings = config
            .generate_build_settings(
                Some(PackageOrExample::Package("My-Package".to_string())),
                &[],
            )
            .expect("Couldn't generate build settings");

        assert_eq!(build_settings.len(), 1);

        let (target, settings) = build_settings.first().expect("No Targets Set Up");

        assert_eq!(target, &default_target);

        assert!(
            if let PackageOrExample::Package(package) = &settings.package_or_example {
                package == "My-Package"
            } else {
                false
            }
        );
        assert_eq!(settings.features.len(), 1);
        assert_eq!(settings.features.first().unwrap(), "my-feature");
        assert_eq!(settings.asset_folders.len(), 1);
        assert_eq!(
            settings.asset_folders.first().unwrap().to_string(),
            "/asset"
        );
    }

    #[test]
    fn given_a_manifest_with_an_example_provides_the_correct_example() {
        let default_target = Target::current().expect("No default target for this platform");

        let config = DexterousConfig {
            examples: ([
                (
                    "My-Example".to_owned(),
                    ReloadTargetConfig {
                        features: vec!["my-feature".to_string()],
                        asset_folders: vec![Utf8PathBuf::from("/asset")],
                        ..Default::default()
                    },
                ),
                (
                    "My-Other-Example".to_owned(),
                    ReloadTargetConfig {
                        features: vec!["shouldnt-load".to_string()],
                        asset_folders: vec![],
                        ..Default::default()
                    },
                ),
            ])
            .into_iter()
            .collect(),
            ..Default::default()
        };

        let build_settings = config
            .generate_build_settings(
                Some(PackageOrExample::Example("My-Example".to_string())),
                &[],
            )
            .expect("Couldn't generate build settings");

        assert_eq!(build_settings.len(), 1);

        let (target, settings) = build_settings.first().expect("No Targets Set Up");

        assert_eq!(target, &default_target);

        assert!(
            if let PackageOrExample::Example(example) = &settings.package_or_example {
                example == "My-Example"
            } else {
                false
            }
        );
        assert_eq!(settings.features.len(), 1);
        assert_eq!(settings.features.first().unwrap(), "my-feature");
        assert_eq!(settings.asset_folders.len(), 1);
        assert_eq!(
            settings.asset_folders.first().unwrap().to_string(),
            "/asset"
        );
    }

    #[test]
    fn given_a_feature_list_it_provides_them() {
        let default_target = Target::current().expect("No default target for this platform");

        let config = DexterousConfig {
            asset_folders: vec![Utf8PathBuf::from("/asset")],
            ..Default::default()
        };

        let build_settings = config
            .generate_build_settings(None, &["my-feature".to_string()])
            .expect("Couldn't generate build settings");

        assert_eq!(build_settings.len(), 1);

        let (target, settings) = build_settings.first().expect("No Targets Set Up");

        assert_eq!(target, &default_target);

        assert!(matches!(
            settings.package_or_example,
            PackageOrExample::DefaulPackage
        ));
        assert_eq!(settings.features.len(), 1);
        assert_eq!(settings.features.first().unwrap(), "my-feature");
        assert_eq!(settings.asset_folders.len(), 1);
        assert_eq!(
            settings.asset_folders.first().unwrap().to_string(),
            "/asset"
        );
    }
}
