mod cli_test {
    use dexterous_developer_test_utils::{recv_exit, recv_std, setup_test, InMessage};
    use test_temp_dir::*;
    use tracing_test::traced_test;

    #[traced_test]
    #[tokio::test]
    async fn simple_cli_can_run() {
        let dir = test_temp_dir!();
        let dir_path = dir.as_path_untracked().to_path_buf();
        let (comms, send, mut output, _) = setup_test(dir_path, "simple_cli").await;
        
        recv_std(&mut output, "Hey!").await.expect("Failed first line");
        println!("Trying to send input");
        let _ = send.send(InMessage::Std("\n".to_string()));
        println!("Input Sent");
        recv_std(&mut output, "Hey!").await.expect("Failed Second Line");
        let _ = send.send(InMessage::Std("exit\n".to_string()));
        recv_exit(&mut output, Some(0)).await.expect("Wrong Exit Code");
    }
}