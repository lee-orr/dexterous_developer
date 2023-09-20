mod utils;

use std::{
    env,
    path::{Path, PathBuf},
};

use crate::utils::*;

async fn can_run_cold() {
    let mut project = TestProject::new("simple_cli_test", "can_run_cold").unwrap();
    let mut process = project.run_cold().await.unwrap();

    process.is_ready().await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Ran Update"]).await;

    process.send("\n").expect("Failed to send empty line");

    process.next_line_contains("Ran Update").await;

    process.exit().await;
}

async fn can_run_hot() {
    let mut project = TestProject::new("simple_cli_test", "can_run_hot").unwrap();
    let mut process = project.run_hot_cli().await.unwrap();

    process.is_ready().await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Ran Update"]).await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Ran Update"]).await;

    process.exit().await;
}

async fn can_run_hot_and_edit() {
    let mut project = TestProject::new("simple_cli_test", "can_run_hot_and_edit").unwrap();
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

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Got some new text!"]).await;

    process.exit().await;
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

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Got some new text!"]).await;

    process.exit().await;
}

async fn insert_replacable_resource() {
    let mut project: TestProject =
        TestProject::new("reloadables_test", "insert_replacable_resource").unwrap();
    let mut process = project.run_hot_cli().await.unwrap();

    process.is_ready().await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Ran Update"]).await;

    project
        .write_file(
            PathBuf::from("src/update.rs").as_path(),
            include_str!("./insert_replacable_resource.txt"),
        )
        .expect("Couldn't update file");

    process.has_updated().await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Got: Resource Added"]).await;
    process.exit().await;
}

async fn update_replacable_resource() {
    let mut project: TestProject =
        TestProject::new("reloadables_test", "update_replacable_resource").unwrap();

    project
        .write_file(
            PathBuf::from("src/update.rs").as_path(),
            &include_str!("./update_replaceable_resource.txt").replace("Retained:", "Got:"),
        )
        .expect("Couldn't update file");
    let mut process = project.run_hot_cli().await.unwrap();

    process.is_ready().await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Got: Resource Replaced"]).await;

    process
        .send("And Updated\n")
        .expect("Failed to send empty line");

    process
        .wait_for_lines(&["Updated: Resource Replaced And Updated"])
        .await;

    project
        .write_file(
            PathBuf::from("src/update.rs").as_path(),
            include_str!("./update_replaceable_resource.txt"),
        )
        .expect("Couldn't update file");

    process.has_updated().await;

    process.send("\n").expect("Failed to send empty line");

    process
        .wait_for_lines(&["Retained: Resource Replaced And Updated"])
        .await;

    process.exit().await;
}

async fn reset_replacable_resource() {
    let mut project: TestProject =
        TestProject::new("reloadables_test", "reset_replacable_resource").unwrap();

    project
        .write_file(
            PathBuf::from("src/update.rs").as_path(),
            include_str!("./reset_replacable_resource.txt"),
        )
        .expect("Couldn't update file");
    let mut process = project.run_hot_cli().await.unwrap();

    process.is_ready().await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Got: Resource Replaced"]).await;

    process
        .send("And Updated\n")
        .expect("Failed to send empty line");

    process
        .wait_for_lines(&["Updated: Resource Replaced And Updated"])
        .await;

    project
        .write_file(
            PathBuf::from("src/update.rs").as_path(),
            &include_str!("./reset_replacable_resource.txt").replace("Got:", "Reset:"),
        )
        .expect("Couldn't update file");

    process.has_updated().await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Reset: Resource Replaced"]).await;
    process.exit().await;
}

async fn reset_replacable_resource_to_value() {
    let mut project: TestProject =
        TestProject::new("reloadables_test", "reset_replacable_resource_to_value").unwrap();

    project
        .write_file(
            PathBuf::from("src/update.rs").as_path(),
            include_str!("./reset_replacable_resource_to_value.txt"),
        )
        .expect("Couldn't update file");
    let mut process = project.run_hot_cli().await.unwrap();

    process.is_ready().await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Got: Resource At Value"]).await;

    process
        .send("And Updated\n")
        .expect("Failed to send empty line");

    process
        .wait_for_lines(&["Updated: Resource At Value And Updated"])
        .await;

    project
        .write_file(
            PathBuf::from("src/update.rs").as_path(),
            &include_str!("./reset_replacable_resource_to_value.txt").replace("Got:", "Reset:"),
        )
        .expect("Couldn't update file");

    process.has_updated().await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Reset: Resource At Value"]).await;
    process.exit().await;
}

async fn update_resource_schema() {
    let mut project: TestProject =
        TestProject::new("reloadables_test", "update_resource_schema").unwrap();

    project
        .write_file(
            PathBuf::from("src/update.rs").as_path(),
            include_str!("./insert_replacable_resource.txt"),
        )
        .expect("Couldn't update file");
    let mut process = project.run_hot_cli().await.unwrap();

    process.is_ready().await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Got: Resource Added"]).await;
    project
        .write_file(
            PathBuf::from("src/update.rs").as_path(),
            include_str!("./update_schema_resource.txt"),
        )
        .expect("Couldn't update file");

    process.has_updated().await;

    process.send("\n").expect("Failed to send empty line");

    process
        .wait_for_lines(&["Got: Resource Replaced - Added Field"])
        .await;
    process.exit().await;
}

