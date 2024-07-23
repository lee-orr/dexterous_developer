#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dexterous_developer_builder::incremental_builder::dll_tool::dll_tool().await
}
