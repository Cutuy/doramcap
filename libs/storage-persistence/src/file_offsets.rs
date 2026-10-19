//! Append-only range allocation. Abandoned reservations leave no holes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetError {
    InvalidAlignment,
    Unaligned,
    Empty,
    Overflow,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileRange {
    pub offset: u64,
    pub len: u64,
}
pub struct OffsetAllocator {
    next: u64,
    alignment: u64,
}
impl OffsetAllocator {
    pub fn new(start: u64, alignment: u64) -> Result<Self, OffsetError> {
        if !alignment.is_power_of_two() {
            return Err(OffsetError::InvalidAlignment);
        }
        if !start.is_multiple_of(alignment) {
            return Err(OffsetError::Unaligned);
        }
        Ok(Self {
            next: start,
            alignment,
        })
    }
    pub fn next_offset(&self) -> u64 {
        self.next
    }
    pub fn reserve(&mut self, len: u64) -> Result<Reservation<'_>, OffsetError> {
        if len == 0 {
            return Err(OffsetError::Empty);
        }
        if !len.is_multiple_of(self.alignment) {
            return Err(OffsetError::Unaligned);
        }
        let end = self.next.checked_add(len).ok_or(OffsetError::Overflow)?;
        let range = FileRange {
            offset: self.next,
            len,
        };
        Ok(Reservation {
            allocator: self,
            range,
            end,
        })
    }
}
/// Exclusive borrow serializes tentative reservations. Commit advances the cursor;
/// dropping the reservation leaves the cursor unchanged.
pub struct Reservation<'a> {
    allocator: &'a mut OffsetAllocator,
    range: FileRange,
    end: u64,
}
impl Reservation<'_> {
    pub fn range(&self) -> FileRange {
        self.range
    }
    pub fn commit(self) -> FileRange {
        self.allocator.next = self.end;
        self.range
    }
}
