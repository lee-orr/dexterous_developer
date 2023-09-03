use anyhow::bail;

use crate::paths::{get_paths, CliPaths};

pub async fn install_cross() -> anyhow::Result<()> {
    let CliPaths { data: _ } = get_paths()?;

    for rust in CROSS_TARGETS.iter() {
        setup_target(rust).await?;
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
    #[cfg(target_os = "linux")]
    "aarch64-unknown-linux-gnu",
    #[cfg(target_os = "linux")]
    "x86_64-unknown-linux-gnu",
    #[cfg(target_os = "linux")]
    "x86_64-pc-windows-gnu",
    #[cfg(target_os = "linux")]
    "aarch64-apple-darwin",
    #[cfg(target_os = "linux")]
    "x86_64-apple-darwin",
];
