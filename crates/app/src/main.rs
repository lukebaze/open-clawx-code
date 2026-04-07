use clap::Parser;

/// OCX — Open `ClawX` Code terminal
#[derive(Parser)]
#[command(name = "ocx", version, about = "Pure Rust coding terminal")]
#[allow(clippy::struct_field_names)]
struct Cli {
    /// Run in CLI mode (line-oriented REPL, no TUI)
    #[arg(long)]
    cli: bool,

    /// Start HTTP+SSE server for headless mode
    #[arg(long)]
    server: bool,

    /// Server bind address (default: 127.0.0.1:4200)
    #[arg(long, default_value = "127.0.0.1:4200")]
    bind: String,

    /// Connect TUI to a remote OCX server
    #[arg(long)]
    connect: Option<String>,

    /// Resume the latest session (optionally specify session ID)
    #[arg(long)]
    resume: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.cli {
        run_cli_mode();
        return Ok(());
    }

    if cli.server {
        return ocx_server::start_server(&cli.bind).await;
    }

    if let Some(_url) = cli.connect {
        // Remote TUI mode — connect to server via HTTP+SSE
        // TODO: wire RemoteClient into TUI event loop
        println!("Remote TUI mode not yet fully wired. Use --server on the host.");
        return Ok(());
    }

    ocx_tui::run().await
}

/// Minimal CLI fallback — line-oriented REPL.
fn run_cli_mode() {
    println!("ocx {} (CLI mode)", env!("CARGO_PKG_VERSION"));
    println!("Type your message and press Enter. Ctrl+C to quit.\n");

    let stdin = std::io::stdin();
    let mut input = String::new();

    loop {
        input.clear();
        eprint!("> ");
        match stdin.read_line(&mut input) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let trimmed = input.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "/quit" || trimmed == "/exit" {
            break;
        }
        println!("[assistant] I received: {trimmed}\n");
    }
}
