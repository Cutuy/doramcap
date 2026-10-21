use eyre::{Result, ensure};

/// Default slot-address alignment for compatibility with 4 KiB direct I/O.
/// Payload transfer length and storage completion ownership remain separate.
pub const DEFAULT_SLOT_ALIGNMENT: usize = 4096;

/// One calculation for local allocation and incoming manifest validation.
/// mmap supplies page-aligned bases; larger alignments need a different mapper.
pub(crate) fn slot_stride(capacity: usize, alignment: usize) -> Result<usize> {
    ensure!(capacity > 0, "slot capacity must be nonzero");
    ensure!(
        alignment.is_power_of_two(),
        "slot alignment must be a nonzero power of two"
    );
    // SAFETY: sysconf takes a constant selector and no pointers.
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    ensure!(
        page_size > 0 && alignment <= page_size as usize,
        "slot alignment exceeds mmap base alignment"
    );
    capacity
        .checked_next_multiple_of(alignment)
        .ok_or_else(|| eyre::eyre!("slot stride overflow"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checked_alignment_covers_padding_invalid_policy_and_overflow() {
        // Arrange
        let cases = [
            (4096, 4096, Some(4096)),
            (4097, 4096, Some(8192)),
            (4097, 64, Some(4160)),
            (4097, 1, Some(4097)),
            (0, 4096, None),
            (4096, 0, None),
            (4096, 3, None),
            (usize::MAX, 4096, None),
            (4096, 1 << 30, None),
        ];
        for (capacity, alignment, expected) in cases {
            // Act
            let result = slot_stride(capacity, alignment);
            // Assert
            assert_eq!(result.ok(), expected, "{capacity}/{alignment}");
        }
    }
}
