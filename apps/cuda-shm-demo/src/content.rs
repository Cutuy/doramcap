//! Reproducible test bytes keyed by seed, frame sequence and byte offset.
//! This integer mixer is a test pattern, not a cryptographic random generator.
pub const DEFAULT_SEED: u32 = 0x1234_5678;
pub fn byte(index: u32, sequence: u32, seed: u32) -> u8 {
    let mut value = index ^ seed ^ sequence.wrapping_mul(0x9e37_79b9);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^= value >> 16;
    value as u8
}
/// Return the observed checksum and the first byte mismatch, if any.
pub fn inspect(bytes: &[u8], sequence: u32, seed: u32) -> (u64, Option<usize>) {
    let checksum = bytes.iter().map(|b| u64::from(*b)).sum();
    let mismatch = bytes
        .iter()
        .enumerate()
        .position(|(i, b)| *b != byte(i as u32, sequence, seed));
    (checksum, mismatch)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_seed_matches_reference_bytes_at_boundaries() {
        // Arrange
        let offsets = [0, 1, 255, 256, 4095, 4096, 65536];
        let expected = [123, 91, 52, 116, 84, 212, 208];
        // Act
        let actual = offsets.map(|i| byte(i, 42, DEFAULT_SEED));
        // Assert
        assert_eq!(actual, expected);
    }
    #[test]
    fn corruption_and_equal_sum_swaps_are_detected() {
        // Arrange
        let original: Vec<_> = (0..4097).map(|i| byte(i, 42, DEFAULT_SEED)).collect();
        // Act
        let good = inspect(&original, 42, DEFAULT_SEED);
        // Assert
        assert_eq!(good.1, None);
        // Act
        let mut swapped = original.clone();
        let other = original.iter().position(|b| *b != original[0]).unwrap();
        swapped.swap(0, other);
        let bad = inspect(&swapped, 42, DEFAULT_SEED);
        // Assert
        assert_eq!(bad.0, good.0, "a sum alone misses reordered bytes");
        assert_eq!(bad.1, Some(0));
        // Act
        let mut tail = original.clone();
        tail[4096] ^= 1;
        // Assert
        assert_eq!(inspect(&tail, 42, DEFAULT_SEED).1, Some(4096));
        assert!(inspect(&original, 43, DEFAULT_SEED).1.is_some());
        assert!(inspect(&original, 42, DEFAULT_SEED ^ 1).1.is_some());
    }
}
