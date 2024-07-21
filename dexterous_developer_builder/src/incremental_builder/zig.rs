use std::path::PathBuf;

use cargo_zigbuild::Zig;
use clap::{command, Parser};

#[derive(Debug, Parser)]
#[command(
    version,
    name = "dexterous developer zig compiler",
    display_order = 1,
    styles = cargo_options::styles(),
)]
enum Command {
    #[command(subcommand)]
    Zig(Zig),
}

pub fn zig() -> anyhow::Result<()> {
    let mut args = std::env::args();
    let program_path = PathBuf::from(args.next().expect("no program path"));
    let program_name = program_path.file_stem().expect("no program name");
    if program_name.eq_ignore_ascii_case("ar") {
        let zig = Zig::Ar {
            args: args.collect(),
        };
        zig.execute()?;
    } else {
        eprintln!("Called Incremental C Compiler");
        let command = Command::parse();

        let Command::Zig(zig) = command;

        zig.execute()?;
    }
    Ok(())
}
