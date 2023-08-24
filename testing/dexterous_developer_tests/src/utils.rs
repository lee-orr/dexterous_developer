#![allow(unused)]

use std::{
    error::Error,
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::Arc,
    time::Duration,
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

use anyhow::{bail, Context, Result};

pub struct TestProject {
    path: PathBuf,
    name: String,
    cli_path: PathBuf,
}

impl TestProject {
    pub fn new(template: &'static str, test: &'static str) -> anyhow::Result<Self> {
        let mut cwd = std::env::current_dir()?;
        cwd.pop();

        let mut template_path = cwd.clone();
        template_path.push(template);

        if !template_path.exists() {
            panic!("{template_path:?} does not exist");
        }

        let name = format!("tmp_{test}");
        let mut path = cwd.clone();
        path.push(&name);

        if path.exists() {
            std::fs::remove_dir_all(path.clone());
        }

        println!("copying from {template_path:?} to {path:?}");

        std::process::Command::new("cp")
            .arg("-R")
            .arg(template_path.as_os_str())
            .arg(path.as_os_str())
            .output()?;

        let mut cargo_path = path.clone();
        cargo_path.push("Cargo.toml");
        let cargo = std::fs::read_to_string(cargo_path.as_path())?;
        let cargo = cargo.replace(template, &name);
        std::fs::write(cargo_path.as_path(), cargo);

        let mut main_path = path.clone();
        main_path.push("src/main.rs");
        let main = std::fs::read_to_string(main_path.as_path())?;
        let cargo = main.replace(template, &name);
        std::fs::write(main_path.as_path(), cargo);

        let mut root = cwd.clone();
        root.pop();

        let mut cli_path = root.clone();

        cli_path.push("target");
        cli_path.push("debug");
        cli_path.push("dexterous_developer_cli");

        #[cfg(target_os = "windows")]
        {
            cli_path.set_extension("exe");
        }

        if cli_path.exists() {
            println!("Cli at {cli_path:?}");
        } else {
            println!("Building Cli at {cli_path:?} from {root:?}");
            std::process::Command::new("cargo")
                .current_dir(root.as_path())
                .arg("build")
                .arg("-p")
                .arg("dexterous_developer_cli")
                .output()?;
        }

        Ok(Self {
            path,
            name,
            cli_path,
        })
    }

    pub async fn run_cold(&mut self) -> anyhow::Result<RunningProcess> {
        let wd = self.path.as_path();
        let mut cmd = Command::new("cargo");
        cmd.current_dir(wd).arg("run");
        self.run(cmd, false).await
    }

    pub async fn run_hot_cli(&mut self) -> anyhow::Result<RunningProcess> {
        let mut wd = self.path.clone();

        let mut cmd: Command = Command::new(self.cli_path.as_os_str());
        cmd.current_dir(&wd).arg("-p").arg(&self.name);
        self.run(cmd, true).await
    }

    async fn run(&mut self, mut cmd: Command, is_hot: bool) -> anyhow::Result<RunningProcess> {
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        println!("Running:{cmd:?}");

        let (read_tx, read_rx) = broadcast::channel(1000);
        let (write_tx, write_rx) = broadcast::channel::<LineIn>(5);

        let handle = {
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
                                    println!("> {v}");
                                    read_tx.send(Line::Std(v));
                                },
                                _ => {
                                    return;
                                },
                            }
                        }
                        val = err_reader.next_line() => {
                            match val {
                                Ok(Some(v)) => {
                                    println!("!> {v:?}");
                                    read_tx.send(Line::Err(v));
                                },
                                _ => {
                                    return;
                                },
                            }
                        }
                        val =  child.wait() => {
                            println!("child ended");
                            read_tx.send(Line::Ended(Arc::new(val)));
                            return;
                        }
                        val = write_rx.recv() => {
                            match val {
                                Ok(v) => {
                                    println!("~ {}", v.0);
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
            handle: Some(handle),
            read: read_rx,
            read_sender: read_tx,
            write: write_tx,
            is_hot,
        })
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        println!(
            "Dropping {} - delete {}",
            self.name,
            self.path.to_string_lossy()
        );

        let e = std::fs::remove_dir_all(self.path.as_path());

        println!("Dropped - {e:#?}");
    }
}

#[derive(Clone, Debug)]
pub enum Line {
    Std(String),
    Err(String),
    Ended(Arc<std::io::Result<ExitStatus>>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineIn(String);

pub struct RunningProcess {
    handle: Option<JoinHandle<()>>,
    read: broadcast::Receiver<Line>,
    read_sender: broadcast::Sender<Line>,
    write: broadcast::Sender<LineIn>,
    is_hot: bool,
}

impl RunningProcess {
    pub fn send(&self, msg: impl ToString) -> Result<usize, SendError<LineIn>> {
        self.write.send(LineIn(msg.to_string()))
    }

    pub async fn read_next_line(&mut self) -> Result<Line, RecvError> {
        self.read.recv().await
    }

    pub async fn is_ready(&mut self) {
        if self.is_hot {
            loop {
                match self.read_next_line().await.expect("No Next Line") {
                    Line::Std(line) => {
                        if line.contains("Running with ") {
                            break;
                        }
                    }

                    Line::Err(line) => {
                        panic!("Error occured {line:?}");
                    }

                    Line::Ended(v) => {
                        panic!("Ended - {v:?}");
                    }
                };
            }

            loop {
                match self.read_next_line().await.expect("No Next Line") {
                    Line::Std(line) => {
                        if line.contains("reload complete") {
                            break;
                        }
                    }

                    Line::Err(_) => {}

                    Line::Ended(v) => {
                        panic!("Ended - {v:?}");
                    }
                };
            }
        } else {
            loop {
                match self.read_next_line().await.expect("no Next Line") {
                    Line::Std(line) => {
                        if line.contains("Running") {
                            break;
                        }
                    }

                    Line::Err(line) => {
                        if line.contains("Running") {
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

    pub async fn is_ready_with_timeout(&mut self) {
        timeout(Duration::from_secs_f32(120.), self.is_ready())
            .await
            .expect("Not Ready On Time");
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

    pub async fn exiting(&mut self) {
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
            match self.read_next_line().await.expect("No Next Line") {
                Line::Std(line) => {
                    if line.contains(c) {
                        println!("Got line {line} matching {c}");
                        current = iterator.next();
                    }
                }

                Line::Err(line) => {
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
