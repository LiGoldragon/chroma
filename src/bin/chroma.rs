//! `chroma` — CLI client.
//!
//! Embodies a single Datom record from argv, converts it to a
//! runtime-only rkyv frame, and sends it length-prefixed over the
//! daemon's UDS at `$XDG_RUNTIME_DIR/chroma.sock`, reads a
//! length-prefixed [`chroma::Response`], and prints the reply
//! as Datom.

use datom_codec::{Actualizable, IncorporationBudget, Potential, Textualizable};

fn main() -> chroma::Result<()> {
    let mut args = std::env::args().skip(1);
    let request_text = args.next().unwrap_or_else(|| {
        eprintln!("usage: chroma 'SetWarmth.{{Warm}}' | 'SetWarmthKelvin.{{3500}}' | 'GetWarmth' | …");
        std::process::exit(2);
    });

    let request = Potential::<chroma::generated::Request>::from(request_text.as_str())
        .actualize(IncorporationBudget::try_from(4096).expect("positive request budget"))
        .map_err(|error| chroma::Error::Config { message: format!("Datom request: {error:?}") })?;
    let request = chroma::Request::try_from(request)?;
    let response = chroma::client::send(&request)?;
    match response {
        chroma::Response::Error { message } => Err(chroma::Error::Daemon { message }),
        response => {
            let reply = chroma::generated::Reply::try_from(response)?;
            println!("{}", reply.textualize());
            Ok(())
        }
    }
}
