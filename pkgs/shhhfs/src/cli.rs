use crate::prelude::*;

#[derive(Parser)]
#[command(name = "shhhfs")]
#[command(about = "Virtual file system for secrets", long_about = None)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    /// Set current directory for the command
    #[arg(short, long, value_name = "DIR")]
    pub cd: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

impl Cli {
    pub async fn run() -> Result<()> {
        let cli = Self::parse();
        Command::run(&cli).await
    }
}
