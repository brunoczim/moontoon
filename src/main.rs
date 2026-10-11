use std::path::PathBuf;

use clap::Parser;
use moontoon::{player::Player, runtime::Runtime};

#[derive(Debug, Parser)]
struct Cli {
    #[clap(long = "root", default_value = ".")]
    root: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let mut runtime = Runtime::new(cli.root)?;
    runtime.preload()?;

    let stream = runtime.load_project()?;

    let mut player = Player::connect()?;
    player.play(stream);

    player.wait_until_end()?;

    Ok(())
}
