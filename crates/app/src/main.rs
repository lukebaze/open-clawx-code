use clap::Parser;

/// OCX — Open `ClawX` Code terminal
#[derive(Parser)]
#[command(name = "ocx", version, about = "Pure Rust coding terminal")]
#[allow(clippy::struct_field_names)]
struct Cli {
    /// Run in CLI mode (line-oriented REPL, no TUI)
    #[arg(long)]
    cli: bool,

    /// Start in server mode (stub)
    #[arg(long)]
    server: bool,

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
        println!("server mode not yet implemented");
        return Ok(());
    }

    ocx_tui::run().await
}

/// Minimal CLI fallback — line-oriented REPL using rustyline-style loop.
/// Shares the same session directory as the TUI.
fn run_cli_mode() {
    println!("ocx {} (CLI mode)", env!("CARGO_PKG_VERSION"));
    println!("Type your message and press Enter. Ctrl+C to quit.\n");

    let stdin = std::io::stdin();
    let mut input = String::new();

    loop {
        input.clear();
        eprint!("> ");
        match stdin.read_line(&mut input) {
            Ok(0) | Err(_) => break, // EOF or error
            Ok(_) => {}
        }
        let trimmed = input.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "/quit" || trimmed == "/exit" {
            break;
        }
        // Stub: echo back (real provider integration in Phase 07)
        println!("[assistant] I received: {trimmed}\n");
    }
}
