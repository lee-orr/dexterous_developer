use builder::{TestBuilder, TestBuilderComms};
use camino::Utf8PathBuf;
use dexterous_developer_manager::server::run_test_server;
use std::sync::Arc;
use std::{
    env::current_exe,
    path::PathBuf,
    process::{ExitStatus, Stdio},
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    sync::mpsc::{self, UnboundedReceiver},
    task::JoinHandle,
};

pub mod builder;

pub enum InMessage {
    Std(String),
    Exit,
}

pub enum OutMessage {
    Std(String),
    Err(String),
    Exit(ExitStatus),
}

pub async fn setup_test(
    dir_path: PathBuf,
    test_example: impl ToString,
) -> (
    TestBuilderComms,
    mpsc::UnboundedSender<InMessage>,
    mpsc::UnboundedReceiver<OutMessage>,
    (JoinHandle<()>, JoinHandle<()>),
) {
    let (builder, mut comms) = TestBuilder::new(None, None);
    let manager = dexterous_developer_manager::Manager::default()
        .add_builders(&[Arc::new(builder)])
        .await;
    let (port_tx, port_rx) = tokio::sync::oneshot::channel();

    let server = tokio::spawn(async move {
        run_test_server(0, manager, port_tx).await.unwrap();
        eprintln!("Done?");
    });

    let port = port_rx.await.unwrap();
    comms.set_new_library(test_example.to_string());

    let (command_tx, mut command_rx) = mpsc::unbounded_channel();
    let (out_tx, out_rx) = mpsc::unbounded_channel();

    let runner = tokio::spawn(async move {
        let base = Utf8PathBuf::from_path_buf(current_exe().unwrap()).unwrap();
        #[cfg(target_os = "windows")]
        let runner: Utf8PathBuf = base
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("dexterous_developer_runner.exe");
        #[cfg(not(target_os = "windows"))]
        let runner: Utf8PathBuf = base
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("dexterous_developer_runner");

        let mut command = Command::new(runner);
        command
            .current_dir(dir_path)
            .arg("-s")
            .arg(format!("http://127.0.0.1:{}", port))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped());

        let mut child = command.spawn().unwrap();
        let Some(out) = child.stdout.take() else {
            panic!("No STD Out");
        };

        let mut out = BufReader::new(out).lines();

        let Some(err) = child.stderr.take() else {
            panic!("No STD Err");
        };

        let mut err = BufReader::new(err).lines();
        let Some(mut input) = child.stdin.take() else {
            panic!("No STD In");
        };

        loop {
            tokio::select! {
                Ok(Some(line)) = out.next_line() => {
                    out_tx.send(OutMessage::Std(line.clone())).unwrap();
                }
                Ok(Some(line)) = err.next_line() => {
                    out_tx.send(OutMessage::Err(line.clone())).unwrap();
                }
                Ok(status) = child.wait() => {
                    out_tx.send(OutMessage::Exit(status)).unwrap();
                    break;
                }
                Some(command) = command_rx.recv() => {
                    match command {
                        InMessage::Std(value) => {
                            println!("STD IN ({port}): {value}");
                            input.write_all(value.as_bytes()).await.unwrap();
                        }
                        InMessage::Exit => {
                            break;
                        }
                    };
                }
                else => {
                    break;
                }
            }
        }

        child.kill().await.unwrap();
    });

    (comms, command_tx, out_rx, (server, runner))
}

pub async fn recv_std(
    output: &mut UnboundedReceiver<OutMessage>,
    value: impl ToString,
) -> Result<(), String> {
    tokio::time::timeout(Duration::from_secs(20), async {
        let value = value.to_string().trim().to_string();
        while let Some(out) = output.recv().await {
            match out {
                OutMessage::Std(v) => {
                    eprintln!("STDOUT: {v}");
                    if v.contains(&value) {
                        eprintln!("FOUND STDOUT");
                        return Ok(());
                    }
                }
                OutMessage::Err(_) => {}
                OutMessage::Exit(_) => return Err(format!("Exited While Waiting for {}", value)),
            }
        }
        Err("Got to exit without sucess".to_string())
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|val| val)
}

pub async fn recv_err(
    output: &mut UnboundedReceiver<OutMessage>,
    value: impl ToString,
) -> Result<(), String> {
    tokio::time::timeout(Duration::from_secs(20), async {
        let value = value.to_string().trim().to_string();
        while let Some(out) = output.recv().await {
            match out {
                OutMessage::Err(v) => {
                    eprintln!("STDERR: {v}");
                    if v.contains(&value) {
                        eprintln!("FOUND STDERR");
                        return Ok(());
                    }
                }
                OutMessage::Std(_) => {}
                OutMessage::Exit(_) => return Err(format!("Exited While Waiting for {}", value)),
            }
        }
        Err("Got to exit without sucess".to_string())
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|val| val)
}

pub async fn recv_exit(
    output: &mut UnboundedReceiver<OutMessage>,
    value: Option<i32>,
) -> Result<(), String> {
    tokio::time::timeout(Duration::from_secs(20), async {
        while let Some(out) = output.recv().await {
            if let OutMessage::Exit(code) = out {
                let code = code.code();
                if code == value {
                    return Ok(());
                } else {
                    return Err(format!("Expected exit {value:?} - got {code:?}"));
                }
            }
        }
        Err("Got to exit without sucess".to_string())
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|val| val)
}
