use builder::{TestBuilderComms, TestBuilderInitializer};
use camino::Utf8PathBuf;
use dexterous_developer_manager::server::run_test_server;
use std::{
    env::current_exe,
    process::{ExitStatus, Stdio},
    time::Duration,
};
use tokio::sync::mpsc::UnboundedSender;
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
    test_example: impl ToString,
) -> (
    TestBuilderComms,
    mpsc::UnboundedSender<InMessage>,
    mpsc::UnboundedReceiver<OutMessage>,
    (JoinHandle<()>, JoinHandle<()>),
) {
    let manager = dexterous_developer_manager::Manager::default();
    let (builder, mut comms) =
        TestBuilderInitializer::new(None, None, manager.get_watcher_channel());
    let manager = manager
        .add_builder(builder)
        .expect("Failed to set up builder");
    let (port_tx, port_rx) = tokio::sync::oneshot::channel();

    let server = tokio::spawn(async move {
        run_test_server(0, manager, port_tx).await.unwrap();
        eprintln!("Done?");
    });

    let port = port_rx.await.unwrap();
    comms.set_new_library(test_example.to_string());
    let target_directory = comms.target_directory.clone();

    let (command_tx, mut command_rx) = mpsc::unbounded_channel();
    let (out_tx, mut out_rx) = mpsc::unbounded_channel();

    let runner = tokio::spawn(async move {
        let runner = which::which("dexterous_developer_incremental_runner").unwrap();        

        let mut command = Command::new(runner);
        command
            .env(
                "RUST_LOG",
                "trace,dexterous_developer_runner=trace,dexterous_developer_dylib_runner=trace",
            )
            .arg("-s")
            .arg(format!("http://127.0.0.1:{}", port))
            .arg("--in-workspace")
            .arg("--library-path")
            .arg(target_directory)
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
                            eprintln!("STD IN ({port}): {value}");
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

    recv_std(
        &mut out_rx,
        "dexterous_developer_dylib_runner::remote_connection",
    )
    .await
    .expect("Failed to Setup Remote Connection");
    recv_std(&mut out_rx, "Got Initial State")
        .await
        .expect("Didn't get initial state");
    recv_std(&mut out_rx, "all downloads completed")
        .await
        .expect("Didn't complete downloads");
    recv_std(&mut out_rx, "Loading Initial Root")
        .await
        .expect("Didn't start loading initial root");
    recv_std(&mut out_rx, "Calling Internal Main")
        .await
        .expect("Didn't call internal main");
    recv_std(&mut out_rx, "reload complete")
        .await
        .expect("Didn't complete reload");

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

pub async fn recv_std_avoiding(
    output: &mut UnboundedReceiver<OutMessage>,
    value: impl ToString,
    avoiding: &[impl ToString],
) -> Result<(), String> {
    let avoiding = avoiding.iter().map(|v| v.to_string()).collect::<Vec<_>>();
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
                    for avoid in &avoiding {
                        if v.contains(avoid.as_str()) {
                            eprintln!("Didn't avoid {avoid}");
                            return Err(format!("Didn't avoid {avoid} - {v}"));
                        }
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

pub async fn replace_library(
    name: impl ToString,
    comms: &mut TestBuilderComms,
    output: &mut UnboundedReceiver<OutMessage>,
    send: &UnboundedSender<InMessage>,
) {
    comms.set_new_library(name);
    recv_std(output, "Received Hot Reload Message: BuildStart")
        .await
        .expect("Build didn't start");
    recv_std(output, "Received Hot Reload Message: BuildCompleted")
        .await
        .expect("Build didn't complete");
    recv_std(output, "all downloads completed")
        .await
        .expect("didn't complete all downloads");
    recv_std(output, "Preparing to call update_callback_internal")
        .await
        .expect("didn't call update callback");

    eprintln!("RUNNING THE UPDATE CALLBACK");

    let _ = send.send(InMessage::Std("\n".to_string()));
    recv_std(output, "Swapping Libraries")
        .await
        .expect("Didn't start swap");
    recv_std(output, "Loaded library")
        .await
        .expect("didn't load library");
    recv_std(output, "reload complete")
        .await
        .expect("didn't complete reload");
}
