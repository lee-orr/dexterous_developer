use xshell::cmd;

fn main() {
    // When run locally, results may differ from actual CI runs triggered by
    // .github/workflows/ci.yml

    // See if any code needs to be formatted
    cmd!("cargo fmt --all -- --check")
        .run()
        .expect("Please run 'cargo fmt --all' to format your code.");

    cmd!("cargo clippy --workspace --all-targets --all-features -- -D warnings -A clippy::type_complexity -W clippy::doc_markdown")
    .run()
    .expect("Please fix clippy errors in output above.");

    let mut cwd = std::env::current_dir().expect("Couldn't get current directory");
    cwd.push("testing");
    cwd.push("dexterous_developer_tests");
    std::env::set_current_dir(cwd).expect("Couldn't set current directory");

    cmd!("cargo test -p dexterous_developer_tests")
        .run()
        .expect("Expect cargo test to work");
}
