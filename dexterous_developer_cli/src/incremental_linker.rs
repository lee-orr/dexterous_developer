//! A linker that catches incremental change artifacts applies recent changes to a dynamic library
//! 
//! Heavily derived from Jon Kelley's work - https://github.com/jkelleyrtp/ipbp/blob/main/packages/patch-linker/src/main.rs

use std::time::SystemTime;

use camino::Utf8PathBuf;
use dexterous_developer_builder::incremental_builder::IncrementalRunParams;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = std::env::args().filter(|v| !v.contains("dexterous_developer_incremental_linker")).collect::<Vec<String>>();
    let incremental_run_params : IncrementalRunParams = serde_json::from_str(&std::env::var("DEXTEROUS_DEVELOPER_INCREMENTAL_RUN")?)?;

    match incremental_run_params {
        IncrementalRunParams::InitialRun => basic_link(args).await,
        IncrementalRunParams::Patch { id, out_path_base, timestamp } => patch_link(args, id, out_path_base, timestamp).await
    }
}


async fn basic_link(args: Vec<String>) -> anyhow::Result<()> {
    let output = tokio::process::Command::new("cc").args(&args).spawn()?.wait_with_output().await?;
    std::process::exit(output.status.code().unwrap_or_default());
}

async fn patch_link(args: Vec<String>, id: u32, out_path_base: Utf8PathBuf, timestamp: SystemTime) -> anyhow::Result<()> {
    todo!()
}
