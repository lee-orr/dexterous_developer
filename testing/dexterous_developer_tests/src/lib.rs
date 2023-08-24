mod utils;

use crate::utils::*;

async fn can_run_cold() {
    let mut project = TestProject::new("simple_cli_test", "can_run_cold").unwrap();
    let mut process = project.run_cold().await.unwrap();

    process.is_ready().await;

    process
        .next_line_contains("Press Enter to Progress, or type 'exit' to exit")
        .await;

    process.next_line_contains("Ran Update").await;

    process.send("\n").expect("Failed to send empty line");

    process.next_line_contains("Ran Update").await;

    process.send("exit\n").expect("Failed to send line");

    process.exiting().await;
}

async fn can_run_hot() {
    let mut project = TestProject::new("simple_cli_test", "can_run_hot").unwrap();
    let mut process = project.run_hot_cli().await.unwrap();

    process.is_ready().await;

    process
        .wait_for_lines(&[
            "Press Enter to Progress, or type 'exit' to exit",
            "Ran Update",
        ])
        .await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Ran Update"]).await;

    process.send("exit\n").expect("Failed to send line");

    process.exiting().await;
}

pub async fn run_tests() {
    println!("Can run cold");
    can_run_cold().await;
    println!("Can run hot cli");
    can_run_hot().await;
}

#[cfg(test)]
mod test {
    #[tokio::test]
    async fn can_run_cold() {
        super::can_run_cold().await;
    }
    #[tokio::test]
    async fn can_run_hot() {
        super::can_run_hot().await;
    }
}
