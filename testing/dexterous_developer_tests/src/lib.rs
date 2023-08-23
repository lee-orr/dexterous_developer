mod utils;

use crate::utils::*;

async fn can_run_cold() {
    let mut project = TestProject::new("simple_cli_test", "can_run_cold").unwrap();
    let mut process = project.run_cold().await.unwrap();

    loop {
        match process.read_next_line().await {
            Ok(Line::Std(line)) => {
                if line.contains("Running") {
                    break;
                }
            }

            Ok(Line::Err(line)) => {
                if line.contains("Running") {
                    break;
                }
            }

            Ok(_) => {
                return;
            }

            Err(e) => panic!("Got an error {e:?}"),
        };
    }

    let Ok(Line::Std(line)) = process.read_next_line().await else {
        panic!("Should have gotten a line");
    };

    assert!(line.contains("Press Enter to Progress, or type 'exit' to exit"));
    process.send("\n").expect("Failed to send empty line");
    let Ok(Line::Std(line)) = process.read_next_line().await else {
        panic!("Should have gotten a line");
    };

    assert!(line.contains("Ran Update"));

    process.send("exit\n").expect("Failed to send line");

    let Ok(Line::Std(line)) = process.read_next_line().await else {
        panic!("Should have gotten a line");
    };

    assert!(line.contains("Exiting"));
}

pub async fn run_tests() {
    println!("Can run cold");
    can_run_cold().await;
}

#[cfg(test)]
mod test {
    #[tokio::test]
    async fn can_run_cold() {
        super::can_run_cold().await;
    }
}
