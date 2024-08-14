use std::process::Stdio;

use anyhow::bail;
use tokio::io::{AsyncBufReadExt, BufReader};

pub async fn incremental_rustc() -> anyhow::Result<()> {
    let rustc = Rustc::new(std::env::args())?;
    
    let _ = rustc.run().await?;
    Ok(())
}

#[derive(Debug)]
struct Rustc {
    executable: String,
    operation: RustcOperation
}

#[derive(Debug)]
enum RustcOperation {
    Help,
    Version,
    Command(Vec<String>)
}

impl Rustc {
    fn new(mut args: impl Iterator<Item = String>) -> anyhow::Result<Self> {
        let _current_executable = args.next();

        let Some(executable) = args.next() else {
            bail!("No Rustc Executable In Arguments");
        };

        let args : Vec<String> = args.collect();

        let operation = if args.iter().map(|v| v.as_str()).find(|v| *v == "-V" || *v == "-vV").is_some() {
            RustcOperation::Version
        } else if args.iter().map(|v| v.as_str()).find(|v| *v == "--help").is_some() {
            RustcOperation::Help
        } else {
            RustcOperation::Command(args)
        };

        Ok(Self {
            executable,
            operation
        })
    }

    async fn run(self) -> anyhow::Result<std::process::ExitStatus> {
        let mut command = tokio::process::Command::new(self.executable);

        match self.operation {
            RustcOperation::Help => command.arg("--help"),
            RustcOperation::Version => command.arg("-vV"),
            RustcOperation::Command(args) => {
                command.args(args)
            },
        };

        command.stdout(Stdio::inherit()).stderr(Stdio::inherit());

        let mut child = command.spawn()?;

        // let Some(output) = child.stdout.take() else {
        //     bail!("No Std Out");
        // };
    
        // let Some(error) = child.stderr.take() else {
        //     bail!("No Std Err");
        // };
    
        // tokio::spawn(async move {
        //     let mut out_reader = BufReader::new(error).lines();
        //     while let Ok(Some(line)) = out_reader.next_line().await {
        //         eprintln!("{line}");
        //     }
        // });
        // tokio::spawn(async move {
        //     let mut out_reader = BufReader::new(output).lines();
        //     while let Ok(Some(line)) = out_reader.next_line().await {
        //         println!("{line}");
        //     }
        // });

        Ok(child.wait().await?)
    }
}