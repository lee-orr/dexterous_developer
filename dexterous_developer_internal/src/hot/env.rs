use std::process::Command;

use anyhow::{bail, Context, Error};

use crate::logger::error;

#[cfg(target_os = "windows")]
const RUSTC_ARGS: [(&str, &str); 3] = [
    ("RUSTUP_TOOLCHAIN", "nightly"),
    ("RUSTC_LINKER", "rust-lld.exe"),
    ("RUSTFLAGS", "-Zshare-generics=n"),
];
#[cfg(target_os = "linux")]
const RUSTC_ARGS: [(&str, &str); 3] = [
    ("RUSTUP_TOOLCHAIN", "nightly"),
    ("RUSTC_LINKER", "clang"),
    ("RUSTFLAGS", "-Zshare-generics=y  -Clink-arg=-fuse-ld=lld"),
];
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const RUSTC_ARGS: [(&str, &str); 2] = [
    ("RUSTUP_TOOLCHAIN", "nightly"),
    (
        "RUSTFLAGS",
        "-Zshare-generics=y -Clink-arg=-fuse-ld=/opt/homebrew/opt/llvm/bin/ld64.lld",
    ),
];
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const RUSTC_ARGS: [(&str, &str); 2] = [
    ("RUSTUP_TOOLCHAIN", "nightly"),
    (
        "RUSTFLAGS",
        "-Zshare-generics=y -Clink-arg=-fuse-ld=/usr/local/opt/llvm/bin/ld64.lld",
    ),
];
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
const RUSTC_ARGS: [(&str, &str); 2] = [
    ("RUSTUP_TOOLCHAIN", "nightly"),
    ("RUSTFLAGS", "-Zshare-generics=y"),
];

pub(crate) fn set_envs(command: &mut Command) -> anyhow::Result<()> {
    for (var, val) in RUSTC_ARGS.iter() {
        if (var == &"RUSTC_LINKER") && which::which(&val).is_err() {
            bail!("Linker {val} is not installed");
        } else if val.contains("-fuse-ld=") {
            let mut split = val.split("-fuse-ld=");
            let _ = split.next();
            let after = split.next().ok_or(Error::msg("No value for -fuse-ld="))?;
            which::which(after).context("Can't find lld")?;
        }
        command.env(var, val);
    }
    Ok(())
}
