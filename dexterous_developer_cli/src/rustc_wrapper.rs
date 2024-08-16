#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dexterous_developer_builder::default_builder::rustc::default_rustc().await
}
