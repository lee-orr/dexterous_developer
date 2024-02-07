#![allow(unused)]

use std::{
    fs, path::{Path, PathBuf}, process::ExitStatus, sync::{Arc, OnceLock}, time::Duration
};

use std::process::Stdio;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
    sync::broadcast::{
        self,
        error::{RecvError, SendError},
    },
    task::JoinHandle,
    time::timeout,
};

use anyhow::{bail, Context, Error, Result};

pub struct TestProject {
    path: PathBuf,
    name: String,
    package: String,
}

static CLI_PATH: OnceLock<anyhow::Result<PathBuf>> = OnceLock::new();
static TEMPLATE_PATH: OnceLock<anyhow::Result<PathBuf>> = OnceLock::new();

fn rebuild_cli() -> anyhow::Result<&'static PathBuf> {
    CLI_PATH
        .get_or_init(|| {
            if let Ok(path) = std::env::var("DEXTEROUS_CLI_PATH") {
                let mut path = PathBuf::from(&path);

                #[cfg(target_os = "windows")]
                {
                    path.set_extension("exe");
                }
                return Ok(path);
            }

            let mut cwd = std::env::current_dir()?;

            let mut root = cwd.clone();

            let mut cli_path = root.clone();

            cli_path.push("target");
            cli_path.push("debug");
            cli_path.push("dexterous_developer_cli");

            #[cfg(target_os = "windows")]
            {
                cli_path.set_extension("exe");
            }
            println!("Building Cli at {cli_path:?} from {root:?}");
            std::process::Command::new("cargo")
                .current_dir(root.as_path())
                .arg("build")
                .arg("-p")
                .arg("dexterous_developer_cli")
                .status()?;
            Ok(cli_path)
        })
        .as_ref()
        .map_err(|e| anyhow::Error::msg(e.to_string()))
}

fn template_path() -> anyhow::Result<&'static PathBuf> {
    TEMPLATE_PATH
        .get_or_init(|| {
            if let Ok(path) = std::env::var("DEXTEROUS_TESTER_PATH") {
                return Ok(PathBuf::from(&path));
            }

            let mut cwd = std::env::current_dir()?;
            Ok(cwd)
        })
        .as_ref()
        .map_err(|e| anyhow::Error::msg(e.to_string()))
}

impl TestProject {
    pub fn new(template: &'static str, test: &'static str) -> anyhow::Result<Self> {
        let mut cwd = template_path()?;


        let mut template_path = cwd.clone();
        template_path.push("testing");
        template_path.push("templates");
        template_path.push(template);

        if !template_path.exists() {
            panic!("{template_path:?} does not exist");
        }

        let name = test.to_string();
        
        let package = format!("tmp_{name}");
        let mut path = cwd.clone();
        path.push("testing");
        path.push("tmp");

        if !path.exists() {
            std::fs::create_dir(path.as_path());
        }

        path.push(&format!("tmp_{name}"));

        if path.exists() {
            std::fs::remove_dir_all(path.clone());
        }

        println!("copying from {template_path:?} to {path:?}");

        std::process::Command::new("cp")
            .arg("-R")
            .arg(template_path.as_os_str())
            .arg(path.as_os_str())
            .output()?;

        let target_dir = path.join("target");

        if target_dir.exists() {
            std::fs::remove_dir_all(target_dir);
        }

        let lock = path.join("Cargo.lock");

        if lock.exists() {
            std::fs::remove_file(lock);
        }

        if path.join("Cargo.toml").exists() {
            let path =  path.join("Cargo.toml");
            let file = std::fs::read_to_string(&path)?;
            let file = file.lines().map(|line| if line.starts_with("name") { format!("name = \"{package}\"")} else { line.to_string() }).collect::<Vec<_>>().join("\n");
            std::fs::write(path, file)?;
        }

        
        if path.join("src").join("main.rs").exists() {
            let path =  path.join("src").join("main.rs");
            let file = std::fs::read_to_string(&path)?;
            let file = file.lines().map(|line| if line.contains("::bevy_main();") { format!("{package}::bevy_main();")} else { line.to_string() }).collect::<Vec<_>>().join("\n");
            std::fs::write(path, file)?;
        }

        Ok(Self {
            path,
            name,
            package,
        })
    }

    pub fn existing_project(path: &Path, name: &'static str) -> anyhow::Result<Self> {
        std::fs::create_dir_all(path.join("assets"))?;
        Ok(Self {
            path: path.to_path_buf(),
            name: name.to_string(),
            package: name.to_string(),
        })
    }

