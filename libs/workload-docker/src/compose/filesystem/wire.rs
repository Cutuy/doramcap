//! Length-delimited protobuf records for filesystem metadata and actions.
use super::{ChangeKind, FileChange, FileMetadata, Snapshot, StorageCategory};
use crate::{
    compose::{Result, error::invalid},
    filesystem_proto as pb,
};
use prost::Message;
use std::{
    collections::BTreeMap,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};

fn path(bytes: Vec<u8>) -> PathBuf {
    PathBuf::from(std::ffi::OsString::from_vec(bytes))
}
fn metadata(m: &FileMetadata) -> pb::FileMetadata {
    pb::FileMetadata {
        mode: m.mode,
        size: m.size,
        uid: m.uid,
        gid: m.gid,
        mtime_seconds: m.mtime.0,
        mtime_nanoseconds: m.mtime.1,
        ctime_seconds: m.ctime.0,
        ctime_nanoseconds: m.ctime.1,
        symlink: m
            .symlink
            .as_ref()
            .map(|p| p.as_os_str().as_bytes().to_vec()),
    }
}
fn decode_metadata(m: pb::FileMetadata) -> FileMetadata {
    FileMetadata {
        mode: m.mode,
        size: m.size,
        uid: m.uid,
        gid: m.gid,
        mtime: (m.mtime_seconds, m.mtime_nanoseconds),
        ctime: (m.ctime_seconds, m.ctime_nanoseconds),
        symlink: m.symlink.map(path),
    }
}

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

pub(super) fn change(row: &FileChange) -> pb::FileChange {
    pb::FileChange {
        category: match row.category {
            StorageCategory::HostBind => 1,
            StorageCategory::ContainerLayer => 2,
            StorageCategory::NamedVolume => 3,
        },
        source: row.source.clone(),
        consumers: row.consumers.clone(),
        path: row.path.as_os_str().as_bytes().to_vec(),
        kind: match row.kind {
            ChangeKind::Added => 1,
            ChangeKind::Modified => 2,
            ChangeKind::Deleted => 3,
        },
        before: row.before.as_ref().map(metadata),
        after: row.after.as_ref().map(metadata),
    }
}
pub(super) fn decode_change(row: pb::FileChange) -> Result<FileChange> {
    Ok(FileChange {
        category: match row.category {
            1 => StorageCategory::HostBind,
            2 => StorageCategory::ContainerLayer,
            3 => StorageCategory::NamedVolume,
            x => return Err(invalid(format!("unknown storage category: {x}"))),
        },
        source: row.source,
        consumers: row.consumers,
        path: path(row.path),
        kind: match row.kind {
            1 => ChangeKind::Added,
            2 => ChangeKind::Modified,
            3 => ChangeKind::Deleted,
            x => return Err(invalid(format!("unknown filesystem change kind: {x}"))),
        },
        before: row.before.map(decode_metadata),
        after: row.after.map(decode_metadata),
    })
}

pub(crate) async fn write_snapshot(file: &Path, snapshot: &Snapshot) -> Result<()> {
    let file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(file)
        .await?;
    let mut out = BufWriter::new(file);
    write_record(
        &mut out,
        &pb::SnapshotRecord {
            body: Some(pb::snapshot_record::Body::Header(pb::SnapshotHeader {
                complete: snapshot.complete,
                reason: snapshot.reason.clone(),
                entries: snapshot.entries.len() as u64,
            })),
        },
    )
    .await?;
    for (path, m) in &snapshot.entries {
        write_record(
            &mut out,
            &pb::SnapshotRecord {
                body: Some(pb::snapshot_record::Body::Entry(pb::SnapshotEntry {
                    path: path.as_os_str().as_bytes().to_vec(),
                    metadata: Some(metadata(m)),
                })),
            },
        )
        .await?;
    }
    out.flush().await?;
    Ok(())
}
pub(crate) async fn read_snapshot(file: &Path) -> Result<Snapshot> {
    let mut input = BufReader::new(tokio::fs::File::open(file).await?);
    let Some(pb::SnapshotRecord {
        body: Some(pb::snapshot_record::Body::Header(header)),
    }) = read_record(&mut input).await?
    else {
        return Err(invalid("snapshot header missing"));
    };
    let mut entries = BTreeMap::new();
    while let Some(record) = read_record::<_, pb::SnapshotRecord>(&mut input).await? {
        let Some(pb::snapshot_record::Body::Entry(entry)) = record.body else {
            return Err(invalid("unexpected snapshot record"));
        };
        let m = entry
            .metadata
            .ok_or_else(|| invalid("snapshot metadata missing"))?;
        if entries
            .insert(path(entry.path), decode_metadata(m))
            .is_some()
        {
            return Err(invalid("duplicate snapshot path"));
        }
    }
    if entries.len() as u64 != header.entries {
        return Err(invalid("truncated snapshot record stream"));
    }
    Ok(Snapshot {
        entries,
        complete: header.complete,
        reason: header.reason,
    })
}
