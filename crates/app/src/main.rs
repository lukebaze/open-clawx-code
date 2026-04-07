use clap::Parser;

/// OCX — Open `ClawX` Code terminal
#[derive(Parser)]
#[command(name = "ocx", version, about = "Pure Rust coding terminal")]
struct Cli {
    /// Run in CLI mode (print version and exit)
    #[arg(long)]
    cli: bool,

    /// Start in server mode (stub)
    #[arg(long)]
    server: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.cli {
        println!("ocx {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if cli.server {
        println!("server mode not yet implemented");
        return Ok(());
    }

    ocx_tui::run().await
}
