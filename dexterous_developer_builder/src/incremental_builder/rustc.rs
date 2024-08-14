use std::{process::Stdio, str::FromStr};

use anyhow::{anyhow, bail};
use camino::Utf8PathBuf;
use tokio::io::{AsyncBufReadExt, BufReader};

pub async fn incremental_rustc() -> anyhow::Result<()> {
    let package_name = std::env::var("DEXTEROUS_DEVELOPER_PACKAGE_NAME")?;
    let rustc = Rustc::new(std::env::args(), &package_name)?;

    let _ = rustc.run().await?;
    Ok(())
}

#[derive(Debug)]
struct Rustc {
    executable: String,
    operation: RustcOperation,
}

#[derive(Debug)]
enum RustcOperation {
    Passthrough(Vec<String>),
    MainCompilation {
        original_args: Vec<String>,
        crate_name: String,
        edition: u32,
        file: Utf8PathBuf,
        crate_type: String,
        emit: String,
        codegen_args: Vec<String>,
        cfg: Vec<String>,
        check_cfg: Vec<String>,
        out_dir: Utf8PathBuf,
        target: String,
        search_paths: Vec<String>,
        library_links: Vec<String>,
        extern_links: Vec<String>,
    },
}

impl Rustc {
    fn new(mut args: impl Iterator<Item = String>, package: &str) -> anyhow::Result<Self> {
        let _current_executable = args.next();

        let Some(executable) = args.next() else {
            bail!("No Rustc Executable In Arguments");
        };

        let args: Vec<String> = args.collect();

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
                let mut out_dir = None;
                let mut target = None;
                let mut search_paths = vec![];
                let mut library_links = vec![];
                let mut extern_links = vec![];

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
                            codegen_args.push(a);
                        }
                    } else if arg.starts_with("-C") && arg.split_whitespace().count() == 1 {
                        codegen_args.push(arg.trim_start_matches("-C").to_string());
                    } else if arg == "--cfg" {
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
                        let path = args_iter.next().map(|v| Utf8PathBuf::from_str(&v)).ok_or(anyhow!("No Out Dir"))??;
                        out_dir = Some(path);
                    } else if arg == "--target" {
                        target = args_iter.next().cloned();
                    } else {
                        eprintln!("IGNORED: {arg}");
                    }
                }

                RustcOperation::MainCompilation {
                    crate_name: crate_name.ok_or(anyhow!("Couldn't determine crate name"))?,
                    edition,
                    file: file.ok_or(anyhow!("Couldn't determine source file"))?,
                    crate_type: crate_type.ok_or(anyhow!("Couldn't determine crate type"))?,
                    emit: emit.ok_or(anyhow!("Couldn't determin emit parameters"))?,
                    codegen_args,
                    cfg,
                    check_cfg,
                    out_dir: out_dir.ok_or(anyhow!("Couldn't determine output directory"))?,
                    target: target.ok_or(anyhow!("Couldn't determine target"))?,
                    search_paths,
                    library_links,
                    extern_links,
                    original_args: args,
                }
            }
        };

        Ok(Self {
            executable,
            operation,
        })
    }

    async fn run(self) -> anyhow::Result<std::process::ExitStatus> {
        let mut command = tokio::process::Command::new(self.executable);

        match self.operation {
            RustcOperation::MainCompilation {
                original_args,
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
            } => {
                eprintln!("RUSTC\n{}", original_args.join("\n"));
                command
                    .arg("--error-format=json")
                    .arg("--json=diagnostic-rendered-ansi,artifacts,future-incompat")
                    .arg("--crate-name")
                    .arg(crate_name)
                    .arg("--edition")
                    .arg(edition.to_string())
                    .arg(file)
                    .arg("--crate-type")
                    .arg(if crate_type.contains("dylib") { &crate_type } else { "dylib" })
                    .arg(format!("--emit={emit}"))
                    .arg("--target")
                    .arg(target)
                    .arg("--out-dir")
                    .arg(out_dir);

                for c in &codegen_args {
                    command.arg("-C").arg(c);
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
                command.args(args);
            },
        };

        command.stdout(Stdio::inherit()).stderr(Stdio::inherit());

        let mut child = command.spawn()?;

        Ok(child.wait().await?)
    }
}
