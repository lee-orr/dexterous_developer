mod resource_test {
    use dexterous_developer_test_utils::{recv_exit, recv_std, setup_test, InMessage};
    use test_temp_dir::*;
    use tracing_test::traced_test;

    #[traced_test]
    #[tokio::test]
    async fn can_reset_a_resource() {
        let dir = test_temp_dir!();
        let dir_path = dir.as_path_untracked().to_path_buf();
        let (mut comms, send, mut output, _) = setup_test(dir_path, "reset_resource_start").await;

        recv_std(&mut output, "Resource Initial")
            .await
            .expect("Failed first line");
        comms.set_new_library("reset_resource_end");
        recv_std(&mut output, "update_callback_internal")
            .await
            .expect("Didn't Get Download");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "New Default")
            .await
            .expect("Failed Second Line");
        let _ = send.send(InMessage::Std("exit\n".to_string()));
        recv_exit(&mut output, Some(0))
            .await
            .expect("Wrong Exit Code");
    }

    #[traced_test]
    #[tokio::test]
    async fn can_reset_a_resource_to_value() {
        let dir = test_temp_dir!();
        let dir_path = dir.as_path_untracked().to_path_buf();

        let (mut comms, send, mut output, _) = setup_test(dir_path, "reset_resource_start").await;

        recv_std(&mut output, "Resource Initial")
            .await
            .expect("Failed first line");
        comms.set_new_library("reset_resource_to_value");
        recv_std(&mut output, "update_callback_internal")
            .await
            .expect("Didn't Get Download");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "New Value")
            .await
            .expect("Failed Second Line");
        let _ = send.send(InMessage::Std("exit\n".to_string()));
        recv_exit(&mut output, Some(0))
            .await
            .expect("Wrong Exit Code");
    }

    #[traced_test]
    #[tokio::test]
    async fn can_serialize_a_resource() {
        let dir = test_temp_dir!();
        let dir_path = dir.as_path_untracked().to_path_buf();

        let (mut comms, send, mut output, _) = setup_test(dir_path, "serde_serializable_resource_start").await;

        recv_std(&mut output, "My Serializable Field")
            .await
            .expect("Failed first line");
        comms.set_new_library("serde_serializable_resource_end");
        recv_std(&mut output, "update_callback_internal")
            .await
            .expect("Didn't Get Download");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "My Serializable Field - My Second Field")
            .await
            .expect("Failed Second Line");
        let _ = send.send(InMessage::Std("exit\n".to_string()));
        recv_exit(&mut output, Some(0))
            .await
            .expect("Wrong Exit Code");
    }

    #[traced_test]
    #[tokio::test]
    async fn can_replace_a_resource() {
        let dir = test_temp_dir!();
        let dir_path = dir.as_path_untracked().to_path_buf();

        let (mut comms, send, mut output, _) = setup_test(dir_path, "replacable_resource_start").await;

        recv_std(&mut output, "My First Field")
            .await
            .expect("Failed first line");
        comms.set_new_library("replacable_resource_end");
        recv_std(&mut output, "update_callback_internal")
            .await
            .expect("Didn't Get Download");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "My First Field - Missing Second Field")
            .await
            .expect("Failed Second Line");
        let _ = send.send(InMessage::Std("exit\n".to_string()));
        recv_exit(&mut output, Some(0))
            .await
            .expect("Wrong Exit Code");
    }
}
