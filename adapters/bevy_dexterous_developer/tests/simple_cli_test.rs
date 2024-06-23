use std::sync::Arc;

use dexterous_developer_manager::{server::{run_test_server}, test_utils::TestBuilder};
use tokio::process::Command;



#[tokio::test]
async fn simple_cli_can_run() {
    let (builder, mut comms) = TestBuilder::new(None, None);
    let manager = dexterous_developer_manager::Manager::default().add_builders(&[Arc::new(builder)]).await;
    let (port_tx, port_rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        run_test_server(0, manager, port_tx).await.unwrap();
    });

    let port = port_rx.await.unwrap();
    comms.set_new_library("simple_cli".to_string());
}