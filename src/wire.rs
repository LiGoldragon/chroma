//! Length-prefixed framing on top of the rkyv archives.
//!
//! Wire format: `[u32 big-endian length][archive bytes]`.

use std::io::{Read, Write};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::{Error, Result};

/// Maximum frame size accepted off the wire (1 MiB). Frames
/// larger than this are rejected before allocation.
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;

/// Write `bytes` as a length-prefixed frame on a synchronous writer.
pub fn write_frame_blocking<W: Write>(writer: &mut W, bytes: &[u8]) -> Result<()> {
    let length = u32::try_from(bytes.len()).map_err(|_| Error::Daemon { message: "frame too large".into() })?;
    writer.write_all(&length.to_be_bytes())?;
    writer.write_all(bytes)?;
    writer.flush()?;
    Ok(())
}

/// Read one length-prefixed frame from a synchronous reader.
pub fn read_frame_blocking<R: Read>(reader: &mut R) -> Result<Vec<u8>> {
    let mut length_bytes = [0u8; 4];
    reader.read_exact(&mut length_bytes)?;
    let length = u32::from_be_bytes(length_bytes) as usize;
    if length > MAX_FRAME_BYTES {
        return Err(Error::Daemon { message: format!("frame too large: {length} bytes") });
    }
    let mut buffer = vec![0u8; length];
    reader.read_exact(&mut buffer)?;
    Ok(buffer)
}

/// Write `bytes` as a length-prefixed frame on an async writer.
pub async fn write_frame<W>(writer: &mut W, bytes: &[u8]) -> Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    let length = u32::try_from(bytes.len()).map_err(|_| Error::Daemon { message: "frame too large".into() })?;
    writer.write_all(&length.to_be_bytes()).await?;
    writer.write_all(bytes).await?;
    writer.flush().await?;
    Ok(())
}

/// Read one length-prefixed frame from an async reader.
pub async fn read_frame<R>(reader: &mut R) -> Result<Vec<u8>>
where
    R: AsyncReadExt + Unpin,
{
    let mut length_bytes = [0u8; 4];
    reader.read_exact(&mut length_bytes).await?;
    let length = u32::from_be_bytes(length_bytes) as usize;
    if length > MAX_FRAME_BYTES {
        return Err(Error::Daemon { message: format!("frame too large: {length} bytes") });
    }
    let mut buffer = vec![0u8; length];
    reader.read_exact(&mut buffer).await?;
    Ok(buffer)
}

/// The Unix-socket path the daemon listens on. Reads
/// `$CHROMA_SOCKET` if set; otherwise
/// `$XDG_RUNTIME_DIR/chroma.sock`; otherwise
/// `/tmp/chroma.sock`.
pub fn socket_path() -> std::path::PathBuf {
    if let Ok(explicit) = std::env::var("CHROMA_SOCKET") {
        return std::path::PathBuf::from(explicit);
    }
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        return std::path::PathBuf::from(runtime_dir).join("chroma.sock");
    }
    std::path::PathBuf::from("/tmp/chroma.sock")
}