    pub fn write_file(&self, path: &Path, content: &str) -> anyhow::Result<()> {
        let mut file_path = self.path.clone();
        file_path.push(path);
        println!("Writing to {path:?}");
        std::fs::write(file_path.as_path(), content)?;
        println!("Written");
        Ok(())
    }

    pub async fn run_cold(&mut self) -> anyhow::Result<RunningProcess> {
        let wd = self.path.as_path();
        let mut cmd = Command::new("cargo");
        cmd.current_dir(wd).arg("run");
        self.run(cmd, ProcessHeat::Cold).await
    }

    pub async fn run_hot_cli(&mut self) -> anyhow::Result<RunningProcess> {
        let Ok(cli_path) = rebuild_cli() else {
            bail!("Couldn't get CLI");
        };

        let mut wd = self.path.clone();
        let mut cmd: Command = Command::new(cli_path);
        cmd.current_dir(&wd).arg("run").arg("-p").arg(&self.package);
        self.run(cmd, ProcessHeat::Hot).await
    }

    pub async fn run_example(&mut self, example: &str) -> anyhow::Result<RunningProcess> {
        let Ok(cli_path) = rebuild_cli() else {
            bail!("Couldn't get CLI");
        };

        let mut wd = self.path.clone();
        let mut cmd: Command = Command::new(cli_path);
        cmd.current_dir(&wd)
            .arg("run")
            .arg("--example")
            .arg(example);
        self.run(cmd, ProcessHeat::Hot).await
    }

    pub async fn run_existing(&mut self) -> anyhow::Result<RunningProcess> {
        let Ok(cli_path) = rebuild_cli() else {
            bail!("Couldn't get CLI");
        };

        let mut wd = self.path.clone();
        let mut cmd: Command = Command::new(cli_path);
        cmd.current_dir(&wd).arg("run-existing").arg(&self.path);
        self.run(cmd, ProcessHeat::Cold).await
    }

    pub async fn run_host_cli(&mut self, port: &str) -> anyhow::Result<RunningProcess> {
        let Ok(cli_path) = rebuild_cli() else {
            bail!("Couldn't get CLI");
        };

        let mut wd = self.path.clone();
        let mut cmd: Command = Command::new(cli_path);
        cmd.current_dir(&wd)
            .arg("serve")
            .arg("-p")
            .arg(&self.package)
            .arg(port);
        self.run(cmd, ProcessHeat::Host).await
    }

    pub async fn run_client_cli(&mut self, port: &str) -> anyhow::Result<RunningProcess> {
        let Ok(cli_path) = rebuild_cli() else {
            bail!("Couldn't get CLI");
        };

        let mut wd = self.path.clone();
        let mut cmd: Command = Command::new(cli_path);
        cmd.current_dir(&wd)
            .arg("remote")
            .arg(&format!("http://localhost:{port}"));
        self.run(cmd, ProcessHeat::Remote).await
    }

