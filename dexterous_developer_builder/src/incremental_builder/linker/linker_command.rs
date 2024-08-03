use anyhow::{bail, Context};
use camino::Utf8PathBuf;
use tokio::io::AsyncWriteExt;

pub struct LinkerCommand {
    pub linker: Linker,
    pub arguments: Vec<String>,
    pub output: Option<String>,
    pub linker_arguments_file: Option<Utf8PathBuf>,   
}

#[derive(PartialEq, Eq, Debug)]
pub enum Linker {
    Windows,
    Apple,
    Linux
}

impl LinkerCommand {
    pub async fn new(target: String, args: Vec<String>) -> anyhow::Result<Self> {
        let linker = if target.contains("windows") {
            Linker::Windows
        } else if target.contains("apple") {
            Linker::Apple
        } else {
            Linker::Linux
        };

        let (arguments, output, linker_arguments_file) = process_arguments(&target, args).await?;

        Ok(Self {
            linker,
            arguments,
            output,
            linker_arguments_file
        })
    }

    pub fn is_main_package(&self, package_name: &str) -> bool {
        if let Some(output) = &self.output {
            output.contains(package_name)
        } else {
            false
        }
    }

    pub fn convert_executable_to_library(mut self) -> Self {
        let mut args = self.arguments.into_iter().filter(|v| !(v.contains("--gc-sections") || v.contains("-pie") || v.contains("--version-script"))).collect::<Vec<_>>();

        if self.linker != Linker::Windows {
            if !args.contains(&"-shared".to_owned()) {
                args.push("-shared".to_owned());
            }
        
            if !args.contains(&"-rdynamic".to_owned()) {
                args.push("-rdynamic".to_owned());
            }
        }

        self.arguments = args;
        self
    }

    pub fn add_libraries(&mut self, lib_directories: Vec<Utf8PathBuf>) {
        if self.linker == Linker::Windows {
            for dir in lib_directories.iter().rev() {
                self.arguments.push(format!("/LIBPATH:{dir}"));
            }
        } else {
            for dir in lib_directories.iter().rev() {
                self.arguments.push("-L".to_string());
                self.arguments.push(dir.to_string());
            }
        }
    }

    pub fn convert_to_patch(mut self, id: u32, previous_versions: Vec<String>) -> Self {
        let mut arg_iter = self.arguments.iter();

        let mut object_files: Vec<String> = vec![];
        let mut arch = None;
        let mut include_args: Vec<String> = vec![];

        if self.linker != Linker::Windows {
            while let Some(arg) = arg_iter.next() {
                if *arg == "-arch" {
                    arch = arg_iter.next().cloned();
                } else if arg == "-L" {
                    if let Some(arg) = arg_iter.next() {
                        include_args.push("-L".to_string());
                        include_args.push(arg.clone());
                    }
                } else if arg.contains('=') || arg.starts_with("-l") {
                    include_args.push(arg.clone());
                } else if arg.ends_with(".o") && !arg.contains("symbols.o") { 
                    object_files.push(arg.clone());
                } else if arg == "-target" {
                    if let Some(arg) = arg_iter.next() {
                        include_args.push("-target".to_string());
                        include_args.push(arg.clone());
                    }
                }
            }


            let mut args = include_args;

            if self.linker == Linker::Apple {
                args.push("-undefined".to_string());
                args.push("dynamic_lookup".to_string());
                args.push("-dylib".to_string());
                args.push("-shared".to_string());
                args.push("-rdynamic".to_string());
            } else {
                if !args.contains(&"-shared".to_owned()) {
                    args.push("-shared".to_string());
                    args.push("-rdynamic".to_string());
                }
                args.push("-fvisibility=default".to_string());
            }

            args.push("-nodefaultlibs".to_string());
            args.push("-fPIC".to_string());

            if let Some(arch) = &arch {
                args.push("-arch".to_string());
                args.push(arch.clone());
            }

            for name in previous_versions.iter().rev() {
                if !name.ends_with(&format!(".{id}")) {
                    args.push(format!("-l{name}"));
                }
            }

            for file in &object_files {
                args.push(file.clone());
            }
            self.arguments = args;
        } else {
            todo!();
            while let Some(arg) = arg_iter.next() {
                if arg.starts_with("/LIBPATH:") {
                        include_args.push(arg.trim_start_matches("/LIBPATH:").to_string());
                } else if arg.contains('=') || arg.starts_with("-l") {
                    include_args.push(arg.clone());
                } else if arg.ends_with(".o") && !arg.contains("symbols.o") { 
                    object_files.push(arg.clone());
                } else if arg == "-target" {
                    if let Some(arg) = arg_iter.next() {
                        include_args.push("-target".to_string());
                        include_args.push(arg.clone());
                    }
                }
            }
        }
        self
    }

