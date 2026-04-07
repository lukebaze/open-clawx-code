//! ASCII art splash screen shown on TUI startup.

/// ASCII art logo for open-clawx-code.
pub const SPLASH_ART: &str = r"
    +===========================================================+
    |                                                           |
    |      OPEN   CLAWX   CODE                                 |
    |                                                           |
    |      .d88b.   .o88b. db    db                             |
    |     .8P  Y8. d8P  Y8 `8b  d8'                             |
    |     88    88 8P       `8bd8'                               |
    |     88    88 8b        88                                  |
    |     `8b  d8' Y8b  d8 .d88b.                               |
    |      `Y88P'   `Y88P' ~Y88P~                               |
    |                                                           |
    |         >>> Pure Rust Coding Terminal <<<                  |
    |                                                           |
    +===========================================================+
";

/// Compact welcome message shown below the splash art.
pub const WELCOME_MSG: &str = "\
  Commands: /help  /model  /session  /clear  /quit
  Modes:    Ctrl+M toggle Plan/Build | Tab switch panels
  Navigate: i insert | Esc normal | j/k scroll | ? help
";
