use futures_util::future::join_all;
use std::{
    collections::{HashMap, HashSet},
    env, fs,
    process::Stdio,
    sync::{atomic::AtomicU32, Arc},
};

use anyhow::bail;

use camino::{Utf8Path, Utf8PathBuf};
use dexterous_developer_types::{cargo_path_utils::dylib_path, Target, TargetBuildSettings};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};
use tracing::{debug, error, info, trace, warn};

use crate::types::{
    BuildOutputMessages, Builder, BuilderIncomingMessages, BuilderOutgoingMessages,
    HashedFileRecord,
};

pub struct IncrementalBuilder {
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
        working_dir,
        package_or_example,
        features,
        manifest_path,
        ..
    }: TargetBuildSettings,
    sender: tokio::sync::broadcast::Sender<BuildOutputMessages>,
    id: u32,
) -> Result<(), anyhow::Error> {
    info!("Build {id} Started");
    let mut cargo = Command::new("cargo");
    if let Some(working_dir) = working_dir {
        cargo.current_dir(&working_dir);
    }
    cargo
        .env_remove("LD_DEBUG")
        .env("RUSTFLAGS", "-Cprefer-dynamic")
        .env("CARGO_TARGET_DIR", format!("./target/hot-reload/{target}"))
        .arg("rustc")
        .arg("--lib")
        .arg("--message-format=json-render-diagnostics")
        .arg("--profile")
        .arg("dev")
        .arg("--target")
        .arg(target.to_string());

    if let Some(manifest) = manifest_path {
        cargo.arg("--manifest-path").arg(manifest.canonicalize()?);
    }

    match &package_or_example {
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

    let _ = sender.send(BuildOutputMessages::StartedBuild(id));
    let mut child = cargo
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut succeeded = false;

    let mut artifacts = Vec::with_capacity(20);

    let Some(output) = child.stdout.take() else {
        bail!("No Std Out");
    };

    let Some(error) = child.stderr.take() else {
        bail!("No Std Err");
    };

    tokio::spawn(async move {
        let mut out_reader = BufReader::new(error).lines();
        while let Ok(Some(line)) = out_reader.next_line().await {
            warn!("Compilation - {line}");
        }
    });

    let mut out_reader = BufReader::new(output).lines();

    while let Some(line) = out_reader.next_line().await? {
        trace!("Compiler Output: {line}");
        let message = serde_json::from_str(&line)?;

        match &message {
            cargo_metadata::Message::CompilerArtifact(artifact) => {
                if artifact.target.crate_types.iter().any(|v| v == "dylib") {
                    artifacts.push(artifact.clone());
                }
            }
            cargo_metadata::Message::BuildFinished(finished) => {
                info!("Build Finished: {finished:?}");
                succeeded = finished.success;
            }
            msg => trace!("Compiler: {msg:?}"),
        }
    }

    if !succeeded {
        error!("Build Failed");
        bail!("Failed to build");
    }

    let mut libraries = HashMap::<String, Utf8PathBuf>::with_capacity(20);
    let mut root_dir = HashSet::new();

    let mut root_library = None;

    for artifact in artifacts.iter() {
        for path in artifact.filenames.iter() {
            if let Some(ext) = path.extension() {
                if ext == target.dynamic_lib_extension() {
                    if let Some(name) = path.file_name() {
                        match &package_or_example {
                            dexterous_developer_types::PackageOrExample::DefaulPackage => {
                                root_library = Some(name);
                            }
                            dexterous_developer_types::PackageOrExample::Package(p) => {
                                let file_section = artifact
                                    .package_id
                                    .repr
                                    .split('/')
                                    .last()
                                    .unwrap_or_default();
                                let package_name =
                                    file_section.split('#').last().unwrap_or_default();
                                let package_name =
                                    package_name.split('@').next().unwrap_or_default();
                                trace!("Checking if {package_name} == {p}");
                                if package_name == p {
                                    root_library = Some(name);
                                }
                            }
                            dexterous_developer_types::PackageOrExample::Example(e) => {
                                if artifact.target.kind.contains(&"example".to_string())
                                    && &artifact.target.name == e
                                {
                                    root_library = Some(name);
                                }
                            }
                        }
                        let path = path.canonicalize_utf8()?;
                        let parent = path
                            .parent()
                            .ok_or_else(|| anyhow::Error::msg("file has no parent"))?;
                        root_dir.insert(parent.to_path_buf());
                        libraries.insert(name.to_string(), path);
                    }
                }
            }
        }
    }

    let initial_libraries = libraries
        .iter()
        .map(|(name, path)| (name.clone(), path.clone()))
        .collect::<Vec<_>>();

    let mut path_var = match env::var_os("PATH") {
        Some(var) => env::split_paths(&var)
            .filter_map(|p| Utf8PathBuf::try_from(p).ok())
            .collect(),
        None => Vec::new(),
    };
    let mut dylib_paths = dylib_path();
    let mut root_dirs = root_dir.into_iter().collect::<Vec<_>>();
    path_var.append(&mut dylib_paths);
    path_var.append(&mut root_dirs);
    path_var.push(
        Utf8PathBuf::from_path_buf(env::current_dir()?)
            .unwrap_or_default()
            .join("target")
            .join("hot-reload")
            .join(target.to_string())
            .join(target.to_string())
            .join("debug")
            .join("deps"),
    );

    {
        let rustup_home = home::rustup_home()?;
        let toolchains = rustup_home.join("toolchains");
        let mut dir = tokio::fs::read_dir(toolchains).await?;

        while let Ok(Some(child)) = dir.next_entry().await {
            if child.file_type().await?.is_dir() {
                let path = Utf8PathBuf::from_path_buf(child.path()).unwrap_or_default();
                path_var.push(path.join("lib"));
            }
        }
    }

    trace!("Path Var for DyLib Search: {path_var:?}");
    let dir_collections = path_var.iter().map(|dir| {
        let dir = dir.clone();
        tokio::spawn(async {
            let Ok(mut dir) = tokio::fs::read_dir(dir).await else {
                return vec![];
            };
            let mut files = vec![];

            while let Ok(Some(child)) = dir.next_entry().await {
                let Ok(file_type) = child.file_type().await else {
                    continue;
                };

                if file_type.is_file() {
                    let Ok(path) = Utf8PathBuf::from_path_buf(child.path()) else {
                        continue;
                    };

                    if let Some(name) = path.file_name() {
                        files.push((name.to_owned(), path))
                    }
                }
            }

            files
        })
    });

    let searchable_files = join_all(dir_collections)
        .await
        .iter()
        .filter_map(|result| match result {
            Ok(v) => Some(v),
            Err(_) => None,
        })
        .flatten()
        .cloned()
        .collect::<HashMap<_, _>>();

    let mut dependencies = HashMap::new();

    for (name, library) in initial_libraries.iter() {
        process_dependencies_recursive(
            &searchable_files,
            &mut libraries,
            &mut dependencies,
            name,
            library,
        )?;
    }

    let libraries = libraries
        .iter()
        .map(|(library, local_path)| {
            let file = std::fs::read(local_path)?;
            let hash = blake3::hash(&file);

            Ok(HashedFileRecord {
                name: library.clone(),
                local_path: local_path.clone(),
                relative_path: Utf8PathBuf::from(format!("./{library}")),
                hash: hash.as_bytes().to_owned(),
                dependencies: dependencies.get(library).cloned().unwrap_or_default(),
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let Some(root_library) = root_library.map(|r| r.to_string()) else {
        error!("No Root Library");
        bail!("No Root Library");
    };

    let _ = sender.send(BuildOutputMessages::EndedBuild {
        id,
        libraries,
        root_library,
    });
    info!("Build {id} Completed");
    Ok(())
}

fn process_dependencies_recursive(
    searchable_files: &HashMap<String, Utf8PathBuf>,
    libraries: &mut HashMap<String, Utf8PathBuf>,
    dependencies: &mut HashMap<String, Vec<String>>,
    current_library_name: &str,
    current_library: &Utf8Path,
) -> Result<(), anyhow::Error> {
    let file = fs::read(current_library)?;
    let file = goblin::Object::parse(&file)?;

    let dependency_vec = match file {
        goblin::Object::Elf(elf) => {
            let str_table = elf.dynstrtab;
            elf.dynamic
                .map(|dynamic| {
                    dynamic
                        .get_libraries(&str_table)
                        .iter()
                        .map(|v| v.to_string())
                        .collect()
                })
                .unwrap_or_default()
        }
        goblin::Object::PE(pe) => pe
            .libraries
            .iter()
            .map(|import| import.to_string())
            .collect(),
        goblin::Object::Mach(mach) => match mach {
            goblin::mach::Mach::Fat(fat) => {
                let mut vec = HashSet::new();
                while let Some(Ok(goblin::mach::SingleArch::MachO(arch))) = fat.into_iter().next() {
                    let imports = arch.imports()?;
                    let inner = imports.iter().map(|v| v.dylib.to_string());
                    vec.extend(inner);
                }
                vec
            }
            goblin::mach::Mach::Binary(std) => {
                std.imports()?.iter().map(|v| v.dylib.to_string()).collect()
            }
        },
        _ => HashSet::default(),
    };

    for library_name in dependency_vec.iter() {
        if library_name.is_empty() {
            continue;
        }
        if libraries.contains_key(library_name) {
            continue;
        }
        let Some(library_path) = searchable_files.get(library_name) else {
            debug!("Couldn't find library with name {library_name}");
            continue;
        };
        libraries.insert(library_name.to_string(), library_path.clone());
    }
    dependencies.insert(
        current_library_name.to_string(),
        dependency_vec.into_iter().collect(),
    );
    Ok(())
}

impl IncrementalBuilder {
    pub fn new(target: Target, settings: TargetBuildSettings) -> Self {
        let (incoming, mut incoming_rx) = tokio::sync::mpsc::unbounded_channel();
        let (outgoing_tx, _) = tokio::sync::broadcast::channel(100);
        let (output_tx, _) = tokio::sync::broadcast::channel(100);
        let id = Arc::new(AtomicU32::new(1));

        let handle = {
            let outgoing_tx = outgoing_tx.clone();
            let output_tx = output_tx.clone();
            let settings = settings.clone();
            let id = id.clone();
            tokio::spawn(async move {
                let mut should_build = false;

                while let Some(recv) = incoming_rx.recv().await {
                    match recv {
                        BuilderIncomingMessages::RequestBuild => {
                            should_build = true;
                            let id = id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            let _ = outgoing_tx.send(BuilderOutgoingMessages::BuildStarted);
                            #[allow(clippy::let_underscore_future)]
                            let _ = tokio::spawn(build(
                                target,
                                settings.clone(),
                                output_tx.clone(),
                                id,
                            ));
                        }
                        BuilderIncomingMessages::CodeChanged => {
                            if should_build {
                                let id = id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                let _ = outgoing_tx.send(BuilderOutgoingMessages::BuildStarted);
                                #[allow(clippy::let_underscore_future)]
                                let _ = tokio::spawn(build(
                                    target,
                                    settings.clone(),
                                    output_tx.clone(),
                                    id,
                                ));
                            }
                        }
                        BuilderIncomingMessages::AssetChanged(asset) => {
                            trace!("Builder Received Asset Change - {asset:?}");
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

impl Builder for IncrementalBuilder {
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

    fn root_lib_name(&self) -> Option<String> {
        None
    }

    fn get_code_subscriptions(&self) -> Vec<camino::Utf8PathBuf> {
        self.settings.code_watch_folders.clone()
    }

    fn get_asset_subscriptions(&self) -> Vec<camino::Utf8PathBuf> {
        self.settings.asset_folders.clone()
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use super::*;
    use dexterous_developer_types::PackageOrExample;
    use test_temp_dir::*;
    use tokio::io::AsyncWriteExt;
    use tokio::process::Command;
    use tokio::time::timeout;

    #[tokio::test]
    async fn can_build_a_package() {
        let dir = test_temp_dir!();
        let dir_path = dir.as_path_untracked().to_path_buf();
        let cargo = dir_path.join("Cargo.toml");

        let _ = Command::new("cargo")
            .current_dir(&dir_path)
            .arg("init")
            .arg("--name=test_lib")
            .arg("--vcs=none")
            .arg("--lib")
            .output()
            .await
            .expect("Failed to create test project");

        {
            let mut file = tokio::fs::File::options()
                .append(true)
                .open(&cargo)
                .await
                .expect("Couldn't open cargo toml");
            file.write_all(
                r#"[lib]
            crate-type = ["rlib", "dylib"]"#
                    .as_bytes(),
            )
            .await
            .expect("Couldn't write to cargo toml");
            file.sync_all()
                .await
                .expect("Couldn't flush write to cargo toml");
        }

        let target = Target::current().expect("Couldn't determine current target");

        let build = IncrementalBuilder::new(
            target,
            TargetBuildSettings {
                package_or_example: PackageOrExample::Package("test_lib".to_string()),
                working_dir: Utf8PathBuf::from_path_buf(dir_path).ok(),
                code_watch_folders: vec![Utf8PathBuf::from_path_buf(
                    dir.as_path_untracked().join("src"),
                )
                .unwrap()],
                ..Default::default()
            },
        );

        let (mut builder_messages, mut build_messages) = build.outgoing_channel();

        build
            .incoming
            .send(BuilderIncomingMessages::RequestBuild)
            .expect("Failed to request build");

        let msg = timeout(Duration::from_secs(10), builder_messages.recv())
            .await
            .expect("Didn't recieve watcher message on time")
            .expect("Didn't recieve watcher message");

        assert!(matches!(msg, BuilderOutgoingMessages::BuildStarted));

        let mut started = false;
        let mut ended = false;
        let mut root_lib_confirmed = false;
        let mut library_update_received = false;

        let mut messages = Vec::new();

        if let Err(e) = timeout(Duration::from_secs(10), async {
            loop {
                let msg = build_messages
                    .recv()
                    .await
                    .expect("Couldn't get build message");
                messages.push(msg.clone());
                match msg {
                    BuildOutputMessages::StartedBuild(id) => {
                        if started {
                            panic!("Started more than once");
                        }
                        assert_eq!(id, 1);
                        started = true;
                    }
                    BuildOutputMessages::EndedBuild {
                        id,
                        libraries,
                        root_library,
                    } => {
                        assert_eq!(id, 1);
                        ended = true;
                        assert_eq!(root_library, target.dynamic_lib_name("test_lib"));
                        root_lib_confirmed = true;
                        for HashedFileRecord {
                            local_path,
                            dependencies,
                            name,
                            ..
                        } in libraries.into_iter()
                        {
                            assert!(local_path.exists());
                            if name == target.dynamic_lib_name("test_lib") {
                                assert!(dependencies.len() == 1, "Dependencies: {dependencies:?}");
                                library_update_received = true;
                            }
                        }
                        break;
                    }
                    BuildOutputMessages::AssetUpdated(_) => {}
                    BuildOutputMessages::KeepAlive => {}
                }
            }
        })
        .await
        {
            panic!("Failed - {e:?}\n{messages:?}");
        }

        assert!(started);
        assert!(ended);
        assert!(root_lib_confirmed);
        assert!(library_update_received);
    }
}
