use prelude::*;

mod cli;
mod command;
mod prelude;
mod ui;

#[tokio::main]
async fn main() {
    match Cli::run().await {
        Ok(_) => {}
        Err(err) => {
            UiMessage::error(err);
            exit(1);
        }
    }
}
