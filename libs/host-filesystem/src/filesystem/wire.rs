//! Length-delimited protobuf records for filesystem metadata and actions.
use crate::{Result, error::invalid};
use prost::Message;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Write one protobuf message preceded by its varint length.
pub(crate) async fn write_record<W: AsyncWrite + Unpin, M: Message>(
    out: &mut W,
    record: &M,
) -> Result<()> {
    let mut bytes = Vec::new();
    record
        .encode_length_delimited(&mut bytes)
        .map_err(|e| invalid(format!("encode filesystem record: {e}")))?;
    out.write_all(&bytes).await?;
    Ok(())
}

/// Read a single protobuf record. Clean EOF and truncated frames are distinct.
pub(crate) async fn read_record<R: AsyncRead + Unpin, M: Message + Default>(
    input: &mut R,
) -> Result<Option<M>> {
    let mut length = 0u64;
    for i in 0..10 {
        let byte = match input.read_u8().await {
            Ok(b) => b,
            Err(e) if i == 0 && e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        if i == 9 && byte > 1 {
            return Err(invalid("invalid protobuf record length"));
        }
        length |= u64::from(byte & 0x7f) << (i * 7);
        if byte & 0x80 == 0 {
            let len = usize::try_from(length)
                .map_err(|_| invalid("protobuf record length exceeds address space"))?;
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(len)
                .map_err(|e| invalid(format!("allocating protobuf record: {e}")))?;
            bytes.resize(len, 0);
            input.read_exact(&mut bytes).await?;
            return M::decode(bytes.as_slice())
                .map(Some)
                .map_err(|e| invalid(format!("decode filesystem record: {e}")));
        }
    }
    Err(invalid("invalid protobuf record length"))
}
