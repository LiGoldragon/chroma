//! Synchronous client — the `chroma` CLI's send path.
//!
//! One-shot: connect, write request, read response, disconnect.
//! Synchronous so the CLI binary doesn't pull a tokio runtime.

use std::os::unix::net::UnixStream;

use crate::error::Result;
use crate::request::Request;
use crate::response::Response;
use crate::wire::{read_frame_blocking, socket_path, write_frame_blocking};

/// Send `request` to the running daemon and return its response.
pub fn send(request: &Request) -> Result<Response> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path)?;
    let archive = request.archive()?;
    write_frame_blocking(&mut stream, &archive)?;
    let reply_bytes = read_frame_blocking(&mut stream)?;
    Response::from_archive(&reply_bytes)
}
