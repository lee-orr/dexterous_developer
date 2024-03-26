use dexterous_developer_types::{Target, TargetBuildSettings};
use tokio::process::Command;

use crate::types::{
    BuildOutputMessages, Builder, BuilderIncomingMessages, BuilderOutgoingMessages,
};

pub struct SimpleBuilder {
    target: Target,
    settings: TargetBuildSettings,
    incoming: tokio::sync::mpsc::UnboundedSender<BuilderIncomingMessages>,
    outgoing: tokio::sync::broadcast::Sender<BuilderOutgoingMessages>,
    output: tokio::sync::broadcast::Sender<BuildOutputMessages>,
    #[allow(dead_code)]
    handle: tokio::task::JoinHandle<()>,
}

async fn build(
    target: Target,
    TargetBuildSettings {
        package_or_example,
        features,
        ..
    }: TargetBuildSettings,
    _sender: tokio::sync::broadcast::Sender<BuildOutputMessages>,
) {
    let mut cargo = Command::new("cargo");
    cargo
        .env_remove("LD_DEBUG")
        .env("RUSTFLAGS", "-Cprefer-dynamic")
        .env("CARGO_TARGET_DIR", format!("./target/hot-reload/{target}"))
        .arg("build")
        .arg("--lib")
        .arg("--message-format=json-render-diagnostics")
        .arg("--profile")
        .arg("dev")
        .arg("--target")
        .arg(target.to_string());

    match package_or_example {
        dexterous_developer_types::PackageOrExample::DefaulPackage => {}
        dexterous_developer_types::PackageOrExample::Package(package) => {
            cargo.arg("-p").arg(package.as_str());
        }
        dexterous_developer_types::PackageOrExample::Example(example) => {
            cargo.arg("--example").arg(example.as_str());
        }
    }

    if !features.is_empty() {
        cargo.arg("--features");
        cargo.arg(features.join(",").as_str());
    }
}

impl SimpleBuilder {
    pub fn new(target: Target, settings: TargetBuildSettings) -> Self {
        let (incoming, mut incoming_rx) = tokio::sync::mpsc::unbounded_channel();
        let (outgoing_tx, _) = tokio::sync::broadcast::channel(100);
        let (output_tx, _) = tokio::sync::broadcast::channel(100);

        let handle = {
            let outgoing_tx = outgoing_tx.clone();
            let output_tx = output_tx.clone();
            let settings = settings.clone();
            tokio::spawn(async move {
                let mut should_build = false;

                while let Some(recv) = incoming_rx.recv().await {
                    match recv {
                        BuilderIncomingMessages::RequestBuild => {
                            should_build = true;
                            let _ = outgoing_tx.send(BuilderOutgoingMessages::BuildStarted);
                            build(target, settings.clone(), output_tx.clone()).await;
                        }
                        BuilderIncomingMessages::CodeChanged => {
                            if should_build {
                                let _ = outgoing_tx.send(BuilderOutgoingMessages::BuildStarted);
                                build(target, settings.clone(), output_tx.clone()).await;
                            }
                        }
                        BuilderIncomingMessages::AssetChanged(asset) => {
                            let _ = output_tx.send(BuildOutputMessages::AssetUpdated(asset));
                        }
                    }
                }
            })
        };

        Self {
            settings,
            target,
            incoming,
            outgoing: outgoing_tx,
            output: output_tx,
            handle,
        }
    }
}

impl Builder for SimpleBuilder {
    fn target(&self) -> Target {
        self.target
    }

    fn incoming_channel(
        &self,
    ) -> tokio::sync::mpsc::UnboundedSender<crate::types::BuilderIncomingMessages> {
        self.incoming.clone()
    }

    fn outgoing_channel(
        &self,
    ) -> (
        tokio::sync::broadcast::Receiver<crate::types::BuilderOutgoingMessages>,
        tokio::sync::broadcast::Receiver<crate::types::BuildOutputMessages>,
    ) {
        (self.outgoing.subscribe(), self.output.subscribe())
    }

    fn root_lib_name(&self) -> Option<camino::Utf8PathBuf> {
        None
    }

    fn get_code_subscriptions(&self) -> Vec<camino::Utf8PathBuf> {
        self.settings.code_watch_folders.clone()
    }

    fn get_asset_subscriptions(&self) -> Vec<camino::Utf8PathBuf> {
        self.settings.asset_folders.clone()
    }
}
