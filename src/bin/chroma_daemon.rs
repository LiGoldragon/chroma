//! `chroma-daemon` — the daemon binary.
//!
//! Wires the gamma client (zbus → wl-gammarelay-rs) into the
//! socket server (UDS at `$XDG_RUNTIME_DIR/chroma.sock`), then
//! runs until SIGTERM.

#[tokio::main(flavor = "multi_thread")]
async fn main() -> chroma::Result<()> {
    chroma::daemon::run().await
}
