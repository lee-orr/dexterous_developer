//! A linker that catches incremental change artifacts applies recent changes to a dynamic library
//!
//! Heavily derived from Jon Kelley's work - <https://github.com/jkelleyrtp/ipbp/blob/main/packages/patch-linker/src/main.rs>

use camino::Utf8PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    eprintln!("Called Incremental C Compiler");
    let mut args = std::env::args()
        .filter(|v| {
            !v.contains("dexterous_developer_incremental_linker")
                && !v.contains("incremental_c_compiler")
                && !v.contains("--target=")
        })
        .collect::<Vec<_>>();

    let target = std::env::var("DEXTEROUS_DEVELOPER_LINKER_TARGET")?;
    let zig_path: Utf8PathBuf = Utf8PathBuf::from(std::env::var("ZIG_PATH")?);

    args.insert(0, "cc".to_string());

    eprintln!("Calling Zig cc - {args:?}");
    args.push("-target".to_string());
    args.push(target);

    let output = tokio::process::Command::new(zig_path)
        .args(&args)
        .spawn()?
        .wait_with_output()
        .await?;

    if !output.status.success() {
        eprintln!("FAILED CC ARGS: zig {}", args.join(" "));
    }

    std::process::exit(output.status.code().unwrap_or_default());
}
