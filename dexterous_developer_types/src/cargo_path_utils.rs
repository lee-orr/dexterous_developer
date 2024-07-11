use std::{env, ffi, io};

use camino::{Utf8Path, Utf8PathBuf};
use thiserror::Error;

pub fn dylib_path_envvar() -> &'static str {
    if cfg!(windows) {
        "PATH"
    } else if cfg!(target_os = "macos") {
        // When loading and linking a dynamic library or bundle, dlopen
        // searches in LD_LIBRARY_PATH, DYLD_LIBRARY_PATH, PWD, and
        // DYLD_FALLBACK_LIBRARY_PATH.
        // In the Mach-O format, a dynamic library has an "install path."
        // Clients linking against the library record this path, and the
        // dynamic linker, dyld, uses it to locate the library.
        // dyld searches DYLD_LIBRARY_PATH *before* the install path.
        // dyld searches DYLD_FALLBACK_LIBRARY_PATH only if it cannot
        // find the library in the install path.
        // Setting DYLD_LIBRARY_PATH can easily have unintended
        // consequences.
        //
        // Also, DYLD_LIBRARY_PATH appears to have significant performance
        // penalty starting in 10.13. Cargo's testsuite ran more than twice as
        // slow with it on CI.
        "DYLD_FALLBACK_LIBRARY_PATH"
    } else if cfg!(target_os = "aix") {
        "LIBPATH"
    } else {
        "LD_LIBRARY_PATH"
    }
}

/// Returns a list of directories that are searched for dynamic libraries.
///
/// Note that some operating systems will have defaults if this is empty that
/// will need to be dealt with.
pub fn dylib_path() -> Vec<Utf8PathBuf> {
    match env::var_os(dylib_path_envvar()) {
        Some(var) => env::split_paths(&var)
            .filter_map(|p| Utf8PathBuf::try_from(p).ok())
            .collect(),
        None => Vec::new(),
    }
}

pub fn add_to_dylib_path(path: &[&Utf8Path]) -> Result<(&'static str, ffi::OsString), Error> {
    let mut cannonical = path
        .iter()
        .map(|path| {
            if !path.exists() {
                std::fs::create_dir_all(path);
            }
            path.canonicalize_utf8().map_err(|e| e.into())
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let mut dylibs = dylib_path();
    dylibs.append(&mut cannonical);
    let value = env::join_paths(&dylibs)?;
    let env_var = dylib_path_envvar();
    env::set_var(env_var, &value);

    Ok((env_var, value))
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO Error {0}")]
    IoError(#[from] io::Error),
    #[error("Join Paths Error {0}")]
    JoinPathsError(#[from] env::JoinPathsError),
}

/// Returns a list of directories that are searched for dynamic libraries.
///
/// Note that some operating systems will have defaults if this is empty that
/// will need to be dealt with.
pub fn bin_path() -> Vec<Utf8PathBuf> {
    match env::var_os("PATH") {
        Some(var) => env::split_paths(&var)
            .filter_map(|p| Utf8PathBuf::try_from(p).ok())
            .collect(),
        None => Vec::new(),
    }
}

pub fn print_dylib_path() -> String {
    dylib_path()
        .iter()
        .map(|v| format!("{v:?}"))
        .collect::<Vec<_>>()
        .join("\n")
}
