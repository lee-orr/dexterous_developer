use std::{path::PathBuf, process::Command};

use anyhow::Context;

use crate::Target;

pub trait BuildArgsProvider {
    fn get_cargo(&self) -> &'static str {
        "cargo"
    }

    fn get_cargo_command(&self) -> &'static [&'static str] {
        &["build"]
    }

    fn set_env_vars(&self, command: &mut Command);

    fn get_linker(&self) -> Option<&'static str> {
        None
    }
}

trait GetBuildArgProvider {
    fn get_provider(target: &Target) -> anyhow::Result<Box<dyn BuildArgsProvider>>;
}

pub(crate) fn cargo_command(target: Option<&Target>) -> anyhow::Result<&'static str> {
    let provider = default_host::get_provider(target)?;
    Ok(provider.get_cargo())
}

pub(crate) fn set_envs(
    command: &mut Command,
    target: Option<&Target>,
) -> anyhow::Result<&'static [&'static str]> {
    let provider = default_host::get_provider(target)?;

    if let Some(linker) = provider.get_linker() {
        which::which(linker).context("Can't find lld")?;
    }

    provider.set_env_vars(command);
    Ok(provider.get_cargo_command())
}

mod default_host {
    use crate::Target;

    use super::{BuildArgsProvider, GetBuildArgProvider};
    use std::process::Command;

    use anyhow::{bail, Context};

    #[cfg(target_os = "linux")]
    use super::linux_host::DefaultProvider;
    #[cfg(target_os = "macos")]
    use super::macos_host::DefaultProvider;
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    use super::unknown_host::DefaultProvider;
    #[cfg(target_os = "windows")]
    use super::windows_host::DefaultProvider;

    pub fn get_provider(target: Option<&Target>) -> anyhow::Result<Box<dyn BuildArgsProvider>> {
        if let Some(target) = target {
            let targets = Command::new("rustup")
                .arg("target")
                .arg("list")
                .arg("--installed")
                .output()
                .context("Checking if {target} is installed")?;
            let output = std::str::from_utf8(&targets.stdout)?;

            if output.lines().any(|v| {
                if let Ok(v) = v.parse::<Target>() {
                    v == *target
                } else {
                    false
                }
            }) {
                DefaultProvider::get_provider(target)
            } else {
                bail!("Target {target} is not installed");
            }
        } else {
            Ok(Box::new(DefaultProvider))
        }
    }
}

mod unknown_host {
    use super::*;
    pub struct DefaultProvider;

    impl BuildArgsProvider for DefaultProvider {
        fn set_env_vars(&self, _: &mut Command) {}
    }

    impl GetBuildArgProvider for DefaultProvider {
        fn get_provider(_target: &Target) -> anyhow::Result<Box<dyn BuildArgsProvider>> {
            Ok(Box::new(Self))
        }
    }
}

mod linux_host {
    use super::*;
    pub struct DefaultProvider;

    impl BuildArgsProvider for DefaultProvider {
        fn set_env_vars(&self, command: &mut Command) {
            command
                .env("RUSTC_LINKER", "clang")
                .env("RUSTFLAGS", "-Zshare-generics=y  -Clink-arg=-fuse-ld=lld");
        }
    }

    impl GetBuildArgProvider for DefaultProvider {
        fn get_provider(target: &Target) -> anyhow::Result<Box<dyn BuildArgsProvider>> {
            if *target == Target::Windows {
                return Ok(Box::new(WindowsGNUProvider));
            }
            if *target == Target::Mac {
                return Ok(Box::new(AppleDarwinProvider));
            }
            if *target == Target::MacArm {
                return Ok(Box::new(AppleDarwinArmProvider));
            }
            Ok(Box::new(Self))
        }
    }
    struct WindowsGNUProvider;

    impl BuildArgsProvider for WindowsGNUProvider {
        fn get_cargo(&self) -> &'static str {
            "cross"
        }

        fn set_env_vars(&self, command: &mut Command) {
            command.env("RUSTFLAGS", format!("-Zshare-generics=n"));
        }
    }

    struct AppleDarwinProvider;

    impl BuildArgsProvider for AppleDarwinProvider {
        fn get_cargo(&self) -> &'static str {
            "cross"
        }

        fn set_env_vars(&self, command: &mut Command) {
            command
                .env("RUSTC_LINKER", "clang")
                .env("RUSTFLAGS", format!("-Zshare-generics=y"))
                .env("SYSTEM_VERSION_COMPAT", "0");
        }
    }

    struct AppleDarwinArmProvider;

    impl BuildArgsProvider for AppleDarwinArmProvider {
        fn get_cargo(&self) -> &'static str {
            "cross"
        }

        fn set_env_vars(&self, command: &mut Command) {
            command
                .env("CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER", "clang")
                .env("RUSTC_LINKER", "clang")
                .env("RUSTFLAGS", format!("-Zshare-generics=y -L /opt/osxcross/SDK/latest/usr/include/c++/v1/stdbool.h"))
                .env("SYSTEM_VERSION_COMPAT", "0");
        }
    }
}

mod cross_host {
    use super::*;

    pub struct CrossProvider(PathBuf);

    impl BuildArgsProvider for CrossProvider {
        fn set_env_vars(&self, command: &mut Command) {
            command
                .env("CC", self.0.as_os_str())
                .env("RUST_LINKER", self.0.as_os_str());
        }
    }
}

mod windows_host {
    use super::*;
    pub struct DefaultProvider;

    impl BuildArgsProvider for DefaultProvider {
        fn set_env_vars(&self, command: &mut Command) {
            command
                .env("RUSTC_LINKER", "rust-lld.exe")
                .env("RUSTFLAGS", "-Zshare-generics=n");
        }
    }

    impl GetBuildArgProvider for DefaultProvider {
        fn get_provider(_target: &Target) -> anyhow::Result<Box<dyn BuildArgsProvider>> {
            Ok(Box::new(Self))
        }
    }
}

mod macos_host {
    use super::*;
    pub struct DefaultProvider;

    #[cfg(target_arch = "aarch64")]
    const LLDPATH: &str = "/opt/homebrew/opt/llvm/bin/ld64.lld";
    #[cfg(not(target_arch = "aarch64"))]
    const LLDPATH: &str = "/usr/local/opt/llvm/bin/ld64.lld";

    impl BuildArgsProvider for DefaultProvider {
        fn set_env_vars(&self, command: &mut Command) {
            command.env(
                "RUSTFLAGS",
                format!("-Zshare-generics=y -Clink-arg=-fuse-ld={LLDPATH}"),
            );
        }
    }

    impl GetBuildArgProvider for DefaultProvider {
        fn get_provider(_target: &Target) -> anyhow::Result<Box<dyn BuildArgsProvider>> {
            Ok(Box::new(Self))
        }
    }
}
