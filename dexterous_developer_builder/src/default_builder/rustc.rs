use std::{process::Stdio, str::FromStr};

use anyhow::{anyhow, bail};
use camino::Utf8PathBuf;

use super::builder::DefaultRunParams;

pub async fn default_rustc() -> anyhow::Result<()> {
    let package_name = std::env::var("DEXTEROUS_DEVELOPER_PACKAGE_NAME")?;
    let output_file = std::env::var("DEXTEROUS_DEVELOPER_OUTPUT_FILE")?;
    let default_run_params: DefaultRunParams =
        serde_json::from_str(&std::env::var("DEXTEROUS_DEVELOPER_DEFAULT_RUN")?)?;

    let rustc = Rustc::new(
        std::env::args(),
        &package_name,
        &output_file,
        &default_run_params,
    )
    .await?;

    let _ = rustc.run().await?;
    Ok(())
}

#[derive(Debug)]
struct Rustc {
    executable: String,
    operation: RustcOperation,
    arg_file: Option<Utf8PathBuf>,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum RustcOperation {
    Passthrough(Vec<String>),
    MainCompilation {
        crate_name: String,
        edition: u32,
        file: Utf8PathBuf,
        crate_type: String,
        emit: String,
        codegen_args: Vec<String>,
        cfg: Vec<String>,
        check_cfg: Vec<String>,
        out_dir: Utf8PathBuf,
        file_name_extras: String,
        target: String,
        search_paths: Vec<String>,
        library_links: Vec<String>,
        extern_links: Vec<String>,
        unstable_flags: Vec<String>
    },
}

impl Rustc {
    async fn new(
        mut args: impl Iterator<Item = String>,
        package: &str,
        output_file: &str,
        run_params: &DefaultRunParams,
    ) -> anyhow::Result<Self> {
        let _current_executable = args.next();

        let Some(executable) = args.next() else {
            bail!("No Rustc Executable In Arguments");
        };

        let mut args: Vec<String> = args.collect();
        let mut arg_file = None;

        if args.len() == 1 {
            if let Some(first) = args.first() {
                if first.starts_with("@") {
                    let path = Utf8PathBuf::from(first.trim_start_matches("@"));

                    if path.exists() {
                        args = tokio::fs::read_to_string(&path)
                            .await?
                            .lines()
                            .map(|v| v.to_owned())
                            .collect();
                        arg_file = Some(path);
                    }
                }
            }
        }

        let operation = {
            let mut is_passthrough = true;
            let mut args_iter = args.iter();
            while let Some(arg) = args_iter.next() {
                if arg.as_str() == "--crate-name" {
                    if let Some(name) = args_iter.next() {
                        if name.as_str() == package {
                            is_passthrough = false;
                            break;
                        }
                    }
                }
            }
            if is_passthrough {
                RustcOperation::Passthrough(args)
            } else {
                let mut crate_name = None;
                let mut edition = 2021;
                let mut file = None;
                let mut crate_type = None;
                let mut emit = None;
                let mut codegen_args = vec![];
                let mut cfg = vec![];
                let mut check_cfg = vec![];
                let mut target = None;
                let mut search_paths = vec![];
                let mut library_links = vec![];
                let mut extern_links = vec![];
                let mut unstable_flags = vec![];

                let mut args_iter = args.iter();
                while let Some(arg) = args_iter.next() {
                    if arg.as_str() == "--crate-name" {
                        crate_name = args_iter.next().cloned();
                    } else if arg.starts_with("--edition") {
                        edition = arg.trim_start_matches("--edition=").parse()?;
                    } else if arg.starts_with("--crate-type") {
                        crate_type = args_iter.next().cloned();
                    } else if arg.starts_with("--emit") {
                        emit = Some(arg.trim_start_matches("--emit=").to_string());
                    } else if arg == "-C" {
                        if let Some(a) = args_iter.next().cloned() {
                            if a.starts_with("extra-filename=") {
                                continue;
                            }
                            codegen_args.push(a);
                        }
                    } else if arg.starts_with("-C") && arg.split_whitespace().count() == 1 {
                        codegen_args.push(arg.trim_start_matches("-C").to_string());
                    } else if arg.starts_with("-Z") {
                        unstable_flags.push(arg.trim_start_matches("-C").to_string());
                    }else if arg == "--cfg" {
                        if let Some(a) = args_iter.next().cloned() {
                            cfg.push(a);
                        }
                    } else if arg == "--check-cfg" {
                        if let Some(a) = args_iter.next().cloned() {
                            check_cfg.push(a);
                        }
                    } else if arg == "-L" {
                        if let Some(a) = args_iter.next().cloned() {
                            search_paths.push(a);
                        }
                    } else if arg == "-l" {
                        if let Some(a) = args_iter.next().cloned() {
                            library_links.push(a);
                        }
                    } else if arg == "--extern" {
                        if let Some(a) = args_iter.next().cloned() {
                            extern_links.push(a);
                        }
                    } else if arg.ends_with(".rs") {
                        let path = Utf8PathBuf::from_str(arg)?;
                        if !path.exists() {
                            bail!("Rust source file doesn't exist - {arg}");
                        }
                        file = Some(path)
                    } else if arg == "--out-dir" {
                        let _ = args_iter
                            .next()
                            .map(|v| Utf8PathBuf::from_str(v))
                            .ok_or(anyhow!("No Out Dir"))??;
                        continue;
                    } else if arg == "--target" {
                        target = args_iter.next().cloned();
                    }
                }

                let output_dir = Utf8PathBuf::from(output_file)
                    .parent()
                    .ok_or(anyhow!("No Parent for Output File"))?
                    .to_owned();

                let file_name_extras = match run_params {
                    DefaultRunParams::InitialRun => ".1".to_string(),
                    DefaultRunParams::Patch { id, .. } => format!(".{id}"),
                };

                RustcOperation::MainCompilation {
                    crate_name: crate_name.ok_or(anyhow!("Couldn't determine crate name"))?,
                    edition,
                    file: file.ok_or(anyhow!("Couldn't determine source file"))?,
                    crate_type: crate_type.ok_or(anyhow!("Couldn't determine crate type"))?,
                    emit: emit.ok_or(anyhow!("Couldn't determin emit parameters"))?,
                    codegen_args,
                    cfg,
                    check_cfg,
                    out_dir: output_dir,
                    target: target.ok_or(anyhow!("Couldn't determine target"))?,
                    search_paths,
                    library_links,
                    extern_links,
                    file_name_extras,
                    unstable_flags,
                }
            }
        };

        Ok(Self {
            executable,
            operation,
            arg_file,
        })
    }

