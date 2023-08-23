#![allow(unused)]

use std::{
    error::Error,
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::Arc,
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
};

pub struct TestProject {
    path: PathBuf,
    name: String,
}

impl TestProject {
    pub fn new(template: &'static str, test: &'static str) -> Result<Self, Box<dyn Error>> {
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

        Ok(Self { path, name })
    }

    pub async fn run_cold(&mut self) -> Result<RunningProcess, Box<dyn Error>> {
        let wd = self.path.as_path();
        let mut cmd = Command::new("cargo");
        cmd.current_dir(wd).arg("run");
        self.run(cmd).await
    }

    async fn run(&mut self, mut cmd: Command) -> Result<RunningProcess, Box<dyn Error>> {
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        println!("Running:{cmd:?}");

        let (read_tx, read_rx) = broadcast::channel(1000);
        let (write_tx, write_rx) = broadcast::channel::<LineIn>(5);

        let handle = {
            let mut child = cmd.spawn()?;
            let out = child.stdout.take().ok_or("Couldn't get std out")?;
            let mut stdin = child.stdin.take().ok_or("Couldn't get std in")?;
            let childerr = child.stderr.take().ok_or("Couldn't get std err")?;

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
                                    read_tx.send(Line::Err(v));
                                },
                                _ => {
                                    return;
                                },
                            }
                        }
                        val =  child.wait() => {
                            read_tx.send(Line::Ended(Arc::new(val)));
                            return;
                        }
                        val = write_rx.recv() => {
                            match val {
                                Ok(v) => {
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
            handle: handle,
            read: read_rx,
            read_sender: read_tx,
            write: write_tx,
        })
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.path.as_path());
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
    handle: JoinHandle<()>,
    read: broadcast::Receiver<Line>,
    read_sender: broadcast::Sender<Line>,
    write: broadcast::Sender<LineIn>,
}

impl RunningProcess {
    pub fn send(&self, msg: impl ToString) -> Result<usize, SendError<LineIn>> {
        self.write.send(LineIn(msg.to_string()))
    }

    pub async fn read_next_line(&mut self) -> Result<Line, RecvError> {
        self.read.recv().await
    }
}
