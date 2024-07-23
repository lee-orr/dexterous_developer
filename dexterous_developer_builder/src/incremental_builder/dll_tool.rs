use cargo_zigbuild::Zig;

pub async fn dll_tool() -> anyhow::Result<()> {
    let mut args = std::env::args().collect::<Vec<_>>();

    if let Some(first) = args.first() {
        if first.contains("dlltool") {
            args.remove(0);
        }
    }

    let (zig, _) = Zig::find_zig()?;

    let mut cmd = tokio::process::Command::new(zig);
    cmd.arg("dlltool").args(args);

    let result = cmd.output().await;

    match result {
        Err(e) => {
            panic!("Couldn't spawn dlltool - {e}");
        }
        Ok(r) => {
            if !r.status.success() {
                let err = std::str::from_utf8(&r.stderr).unwrap_or("No UTF8");
                panic!("Dlltool Failed - {err}")
            }
        }
    }

    Ok(())
}
