mod utils;

use std::path::PathBuf;

use crate::utils::*;

async fn can_run_cold() {
    let mut project = TestProject::new("simple_cli_test", "can_run_cold").unwrap();
    let mut process = project.run_cold().await.unwrap();

    process.is_ready().await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Ran Update"]).await;

    process.send("\n").expect("Failed to send empty line");

    process.next_line_contains("Ran Update").await;

    process.send("exit\n").expect("Failed to send line");

    process.exiting().await;
}

async fn can_run_hot() {
    let mut project = TestProject::new("simple_cli_test", "can_run_hot").unwrap();
    let mut process = project.run_hot_cli().await.unwrap();

    process.is_ready().await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Ran Update"]).await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Ran Update"]).await;

    process.send("exit\n").expect("Failed to send line");

    process.exiting().await;
}

async fn can_run_hot_and_edit() {
    let mut project = TestProject::new("simple_cli_test", "can_run_hotand_edit").unwrap();
    let mut process = project.run_hot_cli().await.unwrap();

    process.is_ready().await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Ran Update"]).await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Ran Update"]).await;

    project
        .write_file(
            PathBuf::from("src/update.rs").as_path(),
            include_str!("./updated_file.txt"),
        )
        .expect("Couldn't update file");

    process.has_updated().await;

    process.wait_for_lines(&["Got some new text!"]).await;

    process.send("exit\n").expect("Failed to send line");

    process.exiting().await;
}

async fn can_run_hot_and_edit_with_launcher() {
    let mut project = TestProject::new("no_cli_test", "no_cli").unwrap();
    let mut process = project.run_hot_launcher("lib_simple").await.unwrap();

    process.is_ready().await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Ran Update"]).await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Ran Update"]).await;

    project
        .write_file(
            PathBuf::from("./simple/src/update.rs").as_path(),
            include_str!("./updated_file.txt"),
        )
        .expect("Couldn't update file");

    process.has_updated().await;

    process.wait_for_lines(&["Got some new text!"]).await;

    process.send("exit\n").expect("Failed to send line");

    process.exiting().await;
}

pub async fn run_tests() {
    println!("Can run cold");
    can_run_cold().await;
    println!("Can run hot cli");
    can_run_hot().await;
    println!("Can edit with hot reload cli");
    can_run_hot_and_edit().await;
    println!("Can edit with hot reload launcher");
    can_run_hot_and_edit_with_launcher().await;
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
    #[tokio::test]
    async fn can_run_hot_and_edit() {
        super::can_run_hot_and_edit().await;
    }
    #[tokio::test]
    async fn can_run_hot_and_edit_with_launcher() {
        super::can_run_hot_and_edit_with_launcher().await;
    }
}