async fn insert_replacable_components() {
    let mut project: TestProject =
        TestProject::new("reloadables_test", "insert_replacable_components").unwrap();

    project
        .write_file(
            PathBuf::from("src/update.rs").as_path(),
            include_str!("./insert_replacable_components.txt"),
        )
        .expect("Couldn't update file");
    let mut process = project.run_hot_cli().await.unwrap();

    process.is_ready().await;

    process.send("first\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Has component: first"]).await;

    process.send("second\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Has component: second"]).await;
    project
        .write_file(
            PathBuf::from("src/update.rs").as_path(),
            &include_str!("./insert_replacable_components.txt")
                .replace("Has component", "COMPONENTS"),
        )
        .expect("Couldn't update file");

    process.has_updated().await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["COMPONENTS: first"]).await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["COMPONENTS: second"]).await;
    process.exit().await;
}

async fn update_schema_component() {
    let mut project: TestProject =
        TestProject::new("reloadables_test", "update_schema_component").unwrap();

    project
        .write_file(
            PathBuf::from("src/update.rs").as_path(),
            include_str!("./insert_replacable_components.txt"),
        )
        .expect("Couldn't update file");
    let mut process = project.run_hot_cli().await.unwrap();

    process.is_ready().await;

    process.send("first\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Has component: first"]).await;

    process.send("second\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Has component: second"]).await;
    project
        .write_file(
            PathBuf::from("src/update.rs").as_path(),
            include_str!("./update_schema_component.txt"),
        )
        .expect("Couldn't update file");
    process.has_updated().await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["inner - first"]).await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["inner - second"]).await;

    process.exit().await;
}

async fn clear_component_on_reload() {
    let mut project: TestProject =
        TestProject::new("reloadables_test", "clear_component_on_reload").unwrap();

    project
        .write_file(
            PathBuf::from("src/update.rs").as_path(),
            include_str!("./clear_on_reload.txt"),
        )
        .expect("Couldn't update file");
    let mut process = project.run_hot_cli().await.unwrap();

    process.is_ready().await;
    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["No components"]).await;

    process.send("first\n").expect("Failed to send first line");

    process.wait_for_lines(&["Has component: first"]).await;

    process
        .send("second\n")
        .expect("Failed to send second line");

    process.wait_for_lines(&["Has component: second"]).await;
    project
        .write_file(
            PathBuf::from("src/update.rs").as_path(),
            &include_str!("./clear_on_reload.txt").replace("Has component", "COMPONENTS"),
        )
        .expect("Couldn't update file");

    process.has_updated().await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["No components"]).await;
    process.exit().await;
}

async fn run_setup_on_reload() {
    let mut project: TestProject =
        TestProject::new("reloadables_test", "run_setup_on_reload").unwrap();

    project
        .write_file(
            PathBuf::from("src/update.rs").as_path(),
            include_str!("./setup_on_reload.txt"),
        )
        .expect("Couldn't update file");
    let mut process = project.run_hot_cli().await.unwrap();

    process.is_ready().await;
    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Components: a_thing"]).await;
    project
        .write_file(
            PathBuf::from("src/update.rs").as_path(),
            &include_str!("./setup_on_reload.txt").replace("a_thing", "b_another"),
        )
        .expect("Couldn't update file");

    process.has_updated().await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Components: b_another"]).await;
    process.exit().await;
}

async fn run_setup_in_state() {
    let mut project: TestProject =
        TestProject::new("reloadables_test", "run_setup_in_state").unwrap();

    project
        .write_file(
            PathBuf::from("src/update.rs").as_path(),
            include_str!("./setup_in_state.txt"),
        )
        .expect("Couldn't update file");
    let mut process = project.run_hot_cli().await.unwrap();

    process.is_ready().await;
    process.send("\n").expect("Failed to send empty line");
    process.wait_for_lines(&["No components"]).await;

    process
        .send("another_state\n")
        .expect("failed to set state");
    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Components: a_thing"]).await;

    process
        .send("default_state\n")
        .expect("failed to set state");
    process.send("\n").expect("Failed to send empty line");
    process.wait_for_lines(&["No components"]).await;

    process
        .send("another_state\n")
        .expect("failed to set state");
    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Components: a_thing"]).await;

    project
        .write_file(
            PathBuf::from("src/update.rs").as_path(),
            &include_str!("./setup_in_state.txt").replace("a_thing", "b_another"),
        )
        .expect("Couldn't update file");

    process.has_updated().await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Components: b_another"]).await;

    process
        .send("default_state\n")
        .expect("failed to set state");
    process.send("\n").expect("Failed to send empty line");
    process.wait_for_lines(&["No components"]).await;
    process.exit().await;
}

