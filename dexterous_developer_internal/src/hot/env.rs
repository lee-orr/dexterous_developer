use std::process::Command;

use anyhow::{bail, Context, Error};

use crate::logger::error;

trait BuildArgsProvider {
    fn get_cargo_command(&self) -> &'static str {
        "build"
    }

    fn set_env_vars(&self, command: &mut Command);

    fn get_linker(&self) -> Option<&'static str> {
        None
    }
}

trait GetBuildArgProvider {
    fn get_provider(target: Option<&str>) -> Option<Box<dyn BuildArgsProvider>>;
}

pub(crate) fn set_envs(
    command: &mut Command,
    target: Option<&str>,
) -> anyhow::Result<&'static str> {
    let provider = default_host::DefaultProvider::get_provider(target)
        .ok_or(anyhow::Error::msg("No Build Arg Provider for {target:?}"))?;

    if let Some(linker) = provider.get_linker() {
        which::which(linker).context("Can't find lld")?;
    }

    provider.set_env_vars(command);
    Ok(provider.get_cargo_command())
}

mod default_host {
    #[cfg(target_os = "linux")]
    pub use super::linux_host::DefaultProvider;
    #[cfg(target_os = "macos")]
    pub use super::macos_host::DefaultProvider;
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    pub use super::unknown_host::DefaultProvider;
    #[cfg(target_os = "windows")]
    pub use super::windows_host::DefaultProvider;
}

mod unknown_host {
    use super::*;
    pub struct DefaultProvider;

    impl BuildArgsProvider for DefaultProvider {
        fn set_env_vars(&self, _: &mut Command) {}
    }

    impl GetBuildArgProvider for DefaultProvider {
        fn get_provider(target: Option<&str>) -> Option<Box<dyn BuildArgsProvider>> {
            Some(Box::new(Self))
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
        fn get_provider(target: Option<&str>) -> Option<Box<dyn BuildArgsProvider>> {
            Some(Box::new(Self))
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
        fn get_provider(target: Option<&str>) -> Option<Box<dyn BuildArgsProvider>> {
            Some(Box::new(Self))
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
        fn get_provider(target: Option<&str>) -> Option<Box<dyn BuildArgsProvider>> {
            Some(Box::new(Self))
        }
    }
}