    async fn run(self) -> anyhow::Result<std::process::ExitStatus> {
        let mut command = WrappedCommand::new(self.executable);

        match self.operation {
            RustcOperation::MainCompilation {
                crate_name,
                edition,
                file,
                crate_type,
                emit,
                codegen_args,
                cfg,
                check_cfg,
                out_dir,
                target,
                search_paths,
                library_links,
                extern_links,
                file_name_extras,
                unstable_flags,
                ..
            } => {
                command
                    .arg("--error-format=json")
                    .arg("--json=diagnostic-rendered-ansi,artifacts,future-incompat")
                    .arg("--crate-name")
                    .arg(crate_name)
                    .arg("--edition")
                    .arg(edition.to_string())
                    .arg(file)
                    .arg("--crate-type")
                    .arg(if crate_type == "dylib" || crate_type == "cdylib" {
                        &crate_type
                    } else {
                        "dylib"
                    })
                    .arg(format!("--emit={emit}"))
                    .arg("--target")
                    .arg(target)
                    .arg("--out-dir")
                    .arg(out_dir)
                    .arg("-C")
                    .arg(format!("extra-filename={file_name_extras}"));

                for c in &codegen_args {
                    command.arg("-C").arg(c);
                }
                
                for c in &unstable_flags {
                    command.arg(format!("-Z{c}"));
                }

                for c in &cfg {
                    command.arg("--cfg").arg(c);
                }
                for c in &check_cfg {
                    command.arg("--check-cfg").arg(c);
                }
                for c in &search_paths {
                    command.arg("-L").arg(c);
                }
                for c in &library_links {
                    command.arg("-l").arg(c);
                }
                for c in &extern_links {
                    command.arg("--extern").arg(c);
                }
            }
            RustcOperation::Passthrough(args) => {
                command.args(args.iter());
            }
        };

        if let Some(file) = self.arg_file {
            command.arg_file(file);
        }

        let mut command = command.command().await?;

        command.stdout(Stdio::inherit()).stderr(Stdio::inherit());

        let mut child = command.spawn()?;

        Ok(child.wait().await?)
    }
}

struct WrappedCommand {
    executable: String,
    arguments: Vec<String>,
    arg_file: Option<Utf8PathBuf>,
}

impl WrappedCommand {
    fn new(executable: impl ToString) -> Self {
        Self {
            executable: executable.to_string(),
            arguments: vec![],
            arg_file: None,
        }
    }

    fn arg_file(&mut self, path: Utf8PathBuf) {
        self.arg_file = Some(path);
    }

    pub fn arg(&mut self, arg: impl ToString) -> &mut Self {
        let arg = arg.to_string();
        self.arguments.push(arg);
        self
    }

    pub fn args<S: ToString>(&mut self, args: impl Iterator<Item = S>) -> &mut Self {
        for arg in args {
            let arg = arg.to_string();
            self.arguments.push(arg);
        }
        self
    }

    pub async fn command(self) -> anyhow::Result<tokio::process::Command> {
        let mut cmd = tokio::process::Command::new(self.executable);

        if let Some(file) = self.arg_file {
            let content = self.arguments.join("\n");
            if file.exists() {
                tokio::fs::remove_file(&file).await?;
            }
            tokio::fs::write(&file, content.as_bytes()).await?;
            cmd.arg(format!("@{file}"));
        } else {
            cmd.args(self.arguments);
        }

        Ok(cmd)
    }
}
