#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dexterous_developer_builder::incremental_builder::rustc::incremental_rustc().await
}
