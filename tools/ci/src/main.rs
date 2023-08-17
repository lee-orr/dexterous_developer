use xshell::cmd;

fn main() {
    // When run locally, results may differ from actual CI runs triggered by
    // .github/workflows/ci.yml

    // See if any code needs to be formatted
    cmd!("cargo fmt --all -- --check")
        .run()
        .expect("Please run 'cargo fmt --all' to format your code.");

    // Run tests
    cmd!("cargo test")
        .run()
        .expect("Please fix failing tests in output above.");

    // Run doc tests: these are ignored by `cargo test`
    cmd!("cargo test --doc --workspace")
        .run()
        .expect("Please fix failing doc-tests in output above.");

    // See if clippy has any complaints.
    // - Type complexity must be ignored because we use huge templates for queries
    cmd!("cargo clippy --workspace --all-targets --all-features -- -D warnings -A clippy::type_complexity -W clippy::doc_markdown")
    .run()
    .expect("Please fix clippy errors in output above.");
}