async fn can_run_remote() {
    let mut project = TestProject::new("simple_cli_test", "can_run_remote_host").unwrap();
    let mut client = TestProject::new("remote_client", "can_run_remote_client").unwrap();

    let mut host_process = project.run_host_cli("1234").await.unwrap();

    host_process.wait_for_lines(&["Serving on 1234"]).await;

    let mut process = client.run_client_cli("1234").await.unwrap();

    process.wait_for_lines(&["Got Message"]).await;
    process.exit().await;

    host_process.is_ready().await;

    let mut process = client.run_client_cli("1234").await.unwrap();

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

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Got some new text!"]).await;

    process.exit().await;
    host_process.exit().await;
}

async fn can_update_assets() {
    let mut project = TestProject::new("simple_cli_test", "can_update_assets_host").unwrap();
    let mut client = TestProject::new("remote_client", "can_update_assets_client").unwrap();

    let mut host_process = project.run_host_cli("2345").await.unwrap();

    host_process.wait_for_lines(&["Serving on 2345"]).await;

    let mut process = client.run_client_cli("2345").await.unwrap();

    process.wait_for_lines(&["Got Message"]).await;
    process.exit().await;

    host_process.is_ready().await;

    let mut process = client.run_client_cli("2345").await.unwrap();

    process.is_ready().await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Ran Update"]).await;

    project
        .write_file(
            PathBuf::from("assets/nesting/another_placeholder.txt").as_path(),
            "changed content",
        )
        .expect("Couldn't update file");

    process.wait_for_lines(&["Downloaded Asset"]).await;

    process.exit().await;
    host_process.exit().await;
}

async fn can_run_existing(path: &Path) {
    let mut project = TestProject::existing_project(path, "can_run_existing").unwrap();

    let mut process = project.run_existing().await.unwrap();

    process.is_ready().await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Ran Update"]).await;

    process.send("\n").expect("Failed to send empty line");

    process.wait_for_lines(&["Ran Update"]).await;

    process.exit().await;
}

pub async fn run_tests() {
    let mut args = env::args();
    args.next();
    let argument = args.next().unwrap_or_default();

    match argument.as_str() {
        "cold" => {
            println!("Can run cold");
            can_run_cold().await;
        }
        "hot" => {
            println!("Can run hot cli");
            can_run_hot().await;
        }
        "edit" => {
            println!("Can edit with hot reload cli");
            can_run_hot_and_edit().await;
        }
        "launcher" => {
            println!("Can edit with hot reload launcher");
            can_run_hot_and_edit_with_launcher().await;
        }
        "initialize_resource" => {
            insert_replacable_resource().await;
        }
        "update_resource" => {
            update_replacable_resource().await;
        }
        "reset_resource" => {
            reset_replacable_resource().await;
        }
        "reset_resource_to_value" => {
            reset_replacable_resource_to_value().await;
        }
        "resource_schema" => {
            update_resource_schema().await;
        }
        "insert_components" => {
            insert_replacable_components().await;
        }
        "component_schema" => {
            update_schema_component().await;
        }
        "clear_on_reload" => {
            clear_component_on_reload().await;
        }
        "setup_on_reload" => {
            run_setup_on_reload().await;
        }
        "setup_in_state" => {
            run_setup_in_state().await;
        }
        "remote" => {
            println!("Can run remote");
            can_run_remote().await;
        }
        "asset" => {
            println!("Can update asset");
            can_update_assets().await;
        }
        "existing" => {
            println!("Can run existing assets");
            let libs = args.next().expect("No next lib set");
            let mut libs = PathBuf::from(libs);
            if !libs.is_absolute() {
                libs = std::env::current_dir().unwrap().join(libs);
            }
            if !libs.exists() || !libs.is_dir() {
                panic!("libs should be a directory");
            }
            let libs = libs.canonicalize().unwrap();
            can_run_existing(&libs).await;
        }
        _ => {
            eprintln!("{argument} is an invalid test");
            println!("Valid tests are:");
            println!("cold");
            println!("hot");
            println!("edit");
            println!("launcher");
            println!("remote");
            println!("asset");
            println!("initialize_resource");
            println!("update_resource");
            println!("reset_resource");
            println!("reset_replacable_resource_to_value");
            println!("resource_schema");
            println!("insert_components");
            println!("component_schema");
            println!("clear_on_reload");
            println!("setup_on_reload");
            println!("setup_in_state");
            std::process::exit(1)
        }
    }
}

#[cfg(test)]
mod test {
    // #[tokio::test]
    // async fn can_run_cold() {
    //     super::can_run_cold().await;
    // }
    // #[tokio::test]
    // async fn can_run_hot() {
    //     super::can_run_hot().await;
    // }
    // #[tokio::test]
    // async fn can_run_hot_and_edit() {
    //     super::can_run_hot_and_edit().await;
    // }
    // #[tokio::test]
    // async fn can_run_hot_and_edit_with_launcher() {
    //     super::can_run_hot_and_edit_with_launcher().await;
    // }
    // #[tokio::test]
    // async fn can_run_with_reloadables() {
    //     super::can_run_with_reloadables().await;
    // }
    #[tokio::test]
    async fn can_run_remote() {
        super::can_run_remote().await;
    }
    // #[tokio::test]
    // async fn can_update_assets() {
    //     super::can_update_assets().await;
    // }
}
