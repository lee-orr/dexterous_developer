use std::{
    collections::{HashMap, HashSet},
    env, fs,
    process::{Command, Stdio},
    sync::{atomic::AtomicU32, Arc},
};

use anyhow::bail;

use camino::{Utf8Path, Utf8PathBuf};
use dexterous_developer_types::{cargo_path_utils::dylib_path, Target, TargetBuildSettings};
use tracing::{error, info};

use crate::types::{
    BuildOutputMessages, Builder, BuilderIncomingMessages, BuilderOutgoingMessages,
    HashedFileRecord,
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

fn build(
    target: Target,
    TargetBuildSettings {
        working_dir,
        package_or_example,
        features,
        ..
    }: TargetBuildSettings,
    sender: tokio::sync::broadcast::Sender<BuildOutputMessages>,
    id: u32,
) -> Result<(), anyhow::Error> {
    let mut cargo = Command::new("cargo");
    if let Some(working_dir) = working_dir {
        cargo.current_dir(&working_dir);
    }
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
    let mut child = cargo.stdout(Stdio::piped()).spawn()?;

    let reader =
        std::io::BufReader::new(child.stdout.take().ok_or(anyhow::Error::msg("no stdout"))?);

    let mut succeeded = false;

    let mut artifacts = Vec::with_capacity(20);

    for msg in cargo_metadata::Message::parse_stream(reader) {
        let message = msg?;

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
            msg => info!("Compiler: {msg:?}"),
        }
    }

    let result = child.wait()?;

    if !result.success() || !succeeded {
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
                                info!("Checking if {package_name} == {p}");
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
        let dir = std::fs::read_dir(toolchains)?;

        for child in dir {
            let Ok(child) = child else {
                continue;
            };
            if child.file_type()?.is_dir() {
                let path = Utf8PathBuf::from_path_buf(child.path()).unwrap_or_default();
                path_var.push(path.join("lib"));
            }
        }
    }

    info!("Path Var for DyLib Search: {path_var:?}");

    let mut searchable_files = HashMap::new();

    for (name, path) in path_var
        .iter()
        .map(|dir| {
            std::fs::read_dir(dir).map(|value| {
                let files = value.filter_map(|value| {
                    if let Ok(value) = value {
                        if let Ok(t) = value.file_type() {
                            if t.is_file() {
                                let path = Utf8PathBuf::from_path_buf(value.path());
                                if let Ok(path) = path {
                                    if let Some(name) = path.file_name() {
                                        return Some((name.to_string(), path));
                                    }
                                }
                            }
                        }
                    }
                    None
                });
                files.collect::<Vec<(String, Utf8PathBuf)>>()
            })
        })
        .collect::<std::io::Result<Vec<Vec<(String, Utf8PathBuf)>>>>()?
        .into_iter()
        .flatten()
    {
        searchable_files.entry(name).or_insert(path);
    }

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

    for (library, local_path) in libraries.iter() {
        let file = std::fs::read(local_path)?;
        let hash = blake3::hash(&file);

        let _ = sender.send(BuildOutputMessages::LibraryUpdated(HashedFileRecord {
            name: library.clone(),
            local_path: local_path.clone(),
            relative_path: Utf8PathBuf::from(format!("./{library}")),
            hash: hash.as_bytes().to_owned(),
            dependencies: dependencies.get(library).cloned().unwrap_or_default(),
        }));
    }

    if let Some(root_library) = root_library {
        let _ = sender.send(BuildOutputMessages::RootLibraryName(
            root_library.to_string(),
        ));
    } else {
        error!("No Root Library");
    }

    let _ = sender.send(BuildOutputMessages::EndedBuild(id));
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
            error!("Couldn't find library with name {library_name}");
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

impl SimpleBuilder {
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
                            let _ = build(target, settings.clone(), output_tx.clone(), id);
                        }
                        BuilderIncomingMessages::CodeChanged => {
                            if should_build {
                                let id = id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                let _ = outgoing_tx.send(BuilderOutgoingMessages::BuildStarted);
                                let _ = build(target, settings.clone(), output_tx.clone(), id);
                            }
                        }
                        BuilderIncomingMessages::AssetChanged(asset) => {
                            info!("Builder Received Asset Change - {asset:?}");
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

        let build = SimpleBuilder::new(
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

        let msg = timeout(Duration::from_secs(60), builder_messages.recv())
            .await
            .expect("Didn't recieve watcher message on time")
            .expect("Didn't recieve watcher message");

        assert!(matches!(msg, BuilderOutgoingMessages::BuildStarted));

        let mut started = false;
        let mut ended = false;
        let mut root_lib_confirmed = false;
        let mut library_update_received = false;

        timeout(Duration::from_secs(60 * 30), async {
            loop {
                let msg = build_messages
                    .recv()
                    .await
                    .expect("Couldn't get build message");
                match msg {
                    BuildOutputMessages::StartedBuild(id) => {
                        if started {
                            panic!("Started more than once");
                        }
                        assert_eq!(id, 1);
                        started = true;
                    }
                    BuildOutputMessages::EndedBuild(id) => {
                        assert_eq!(id, 1);
                        ended = true;
                        break;
                    }
                    BuildOutputMessages::RootLibraryName(name) => {
                        assert_eq!(name, target.dynamic_lib_name("test_lib"));
                        root_lib_confirmed = true;
                    }
                    BuildOutputMessages::LibraryUpdated(HashedFileRecord {
                        local_path,
                        dependencies,
                        name,
                        ..
                    }) => {
                        assert!(local_path.exists());
                        if name == target.dynamic_lib_name("test_lib") {
                            assert!(dependencies.len() == 1, "Dependencies: {dependencies:?}");
                            library_update_received = true;
                        }
                    }
                    BuildOutputMessages::AssetUpdated(_) => {}
                    BuildOutputMessages::KeepAlive => {}
                }
            }
        })
        .await
        .expect("Didn't complete build on time");

        assert!(started);
        assert!(ended);
        assert!(root_lib_confirmed);
        assert!(library_update_received);
    }
}
