//! A linker that catches incremental change artifacts applies recent changes to a dynamic library
//!
//! Heavily derived from Jon Kelley's work - <https://github.com/jkelleyrtp/ipbp/blob/main/packages/patch-linker/src/main.rs>

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dexterous_developer_builder::incremental_builder::linker::linker().await
}