use airpods_daemon::{app, cli::Cli};
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    cli.init_logging();
    app::run(cli).await
}
