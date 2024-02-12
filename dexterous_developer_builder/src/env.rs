use std::process::Command;

use anyhow::Context;
use dexterous_developer_types::Target;

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

pub fn cargo_command(target: Option<&Target>) -> anyhow::Result<&'static str> {
    let provider = default_host::get_provider(target)?;
    Ok(provider.get_cargo())
}

pub fn set_envs(
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
    use super::{BuildArgsProvider, GetBuildArgProvider};
    use dexterous_developer_types::Target;
    use std::process::Command;
    use tracing::debug;

    use anyhow::{bail, Context};

    #[cfg(target_os = "linux")]
    use super::linux_host::DefaultProvider;
    #[cfg(target_os = "macos")]
    use super::macos_host::DefaultProvider;
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    use super::unknown_host::DefaultProvider;
    #[cfg(target_os = "windows")]
    use super::windows_host::DefaultProvider;

    #[allow(unreachable_code)]
    fn get_native_target() -> Target {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            return Target::Linux;
        }
        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        {
            return Target::LinuxArm;
        }
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        {
            return Target::Windows;
        }
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        {
            return Target::Mac;
        }
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            return Target::MacArm;
        }

        panic!("Invalid Platform...");
    }

    pub fn get_provider(target: Option<&Target>) -> anyhow::Result<Box<dyn BuildArgsProvider>> {
        if let Some(target) = target {
            if *target != get_native_target() {
                let targets = Command::new("rustup")
                    .arg("target")
                    .arg("list")
                    .arg("--installed")
                    .output()
                    .context("Checking if {target} is installed")?;
                let output = std::str::from_utf8(&targets.stdout)?;

                if output.lines().any(|v| {
                    debug!("Checking {v}");
                    if let Ok(v) = v.parse::<Target>() {
                        v == *target
                    } else {
                        false
                    }
                }) {
                    return super::cross_host::DefaultProvider::get_provider(target);
                } else {
                    bail!("Target {target} is not installed");
                }
            }
        }
        Ok(Box::new(DefaultProvider))
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

    #[cfg(target_arch = "aarch64")]
    const LINKER_VAR: &str = "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER";

    #[cfg(not(target_arch = "aarch64"))]
    const LINKER_VAR: &str = "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER";

    impl BuildArgsProvider for DefaultProvider {
        fn set_env_vars(&self, command: &mut Command) {
            if let Ok(ld_path) = std::env::var("DEXTEROUS_DEVELOPER_LD_PATH") {
                command
                    .env(LINKER_VAR, "clang")
                    .env("RUSTFLAGS", format!("-Clink-arg=-fuse-ld={ld_path}"));
            } else {
                command
                    .env(LINKER_VAR, "clang")
                    .env("RUSTFLAGS", "-Clink-arg=-fuse-ld=lld");
            }
        }
    }

    impl GetBuildArgProvider for DefaultProvider {
        fn get_provider(_: &Target) -> anyhow::Result<Box<dyn BuildArgsProvider>> {
            Ok(Box::new(Self))
        }
    }
}

mod cross_host {
    use super::*;

    pub struct DefaultProvider;

    impl GetBuildArgProvider for DefaultProvider {
        fn get_provider(target: &Target) -> anyhow::Result<Box<dyn BuildArgsProvider>> {
            match target {
                Target::Linux => Ok(Box::new(LinuxProvider)),
                Target::LinuxArm => Ok(Box::new(LinuxProvider)),
                Target::Windows => Ok(Box::new(WindowsProvider)),
                Target::Mac => Ok(Box::new(AppleDarwinProvider)),
                Target::MacArm => Ok(Box::new(AppleDarwinArmProvider)),
                Target::Android => Ok(Box::new(AndroidProvider)),
                Target::IOS => Ok(Box::new(AppleIOSProvider)),
            }
        }
    }

    struct LinuxProvider;

    impl BuildArgsProvider for LinuxProvider {
        fn get_cargo(&self) -> &'static str {
            "cross"
        }

        fn set_env_vars(&self, _: &mut Command) {}
    }

    struct WindowsProvider;

    impl BuildArgsProvider for WindowsProvider {
        fn get_cargo(&self) -> &'static str {
            "cross"
        }

        fn set_env_vars(&self, _: &mut Command) {}
    }

    struct AndroidProvider;

    impl BuildArgsProvider for AndroidProvider {
        fn get_cargo(&self) -> &'static str {
            "cross"
        }

        fn set_env_vars(&self, _: &mut Command) {}
    }

    struct AppleDarwinProvider;

    impl BuildArgsProvider for AppleDarwinProvider {
        fn get_cargo(&self) -> &'static str {
            "cross"
        }

        fn set_env_vars(&self, command: &mut Command) {
            command
                .env("CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER", "clang")
                .env("RUSTC_LINKER", "clang")
                .env(
                    "RUSTFLAGS",
                    "-L /opt/osxcross/SDK/latest/usr/include/c++/v1/stdbool.h",
                )
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
                .env(
                    "RUSTFLAGS",
                    "-L /opt/osxcross/SDK/latest/usr/include/c++/v1/stdbool.h",
                )
                .env("SYSTEM_VERSION_COMPAT", "0");
        }
    }
    struct AppleIOSProvider;

    impl BuildArgsProvider for AppleIOSProvider {
        fn get_cargo(&self) -> &'static str {
            "cross"
        }

        fn set_env_vars(&self, command: &mut Command) {
            command
                .env("CARGO_TARGET_AARCH64_APPLE_IOS_LINKER", "clang")
                .env("RUSTC_LINKER", "clang")
                .env(
                    "RUSTFLAGS",
                    "-L /opt/osxcross/SDK/latest/usr/include/c++/v1/stdbool.h",
                )
                .env("SYSTEM_VERSION_COMPAT", "0");
        }
    }
}

mod windows_host {
    use super::*;
    pub struct DefaultProvider;

    impl BuildArgsProvider for DefaultProvider {
        fn set_env_vars(&self, command: &mut Command) {
            command.env("CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER", "rust-lld.exe");
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
            command.env("RUSTFLAGS", format!("-Clink-arg=-fuse-ld={LLDPATH}"));
        }
    }

    impl GetBuildArgProvider for DefaultProvider {
        fn get_provider(_target: &Target) -> anyhow::Result<Box<dyn BuildArgsProvider>> {
            Ok(Box::new(Self))
        }
    }
}