    async fn run(
        &mut self,
        mut cmd: Command,
        is_hot: ProcessHeat,
    ) -> anyhow::Result<RunningProcess> {
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env(
                "RUST_LOG",
                "warn,dexterous_developer_internal=trace,dexterous_developer_cli=trace,bevy_dexterous_developer=trace",
            )
            .kill_on_drop(true);
        println!("Running:{cmd:?}");

        let (read_tx, read_rx) = broadcast::channel(1000);
        let (write_tx, write_rx) = broadcast::channel::<LineIn>(5);

        let handle = {
            let name = self.name.clone();
            let mut child = cmd.spawn()?;
            let out = child.stdout.take().context("Couldn't get std out")?;
            let mut stdin = child.stdin.take().context("Couldn't get std in")?;
            let childerr = child.stderr.take().context("Couldn't get std err")?;

            let read_tx = read_tx.clone();
            let mut out_reader = BufReader::new(out).lines();
            let mut err_reader = BufReader::new(childerr).lines();
            let mut write_rx = write_rx;

            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        val = out_reader.next_line() => {
                            match val {
                                Ok(Some(v)) => {
                                    println!("{name} > {v}");
                                    read_tx.send(Line::Std(v));
                                },
                                Ok(None) => {
                                    println!("{name} __ GOT EMPTY READ __");
                                    read_tx.send(Line::Ended(Arc::new(Err(anyhow::Error::msg("Got Empty STD Read")))));
                                    return;
                                },
                                Err(e) => {
                                    println!("{name} > Got Error in Reader");
                                    read_tx.send(Line::Ended(Arc::new(Err(anyhow::Error::msg("Got Error in Reader")))));
                                    return;
                                }
                            }
                        }
                        val = err_reader.next_line() => {
                            match val {
                                Ok(Some(v)) => {
                                    println!("{name} !> {v:?}");
                                    read_tx.send(Line::Err(v));
                                },
                                Ok(None) => {
                                    println!("{name} __ GOT EMPTY STD ERR READ __");
                                    read_tx.send(Line::Ended(Arc::new(Err(anyhow::Error::msg("Got Empty STD ERR Read")))));
                                    return;
                                },
                                Err(e) => {
                                    println!("{name} !> Got Error in Err Reader");
                                    read_tx.send(Line::Ended(Arc::new(Err(anyhow::Error::msg("Got Error in Err Reader")))));
                                    return;
                                }
                            }
                        }
                        val =  child.wait() => {
                            println!("{name} ended");
                            read_tx.send(Line::Ended(Arc::new(val.context("Ended Process"))));
                            return;
                        }
                        val = write_rx.recv() => {
                            match val {
                                Ok(v) => {
                                    println!("{name} ~ {}", v.0);
                                    stdin.write_all(v.0.as_bytes()).await.expect("Couldn't write to std in");
                                },
                                Err(_) => {
                                    return;
                                },
                            }
                        }
                    }
                }
            })
        };

        Ok(RunningProcess {
            name: self.name.clone(),
            handle: Some(handle),
            read: read_rx,
            read_sender: read_tx,
            write: write_tx,
            is_hot,
        })
    }
}

