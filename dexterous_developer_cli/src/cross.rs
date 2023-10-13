use std::{collections::HashMap, path::Path};

use anyhow::{bail, Context};

use dexterous_developer_internal::Target;

use url::Url;

use crate::paths::{get_paths, CliPaths};

pub async fn install_cross(
    targets: &[Target],
    macos_sdk: Option<AppleSDKPath>,
) -> anyhow::Result<()> {
    let CliPaths { data, cross_config } = get_paths()?;

    let targets = if targets.is_empty() {
        CROSS_TARGETS
    } else {
        targets
    };

    for rust in targets.iter() {
        setup_target(rust).await?;
    }

    let binstall = {
        let command_list = tokio::process::Command::new("cargo")
            .arg("--list")
            .output()
            .await?;
        let commands = std::str::from_utf8(&command_list.stdout)?;
        commands.contains("binstall")
    };

    println!("Installing cross");
    let status = tokio::process::Command::new("cargo")
        .arg("+nightly")
        .args({
            let args: &'static [&'static str] = if binstall {
                &["binstall", "-y"]
            } else {
                &["install"]
            };
            args
        })
        .arg("cross")
        .status()
        .await?;

    if !status.success() {
        bail!("Failed to install cross-rs");
    }

    println!("Setting up custom images");
    setup_custom_images(&data, &cross_config, targets, macos_sdk).await?;

    Ok(())
}

async fn setup_target(rust: &str) -> anyhow::Result<()> {
    println!("Checking for target: {rust}");
    let output = tokio::process::Command::new("rustup")
        .arg("+nightly")
        .arg("target")
        .arg("list")
        .arg("--installed")
        .output()
        .await?;
    let std_out = std::str::from_utf8(&output.stdout)?;
    if !std_out.contains(rust) {
        println!("Installing target: {rust}");
        let result = tokio::process::Command::new("rustup")
            .arg("+nightly")
            .arg("target")
            .arg("add")
            .arg(rust)
            .status()
            .await;
        match result {
            Ok(result) => {
                if !result.success() {
                    eprintln!("Failed to install {rust}");
                    eprintln!("{result:?}");
                    bail!("Failed to install {rust} with {result:?}");
                } else {
                    println!("Successfully installed {rust}");
                }
            }
            Err(e) => {
                eprintln!("Failed to install {rust} with error: {e:?}");
                bail!("Failed to install {rust} with error: {e:?}")
            }
        }
    } else {
        println!("{rust} already installed");
    }

    Ok(())
}

const CROSS_TARGETS: &[Target] = &[
    Target::Linux,
    Target::LinuxArm,
    Target::Windows,
    Target::Mac,
    Target::MacArm,
];

#[allow(clippy::single_match)]
pub fn check_cross_requirements_installed(_target: &Target) -> anyhow::Result<()> {
    Ok(())
}

