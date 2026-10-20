// Test producer stamps MCAP independently of the storage framing APIs.
pub fn stamp(out: &mut [u8], message: storage_persistence::mcap::Message, payload: &[u8]) -> usize {
    let used = 31 + payload.len();
    let mut len = used.next_multiple_of(4096);
    if len != used && len - used < 9 {
        len += 4096;
    }
    out[..len].fill(0);
    out[0] = 5;
    out[1..9].copy_from_slice(&((22 + payload.len()) as u64).to_le_bytes());
    out[9..11].copy_from_slice(&message.channel.to_le_bytes());
    out[11..15].copy_from_slice(&message.sequence.to_le_bytes());
    out[15..23].copy_from_slice(&message.log_time.to_le_bytes());
    out[23..31].copy_from_slice(&message.publish_time.to_le_bytes());
    out[31..used].copy_from_slice(payload);
    if len > used {
        out[used] = 0x80;
        out[used + 1..used + 9].copy_from_slice(&((len - used - 9) as u64).to_le_bytes());
    }
    len
}
