//! `chroma` — CLI client.
//!
//! Parses a single NOTA record from argv, archives it as a
//! [`chroma::Request`], sends it length-prefixed over the
//! daemon's UDS at `$XDG_RUNTIME_DIR/chroma.sock`, reads a
//! length-prefixed [`chroma::Response`], and prints the reply
//! as NOTA.

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
            println!("{}", response.to_nota()?);
            Ok(())
        }
    }
}