pub async fn setup_custom_images(
    data: &Path,
    cross_toml_path: &Path,
    targets: &[Target],
    macos_sdk: Option<AppleSDKPath>,
) -> anyhow::Result<()> {
    let cross_dir = data.join("cross");

    if cross_dir.exists() {
        println!("Updating cross-rs repository");
        let cloned = tokio::process::Command::new("git")
            .args(["pull"])
            .current_dir(&cross_dir)
            .status()
            .await
            .context("Cloning cross-rs repository")?;

        if !cloned.success() {
            bail!("Failed to clone cross-rs");
        }
    } else {
        println!("Cloning cross-rs repository");
        let cloned = tokio::process::Command::new("git")
            .args(["clone", "https://github.com/cross-rs/cross"])
            .current_dir(data)
            .status()
            .await
            .context("Cloning cross-rs repository")?;

        if !cloned.success() {
            bail!("Failed to clone cross-rs");
        }
    }

    tokio::fs::remove_file(cross_dir.join("Cargo.lock")).await?;

    println!("Initializing submodules");
    let submodules = tokio::process::Command::new("git")
        .args(["submodule", "update", "--init", "--remote"])
        .current_dir(&cross_dir)
        .status()
        .await
        .context("initializing cross submodules repository")?;

    if !submodules.success() {
        bail!("Failed to initialize cross submodules");
    }

    let cross_toml = if cross_toml_path.exists() {
        tokio::fs::read_to_string(&cross_toml_path).await?
    } else {
        "".to_string()
    };

    let mut cross_toml: CrossConfig = toml::from_str(&cross_toml).unwrap_or_default();

    for image in targets.iter() {
        let image_name = match image {
            Target::Windows => format!("{image}"),
            Target::Linux => format!("{image}"),
            Target::LinuxArm => format!("{image}"),
            _ => format!("{image}-cross"),
        };

        println!("Building cross image for {image}");

        let mut build_command = tokio::process::Command::new("cargo");
        build_command
            .current_dir(&cross_dir)
            .arg("+nightly")
            .arg("build-docker-image")
            .arg(&image_name)
            .arg("--tag")
            .arg("local");

        if *image == Target::Mac || *image == Target::MacArm {
            let Some(macos_sdk) = macos_sdk.as_ref() else {
                bail!("Building the mac image requires a URL to a packaged mac sdk. Please look at here for more info: https://github.com/cross-rs/cross-toolchains#darwin-targets");
            };

            let macos_sdk = match macos_sdk {
                AppleSDKPath::Url(url) => {
                    println!("Downloading MacOs SDK from {url}");
                    let macos_sdk = url.as_str();
                    let version_number = {
                        let mut split = macos_sdk.split("OSX");
                        let val = split
                            .nth(1)
                            .context("File name doesn't have format of *OSX{version}.sdk*")?;
                        let mut split = val.split(".sdk");
                        split
                            .next()
                            .context("File name doesn't have format of *OSX{version}.sdk*")?
                    };
                    let file_name = url.path_segments().context("URL isn't a download url")?;
                    let file_name = file_name.last().context("No file name in url")?;

                    let client = reqwest::Client::default();
                    let req = client
                        .get(macos_sdk)
                        .header("User-Agent", "dexterous_developer_cli")
                        .build()
                        .context("Constructing SDK Download Request")?;
                    let sdk = client.execute(req).await.context("Downloading MacOS Sdk")?;

                    let sdk_path_folder = cross_dir.join("docker/macos_sdk_dir");
                    tokio::fs::create_dir_all(&sdk_path_folder).await?;

                    let sdk_path = sdk_path_folder.join(file_name);

                    tokio::fs::write(&sdk_path, sdk.bytes().await?).await?;

                    println!("Setting up darwin.sh");

                    let backap_darwin =
                        cross_dir.join("docker/cross-toolchains/docker/darwin_back");
                    let darwin = cross_dir.join("docker/cross-toolchains/docker/darwin.sh");

                    if !backap_darwin.exists() {
                        tokio::fs::copy(&darwin, &backap_darwin).await?;
                    }
                    let darwin_sh_file = tokio::fs::read_to_string(&backap_darwin).await?;
                    let darwin_sh_file = darwin_sh_file.replace(
                        "OSX_VERSION_MIN=10.7",
                        &format!("OSX_VERSION_MIN={version_number}"),
                    );
                    tokio::fs::write(&darwin, darwin_sh_file).await?;

                    let docker_name = format!(
                        "docker/cross-toolchains/docker/Dockerfile.{}-apple-darwin-cross",
                        if image == &Target::MacArm {
                            "aarch64"
                        } else {
                            "x86_64"
                        }
                    );
                    let docker = cross_dir.join(docker_name);
                    let backap_docker = docker.with_extension("backup");

                    if !backap_docker.exists() {
                        tokio::fs::copy(&docker, &backap_docker).await?;
                    }

                    let docker_file = tokio::fs::read_to_string(&backap_docker).await?;
                    let docker_file = format!(
                        r#"{docker_file}
ENV COREAUDIO_SDK_PATH=/opt/osxcross/SDK/latest
"#
                    );
                    tokio::fs::write(&docker, docker_file).await?;

                    (
                        "MACOS_SDK_DIR=./macos_sdk_dir".to_string(),
                        format!("MACOS_SDK_FILE={file_name}"),
                    )
                }
            };

            build_command.args(["--build-arg", &macos_sdk.0, "--build-arg", &macos_sdk.1]);
        }

        let build_image = build_command
            .status()
            .await
            .context(format!("Failed run build-doicker-image for {image}"))?;

        if !build_image.success() {
            bail!("Failed to build image for {image}");
        }

        cross_toml.target.insert(
            *image,
            CrossTargetConfig {
                image: format!("ghcr.io/cross-rs/{image_name}:local"),
            },
        );
    }

    println!("Writing Cross Config");
    tokio::fs::write(&cross_toml_path, toml::to_string(&cross_toml)?)
        .await
        .context("Writing Cross Config")?;

    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct CrossConfig {
    target: HashMap<Target, CrossTargetConfig>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CrossTargetConfig {
    image: String,
}

pub enum AppleSDKPath {
    Url(Url),
}
