use crate::prelude::*;

mod mount;
pub use mount::*;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Mount
    Mount(MountArgs),
}

impl Command {
    pub async fn run(cli: &Cli) -> Result<()> {
        match &cli.command {
            Some(Command::Mount(args)) => Ok(MountCmd::run(cli, args).await?),

            None => unreachable!("No command was provided"),
        }
    }
}
