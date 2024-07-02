mod state_test {
    use dexterous_developer_test_utils::{
        recv_exit, recv_std, recv_std_avoiding, replace_library, setup_test, InMessage,
    };
    use test_temp_dir::*;
    use tracing_test::traced_test;

    #[traced_test]
    #[tokio::test]
    async fn can_initialize_a_state() {
        let dir = test_temp_dir!();
        let dir_path = dir.as_path_untracked().to_path_buf();
        let (mut comms, send, mut output, _) = setup_test(dir_path, "replacable_state_start").await;

        recv_std(&mut output, "In Initial State")
            .await
            .expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "In Another State")
            .await
            .expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "In Another State")
            .await
            .expect("Failed first line");
        replace_library("replacable_state_end", &mut comms, &mut output, &send).await;
        recv_std(&mut output, "In Another State")
            .await
            .expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "In A Third State")
            .await
            .expect("Failed Second Line");
        let _ = send.send(InMessage::Std("exit\n".to_string()));
        recv_exit(&mut output, Some(0))
            .await
            .expect("Wrong Exit Code");
    }

    #[traced_test]
    #[tokio::test]
    async fn triggers_on_enter_on_first_initialization_but_not_after() {
        let dir = test_temp_dir!();
        let dir_path = dir.as_path_untracked().to_path_buf();
        let (mut comms, send, mut output, _) = setup_test(dir_path, "replacable_state_start").await;

        recv_std(&mut output, "Entered Initial")
            .await
            .expect("Failed first line");
        recv_std(&mut output, "In Initial State")
            .await
            .expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "Entered Another")
            .await
            .expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        replace_library("replacable_state_end", &mut comms, &mut output, &send).await;
        recv_std_avoiding(
            &mut output,
            "In Another State",
            &["Initial", "Entered Another"],
        )
        .await
        .expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "In A Third State")
            .await
            .expect("Failed Second Line");
        let _ = send.send(InMessage::Std("exit\n".to_string()));
        recv_exit(&mut output, Some(0))
            .await
            .expect("Wrong Exit Code");
    }

    #[traced_test]
    #[tokio::test]
    async fn can_add_a_sub_state() {
        let dir = test_temp_dir!();
        let dir_path = dir.as_path_untracked().to_path_buf();
        let (mut comms, send, mut output, _) =
            setup_test(dir_path, "replacable_sub_state_start").await;

        recv_std(&mut output, "In Initial State")
            .await
            .expect("Failed first line");
        eprintln!("FOUND  STATE");
        let _ = send.send(InMessage::Std("\n".to_string()));
        eprintln!("SEARCHING FOR SUB STATE");
        recv_std(&mut output, "In Initial Sub State")
            .await
            .expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "In Another Sub State")
            .await
            .expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "In Another Sub State")
            .await
            .expect("Failed first line");
        replace_library("replacable_sub_state_end", &mut comms, &mut output, &send).await;
        recv_std(&mut output, "In Another Sub State")
            .await
            .expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "In Third Sub State")
            .await
            .expect("Failed Second Line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "In Third State")
            .await
            .expect("Failed Second Line");
        let _ = send.send(InMessage::Std("exit\n".to_string()));
        recv_exit(&mut output, Some(0))
            .await
            .expect("Wrong Exit Code");
    }

    #[traced_test]
    #[tokio::test]
    async fn can_add_a_computed_state() {
        let dir = test_temp_dir!();
        let dir_path = dir.as_path_untracked().to_path_buf();
        let (mut comms, send, mut output, _) =
            setup_test(dir_path, "replacable_computed_state_start").await;

        recv_std(&mut output, "In Initial State")
            .await
            .expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "In Another State")
            .await
            .expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "In Another State")
            .await
            .expect("Failed first line");
        replace_library(
            "replacable_computed_state_end",
            &mut comms,
            &mut output,
            &send,
        )
        .await;
        recv_std(&mut output, "In Another State - 2")
            .await
            .expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "In Another State - 3")
            .await
            .expect("Failed Second Line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "In Another State - 3")
            .await
            .expect("Failed Second Line");
        let _ = send.send(InMessage::Std("exit\n".to_string()));
        recv_exit(&mut output, Some(0))
            .await
            .expect("Wrong Exit Code");
    }

    #[traced_test]
    #[tokio::test]
    async fn can_enable_state_scoped() {
        let dir = test_temp_dir!();
        let dir_path = dir.as_path_untracked().to_path_buf();
        let (mut comms, send, mut output, _) = setup_test(dir_path, "state_scoped_start").await;

        recv_std(&mut output, "0 - ab.")
            .await
            .expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "1 - a.")
            .await
            .expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "1 - a.")
            .await
            .expect("Failed first line");
        replace_library("state_scoped_end", &mut comms, &mut output, &send).await;
        recv_std(&mut output, "1 - ac!")
            .await
            .expect("Failed first line");
        let _ = send.send(InMessage::Std("\n".to_string()));
        let _ = send.send(InMessage::Std("\n".to_string()));
        recv_std(&mut output, "3 - a!")
            .await
            .expect("Failed Second Line");
        let _ = send.send(InMessage::Std("exit\n".to_string()));
        recv_exit(&mut output, Some(0))
            .await
            .expect("Wrong Exit Code");
    }
}
