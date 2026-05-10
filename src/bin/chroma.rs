//! `chroma` — CLI client.
//!
//! Parses a single NOTA record from argv, archives it as a
//! [`chroma::Request`], sends it length-prefixed over the
//! daemon's UDS at `$XDG_RUNTIME_DIR/chroma.sock`, reads a
//! length-prefixed [`chroma::Response`], and prints the reply
//! as NOTA.

use std::io::{IsTerminal, Write};

fn main() -> chroma::Result<()> {
    let mut args = std::env::args().skip(1);
    let request_text = args.next().unwrap_or_else(|| {
        eprintln!("usage: chroma '(SetWarmth Warm)' | '(SetWarmthKelvin 3500)' | '(GetWarmth)' | …");
        std::process::exit(2);
    });

    let request = chroma::Request::from_nota(&request_text)?;
    let response = chroma::client::send(&request)?;
    match response {
        chroma::Response::Error { message } => Err(chroma::Error::Daemon { message }),
        response => {
            if let chroma::Request::SetTheme { mode } = request {
                if let Err(error) = apply_local_terminal_theme(mode) {
                    eprintln!("chroma local terminal theme error: {error}");
                }
            }
            println!("{}", response.to_nota()?);
            Ok(())
        }
    }
}

fn apply_local_terminal_theme(mode: chroma::ThemeMode) -> chroma::Result<()> {
    let mut stdout = std::io::stdout();
    if !stdout.is_terminal() {
        return Ok(());
    }

    let theme = chroma::ConfigFile::from_default_locations()?.theme_axis()?;
    let sequence = theme.palettes.for_mode(mode).terminal_osc_sequence();
    stdout.write_all(sequence.as_bytes())?;
    stdout.flush()?;
    Ok(())
}
