use std::path::PathBuf;

use clap::{Parser, Subcommand};
use moontoon::{player::Player, runtime::Runtime};

#[derive(Debug, Parser)]
struct Cli {
    #[clap(long = "root", default_value = ".")]
    root: PathBuf,
    #[clap(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    Run,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Cmd::Run => {
            let runtime = Runtime::new(cli.root)?;

            let stream = runtime.load_project()?;

            let mut player = Player::connect()?;
            player.play(stream);

            player.wait_until_end()?;
        },
    }

    Ok(())
}
