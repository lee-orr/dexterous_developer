cargo clippy --fix --allow-dirty --workspace --all-targets --all-features -- -D warnings -A clippy::type_complexity -W clippy::doc_markdown
cargo fmt --all