    pub async fn execute(mut self) -> anyhow::Result<()> {
        let linker_exec = if self.linker == Linker::Windows {
            "rust_lld.exe"
        } else {
            "cc"
        };

        if let Some(output) = &self.output {
            if self.linker == Linker::Windows {
                self.arguments.push(format!("/OUT:{output}"));
            } else {
                self.arguments.push("-o".to_string());
                self.arguments.push(output.clone());
            }
        }

        eprintln!("LINKING - {linker_exec} - {:?}", self.output);
        eprintln!("{}", self.arguments.join("\n"));

        let mut command = tokio::process::Command::new(linker_exec);

        if self.linker == Linker::Windows {
            command.arg("-flavor").arg("link");
        }

        if let Some(path) = &self.linker_arguments_file {
            tokio::fs::remove_file(&path).await?;
            let mut file = tokio::fs::File::create(&path).await?;
            file.write_all(self.arguments.join("\n").as_bytes()).await?;
            let arg = format!(
                "@{}",
                Utf8PathBuf::from_path_buf(dunce::canonicalize(path)?)
                    .map_err(|v| anyhow::anyhow!("{v:?}"))?
            );

            command.arg(arg);
        } else {
            command.args(&self.arguments);
        }

        let result = command.output().await?;

        if !result.status.success() {
            eprintln!("Failed to Link\n{}", std::str::from_utf8(&result.stderr).unwrap_or_default());
            std::process::exit(1);
        }
        std::process::exit(0);
    }
}

async fn process_arguments(target: &str, args: Vec<String>) -> anyhow::Result<(Vec<String>, Option<String>, Option<Utf8PathBuf>)> {
    let path = if let Some(file) = args.first() {
        if file.starts_with('@') && file.ends_with("linker-arguments") {
            let path = file.trim_start_matches('@');
            let path = Utf8PathBuf::from(path);
            println!("Have the file path");
            if path.exists() {
                Some(path)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let args = if let Some(path) = &path {
        let file = tokio::fs::read(&path).await?;
        let file = if target.contains("msvc") {
            if file[0..2] != [255, 254] {
                bail!(
                    "linker response file `{}` didn't start with a utf16 BOM",
                    &path
                );
            }
            let content_utf16: Vec<u16> = file[2..]
                .chunks_exact(2)
                .map(|a| u16::from_ne_bytes([a[0], a[1]]))
                .collect();
            String::from_utf16(&content_utf16).with_context(|| {
                format!(
                    "linker response file `{}` didn't contain valid utf16 content",
                    &path
                )
            })?
        } else {
            String::from_utf8(file)?
        };
        file.lines().map(|v| v.to_owned()).collect()
    } else {
        args.to_vec()
    };

    let mut new_args = vec![];
    let mut iterator = args.into_iter().filter(|v| {
        !v.contains("dexterous_developer_incremental_linker")
            && !v.contains("incremental_c_compiler")
    });
    let mut output = None;
    while let Some(arg) = iterator.next() {
        if arg == "-o" {
            output = iterator.next();
        } else if arg.starts_with("/OUT:") {
            output = Some(arg.trim_start_matches("/OUT:").to_string());
        } else {
            new_args.push(arg);
        }
    }

    Ok((new_args, output, path))
}