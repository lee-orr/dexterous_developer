use anyhow::{bail, Context};

use dexterous_developer_internal::Target;

use crate::paths::{get_paths, CliPaths};

pub async fn install_cross() -> anyhow::Result<()> {
    let CliPaths { data: _ } = get_paths()?;

    for rust in CROSS_TARGETS.iter() {
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

    let status = tokio::process::Command::new("cargo")
        .arg(if binstall { "binstall" } else { "install" })
        .arg("-y")
        .arg("cross")
        .status()
        .await?;

    if !status.success() {
        bail!("Failed to install cross-rs");
    }

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

pub const CROSS_TARGETS: &[&str] = &[
    Target::Linux.to_static(),
    Target::LinuxArm.to_static(),
    Target::Windows.to_static(),
    // #[cfg(target_os = "linux")]
    // Target::Mac.to_static(),
    // #[cfg(target_os = "linux")]
    // Target::MacArm.to_static(),
];

#[allow(clippy::single_match)]
pub fn check_cross_requirements_installed(target: &Target) -> anyhow::Result<()> {
    Ok(())
}
