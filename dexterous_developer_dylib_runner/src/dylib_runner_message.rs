use camino::Utf8PathBuf;

#[derive(Debug, Clone)]
pub enum DylibRunnerMessage {
    ConnectionClosed,
    LoadRootLib {
        build_id: u32,
        local_path: Utf8PathBuf,
    },
    AssetUpdated {
        local_path: Utf8PathBuf,
        name: String,
    },
}
