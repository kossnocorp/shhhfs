use crate::prelude::*;

use std::time::Duration;
use tokio::time::sleep;

#[derive(Error, Debug)]
pub enum MountError {
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Args, Debug)]
pub struct MountArgs {
    // TODO: Put the path argument here
}

pub struct MountCmd {}

impl MountCmd {
    pub async fn run<'a>(cli: &'a Cli, args: &'a MountArgs) -> Result<(), MountError> {
        // TODO: Get it from args
        let path = PathBuf::from("./vfs");

        let spinner = UiTheme::start_spinner(&format!("Mounting virtual file system in {path:?}"));

        // TODO: Actual code here
        sleep(Duration::from_millis(2000)).await;

        spinner.finish_with_message(format!("Mounted file system in {path:?}"));

        Ok(())
    }
}
