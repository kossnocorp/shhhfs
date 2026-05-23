use crate::prelude::*;

mod mount;
pub use mount::*;

mod unmount;
pub use unmount::*;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Mount
    Mount(MountArgs),

    /// Unmount
    Unmount(UnmountArgs),
}

impl Command {
    pub async fn run(cli: &Cli) -> Result<()> {
        match &cli.command {
            Some(Command::Mount(args)) => Ok(MountCmd::run(cli, args).await?),

            Some(Command::Unmount(args)) => Ok(UnmountCmd::run(args).await?),

            None => unreachable!("No command was provided"),
        }
    }
}