#[derive(Clone, Debug)]
pub enum Line {
    Std(String),
    Err(String),
    Ended(Arc<anyhow::Result<ExitStatus>>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineIn(String);

pub struct RunningProcess {
    name: String,
    handle: Option<JoinHandle<()>>,
    read: broadcast::Receiver<Line>,
    read_sender: broadcast::Sender<Line>,
    write: broadcast::Sender<LineIn>,
    is_hot: ProcessHeat,
}

pub enum ProcessHeat {
    Cold,
    Hot,
    Remote,
    Host,
}

impl RunningProcess {
    pub fn send(&self, msg: impl ToString) -> Result<usize, SendError<LineIn>> {
        println!("Sending {} -> {}", self.name, msg.to_string());
        self.write.send(LineIn(msg.to_string()))
    }

    pub async fn read_next_line(&mut self) -> Result<Line, RecvError> {
        self.read.recv().await
    }

    pub async fn is_ready(&mut self) {
        println!("Checking Readiness");
        match self.is_hot {
            ProcessHeat::Hot => {
                let mut is_watching = false;
                let mut reload_complete = false;
                loop {
                    match self.read_next_line().await.expect("No Next Line") {
                        Line::Std(line) | Line::Err(line) => {
                            if line.contains("Executing first run") {
                                reload_complete = true;
                            }
                            if line.contains("Watching...") {
                                is_watching = true;
                            }
                            if reload_complete && is_watching {
                                break;
                            }
                        }

                        Line::Ended(v) => {
                            panic!("Ended - {v:?}");
                        }
                    };
                }
            }
            ProcessHeat::Cold => loop {
                match self.read_next_line().await.expect("no Next Line") {
                    Line::Std(line) | Line::Err(line) => {
                        if line.contains("Running") {
                            break;
                        }
                    }

                    Line::Ended(v) => {
                        panic!("Ended - {v:?}");
                    }
                };
            },
            ProcessHeat::Remote => {
                let mut is_watching = false;
                let mut reload_complete = false;
                loop {
                    match self.read_next_line().await.expect("No Next Line") {
                        Line::Std(line) | Line::Err(line) => {
                            if line.contains("Executing first run") {
                                reload_complete = true;
                            }
                            if line.contains("Calling Watch Function") {
                                is_watching = true;
                            }
                            if reload_complete && is_watching {
                                break;
                            }
                        }

                        Line::Ended(v) => {
                            panic!("Ended - {v:?}");
                        }
                    };
                }
            }
            ProcessHeat::Host => {
                self.send("\n").expect("Failed to send empty line");
                loop {
                    match self.read_next_line().await.expect("No Next Line") {
                        Line::Std(line) | Line::Err(line) => {
                            if line.contains("Build completed") {
                                break;
                            }
                        }

                        Line::Ended(v) => {
                            panic!("Ended - {v:?}");
                        }
                    };
                }
            }
        }
        println!("Ready");
    }

    pub async fn has_updated(&mut self) {
        println!("Awaiting hot reload");
        match self.is_hot {
            ProcessHeat::Hot => {
                self.send("\n").expect("Failed to send empty line");
                loop {
                    match self.read_next_line().await.expect("No Next Line") {
                        Line::Std(line) | Line::Err(line) => {
                            if line.contains("Build completed") {
                                break;
                            }
                        }

                        Line::Ended(v) => {
                            panic!("Ended - {v:?}");
                        }
                    };
                }
                self.send("\n").expect("Failed to send empty line");
                loop {
                    match self.read_next_line().await.expect("No Next Line") {
                        Line::Std(line) | Line::Err(line) => {
                            if line.contains("reload complete") {
                                break;
                            }
                            println!("got while waiting {line} for \"reload complete\"");
                        }

                        Line::Ended(v) => {
                            panic!("Ended - {v:?}");
                        }
                    };
                }
            }
            ProcessHeat::Cold => panic!("Not a hot reloadable attempt"),
            ProcessHeat::Remote => {
                loop {
                    match self.read_next_line().await.expect("No Next Line") {
                        Line::Std(line) | Line::Err(line) => {
                            if line.contains("Updated Files Downloaded") {
                                break;
                            }
                        }

                        Line::Ended(v) => {
                            panic!("Ended - {v:?}");
                        }
                    };
                }

                self.send("\n").expect("Failed to send empty line");
                loop {
                    match self.read_next_line().await.expect("No Next Line") {
                        Line::Std(v) => {
                            println!("Got a line while waiting {v} for \"reload complete\"");
                        }

                        Line::Err(line) => {
                            if line.contains("reload complete") {
                                break;
                            }
                            println!("got an err while waiting {line} for \"reload complete\"");
                        }

                        Line::Ended(v) => {
                            panic!("Ended - {v:?}");
                        }
                    };
                }
            }
            ProcessHeat::Host => {
                self.send("\n").expect("Failed to send empty line");
                loop {
                    match self.read_next_line().await.expect("No Next Line") {
                        Line::Std(line) | Line::Err(line) => {
                            if line.contains("Build completed") {
                                break;
                            }
                        }

                        Line::Ended(v) => {
                            panic!("Ended - {v:?}");
                        }
                    };
                }
            }
        }
        println!("Successfully hot reloaded");
    }

    pub async fn next_line_contains_with_error(
        &mut self,
        value: impl ToString,
        error: impl ToString,
    ) {
        let Ok(Line::Std(line)) = self.read_next_line().await else {
            panic!("Should have gotten a line");
        };

        let value = value.to_string();

        if !line.contains(&value) {
            let error = error.to_string();
            panic!("Line {line} does not contain {value}\n{error}")
        }
    }

    pub async fn next_line_contains(&mut self, value: impl ToString) {
        self.next_line_contains("Exiting");
    }

    pub async fn exit(mut self) {
        self.send("exit\n").expect("Failed to send line");
        self.next_line_contains("Exiting");
        println!("Exiting");
        self.handle = None;
        tokio::time::sleep(Duration::from_secs_f32(0.1)).await;
        println!("Awaited exit");
    }

    pub async fn wait_for_lines(&mut self, value: &[&str]) {
        let mut iterator = value.iter();
        let mut current = iterator.next();
        while let Some(c) = current {
            println!("{} - Waiting for {c}", self.name);
            match self.read_next_line().await.expect("No Next Line") {
                Line::Std(line) => {
                    if line.contains("KeepAlive") {
                        println!("Keepalive detected - probably need new input?");
                        self.send("\n").expect("Failed to send empty line");
                    }
                    if line.contains(c) {
                        println!("Got line {line} matching {c}");
                        current = iterator.next();
                    }
                }

                Line::Err(line) => {
                    if line == "No Asset" {
                        println!("Waiting for asset...");
                        tokio::time::sleep(Duration::from_secs_f32(5.)).await;
                        self.send("\n").expect("Failed to send empty line");
                    }
                    continue;
                }

                Line::Ended(v) => {
                    panic!("Ended While Waiting For Line - {v:?}");
                }
            };
        }
        println!("Wait Complete");
    }
}
