mod component_test {
    use dexterous_developer_test_utils::{
        recv_exit, recv_std, replace_library, setup_test, InMessage,
    };
    use test_temp_dir::*;
    use tracing_test::traced_test;

    #[traced_test]
    #[tokio::test]
    async fn can_reset_events() {
        let dir = test_temp_dir!();
        let dir_path = dir.as_path_untracked().to_path_buf();

        let (mut comms, send, mut output, _) = setup_test(dir_path, "events_start").await;

        recv_std(&mut output, "Running Update")
            .await
            .expect("Failed first line");
        recv_std(&mut output, "None")
            .await
            .expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "Some(First)")
            .await
            .expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "Some(Second)")
            .await
            .expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "Some(First)")
            .await
            .expect("Failed first line");
        replace_library("events_end", &mut comms, &mut output, &send).await;
        recv_std(&mut output, "None")
            .await
            .expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "Some(A)")
            .await
            .expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "Some(B)")
            .await
            .expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "Some(A)")
            .await
            .expect("Failed first line");
        let _ = send.send(InMessage::Std("exit\n".to_string()));
        recv_exit(&mut output, Some(0))
            .await
            .expect("Wrong Exit Code");
    }
}
