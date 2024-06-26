mod cli_test {
    use dexterous_developer_test_utils::{recv_err, recv_exit, recv_std, setup_test, InMessage};
    use test_temp_dir::*;
    use tracing_test::traced_test;

    #[traced_test]
    #[tokio::test]
    async fn simple_cli_can_run() {
        let dir = test_temp_dir!();
        let dir_path = dir.as_path_untracked().to_path_buf();
        let (comms, send, mut output, _) = setup_test(dir_path, "simple_cli").await;
        
        recv_std(&mut output, "Hey!").await.expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "Hey!").await.expect("Failed Second Line");
        let _ = send.send(InMessage::Std("exit\n".to_string()));
        recv_exit(&mut output, Some(0)).await.expect("Wrong Exit Code");
    }

    #[traced_test]
    #[tokio::test]
    async fn can_swap_a_system() {
        let dir = test_temp_dir!();
        let dir_path = dir.as_path_untracked().to_path_buf();
        let (mut comms, send, mut output, _) = setup_test(dir_path, "simple_cli").await;
        
        recv_std(&mut output, "Hey!").await.expect("Failed first line");
        comms.set_new_library("simple_system_swap");
        recv_std(&mut output, "update_callback_internal").await.expect("Didn't Get Download");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "Swapped Update System!").await.expect("Failed Swapped Line");
        let _ = send.send(InMessage::Std("exit\n".to_string()));
        recv_exit(&mut output, Some(0)).await.expect("Wrong Exit Code");
    }
}