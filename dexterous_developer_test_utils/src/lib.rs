use std::process::ExitStatus;

use builder::{TestBuilder, TestBuilderComms};
use dexterous_developer_manager::server::run_test_server;
use tokio::sync::mpsc;use std::sync::Arc;


pub mod builder;

pub enum OutMessage {
    Std(String),
    Err(String),
    Exit(ExitStatus)
}

pub async fn setup_test(test_example: impl ToString) -> (TestBuilderComms, mpsc::UnboundedSender<String>, mpsc::UnboundedReceiver<OutMessage>) {

    let (builder, mut comms) = TestBuilder::new(None, None);
    let manager = dexterous_developer_manager::Manager::default().add_builders(&[Arc::new(builder)]).await;
    let (port_tx, port_rx) = tokio::sync::oneshot::channel();
    
    tokio::spawn(async move {
        run_test_server(0, manager, port_tx).await.unwrap();
    });

    let port = port_rx.await.unwrap();
    comms.set_new_library(test_example.to_string());

    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (out_tx, out_rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        
    });

    (comms, command_tx, out_rx)
}