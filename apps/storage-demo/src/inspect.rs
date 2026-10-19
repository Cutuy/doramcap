//! Bounded incremental inspection of bytes read back from the file.
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::Path,
};

pub struct Inspector {
    cursor: u64,
    records: VecDeque<Value>,
}
impl Inspector {
    pub fn new() -> Self {
        Self {
            cursor: 8,
            records: VecDeque::with_capacity(80),
        }
    }
    pub fn read(&mut self, path: &Path, index: u64, sealed: bool) -> io::Result<Value> {
        let mut file = File::open(path)?;
        let size = file.metadata()?.len();
        if size >= 8 {
            let mut magic = [0; 8];
            file.read_exact(&mut magic)?;
            if &magic != b"\x89MCAP0\r\n" {
                return Err(io::ErrorKind::InvalidData.into());
            }
        }
        for _ in 0..256 {
            if self.cursor + 8 == size && sealed {
                file.seek(SeekFrom::Start(self.cursor))?;
                let mut magic = [0; 8];
                file.read_exact(&mut magic)?;
                if &magic == b"\x89MCAP0\r\n" {
                    self.push(json!({"offset":self.cursor,"len":8,"kind":"Magic / EOF"}));
                    self.cursor += 8;
                }
                break;
            }
            if self.cursor + 9 > size {
                break;
            }
            file.seek(SeekFrom::Start(self.cursor))?;
            let mut header = [0; 9];
            file.read_exact(&mut header)?;
            let body = u64::from_le_bytes(header[1..].try_into().unwrap());
            let Some(len) = body.checked_add(9) else {
                return Err(io::ErrorKind::InvalidData.into());
            };
            let Some(end) = self.cursor.checked_add(len) else {
                return Err(io::ErrorKind::InvalidData.into());
            };
            if end > size {
                break;
            }
            let kind = match header[0] {
                1 => "Header",
                2 => "Footer",
                3 => "Schema",
                4 => "Channel",
                5 => "Message",
                15 => "DataEnd",
                128 => "Alignment padding",
                _ => return Err(io::ErrorKind::InvalidData.into()),
            };
            let mut row = json!({"offset":self.cursor,"len":len,"kind":kind});
            if header[0] == 5 {
                if body < 22 {
                    return Err(io::ErrorKind::InvalidData.into());
                }
                let mut prefix = [0; 22];
                file.read_exact(&mut prefix)?;
                row["channel"] = json!(u16::from_le_bytes(prefix[..2].try_into().unwrap()));
                row["sequence"] = json!(u32::from_le_bytes(prefix[2..6].try_into().unwrap()));
                row["payload"] = json!(body - 22);
            }
            self.push(row);
            self.cursor = end;
        }
        let start = size.saturating_sub(256);
        file.seek(SeekFrom::Start(start))?;
        let mut tail = vec![0; (size - start) as usize];
        file.read_exact(&mut tail)?;
        Ok(
            json!({"index":index,"name":format!("capture-{index:04}.mcap"),"sealed":sealed,"size":size,
            "parsedThrough":self.cursor.min(size),"records":self.records,"tailOffset":start,"tail":tail}),
        )
    }
    fn push(&mut self, value: Value) {
        if self.records.len() == 80 {
            self.records.pop_front();
        }
        self.records.push_back(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incomplete_record_is_held_until_complete() {
        let path = std::env::temp_dir().join(format!("storage-inspect-{}", std::process::id()));
        let mut data = b"\x89MCAP0\r\n".to_vec();
        data.push(128);
        data.extend_from_slice(&4u64.to_le_bytes());
        data.extend_from_slice(&[0; 2]);
        std::fs::write(&path, &data).unwrap();
        let mut reader = Inspector::new();
        assert_eq!(reader.read(&path, 0, false).unwrap()["parsedThrough"], 8);
        data.extend_from_slice(&[0; 2]);
        std::fs::write(&path, &data).unwrap();
        assert_eq!(reader.read(&path, 0, false).unwrap()["parsedThrough"], 21);
        std::fs::remove_file(path).unwrap();
    }
}
