//! Mapping validity is independent of range reuse. No slice spans a shared header.
use crate::{
    observability::{self, EventData, EventKind, Ids, Observer},
    slabs::Rejected,
};
use memmap2::MmapMut;
use std::{
    fmt, io,
    ops::Range,
    ptr::NonNull,
    sync::{Arc, Mutex},
};

struct RegionInner {
    id: u64,
    ptr: NonNull<u8>,
    len: usize,
    owned: bool,
    active: Mutex<Vec<(u64, Range<usize>)>>,
    observer: Observer,
    _owner: Box<dyn Send + Sync>,
}
// SAFETY: owned ranges are exclusive under active; external leases require the
// importer's synchronization contract. No API exposes a slice over the region.
unsafe impl Send for RegionInner {}
unsafe impl Sync for RegionInner {}
#[derive(Clone)]
pub struct MmapRegion(Arc<RegionInner>);
impl fmt::Debug for MmapRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MmapRegion")
            .field("id", &self.id())
            .field("len", &self.len())
            .finish()
    }
}
impl MmapRegion {
    /// Consume a mapping with exclusive byte access. File-backed mappings retain
    /// memmap2's requirement that other aliases cannot modify/truncate the backing.
    pub fn new(mut map: MmapMut, observer: Observer) -> Self {
        let ptr = NonNull::new(map.as_mut_ptr()).expect("nonempty mapping");
        let len = map.len();
        Self(Arc::new(RegionInner {
            id: observability::next_id(),
            ptr,
            len,
            owned: true,
            active: Mutex::new(Vec::new()),
            observer,
            _owner: Box::new(map),
        }))
    }
    /// Retain a provider's mapping owner independently of its published blocks.
    ///
    /// # Safety
    /// `ptr..ptr+len` must be one stable, writable CPU mapping (len <= isize::MAX),
    /// valid until `owner` drops, including while registered with the kernel.
    /// No truncation, remapping or deallocation may invalidate it. Owner destruction
    /// must be safe on any thread. Data access requires a separate lease contract.
    pub unsafe fn import(
        ptr: NonNull<u8>,
        len: usize,
        owner: impl Send + Sync + 'static,
        observer: Observer,
    ) -> io::Result<Self> {
        if len == 0
            || len > isize::MAX as usize
            || (ptr.as_ptr() as usize).checked_add(len).is_none()
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        Ok(Self(Arc::new(RegionInner {
            id: observability::next_id(),
            ptr,
            len,
            owned: false,
            active: Mutex::new(Vec::new()),
            observer,
            _owner: Box::new(owner),
        })))
    }
    pub fn id(&self) -> u64 {
        self.0.id
    }
    pub fn len(&self) -> usize {
        self.0.len
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub(crate) fn ptr(&self) -> *mut u8 {
        self.0.ptr.as_ptr()
    }
    fn reserve(
        &self,
        range: Range<usize>,
        generation: u64,
        release: Box<dyn Send + Sync>,
    ) -> io::Result<Arc<Hold>> {
        if range.start >= range.end || range.end > self.len() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let mut active = self.0.active.lock().unwrap();
        if active
            .iter()
            .any(|(_, r)| range.start < r.end && r.start < range.end)
        {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let use_id = observability::next_id();
        active.push((use_id, range.clone()));
        drop(active);
        let hold = Arc::new(Hold {
            region: self.clone(),
            range,
            use_id,
            generation,
            _release: release,
        });
        hold.record(EventKind::SlabAcquired);
        Ok(hold)
    }
    /// Claim disjoint bytes from an exclusively owned mapping. Registration does
    /// not prevent a completed range from being acquired again.
    pub fn acquire(&self, range: Range<usize>) -> io::Result<WritableRange> {
        if !self.0.owned {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(WritableRange {
            hold: self.reserve(range, 0, Box::new(()))?,
        })
    }
    /// Lease a block published by an external provider (e.g. a shared ring).
    /// Rejection returns its reuse token intact.
    ///
    /// # Safety
    /// The range must contain initialized bytes. Producer publication must have
    /// been acquired with suitable cross-process synchronization. No alias may
    /// modify these bytes until the final clone releases `reuse`. Other ranges,
    /// including a ring header, may remain mutable. The reuse token's destructor
    /// must publish release safely on any thread and must not panic or block I/O.
    pub unsafe fn lease<T: Send + Sync + 'static>(
        &self,
        range: Range<usize>,
        generation: u64,
        reuse: T,
    ) -> Result<ReadLease, Rejected<T>> {
        // Keep the caller token recoverable until range reservation succeeds.
        let hold = match self.reserve(range, generation, Box::new(())) {
            Ok(h) => h,
            Err(error) => {
                return Err(Rejected {
                    error,
                    value: reuse,
                });
            }
        };
        let mut hold = hold;
        Arc::get_mut(&mut hold).unwrap()._release = Box::new(reuse);
        let len = hold.range.len();
        hold.record(EventKind::SlabSealed);
        Ok(ReadLease {
            hold,
            len,
            io_lease: false,
        })
    }
}
struct Hold {
    _release: Box<dyn Send + Sync>,
    region: MmapRegion,
    range: Range<usize>,
    use_id: u64,
    generation: u64,
}
impl Hold {
    fn record(&self, kind: EventKind) {
        self.region.0.observer.record(kind, || EventData {
            ids: self.ids(),
            offset: self.range.start as u64,
            len: self.range.len() as u64,
            ..Default::default()
        });
    }
    fn ids(&self) -> Ids {
        Ids {
            region: self.region.id(),
            slab_use: self.use_id,
            ..Default::default()
        }
    }
}
impl fmt::Debug for Hold {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RangeHold")
            .field("region", &self.region)
            .field("range", &self.range)
            .field("use_id", &self.use_id)
            .finish()
    }
}
impl Drop for Hold {
    fn drop(&mut self) {
        drop(std::mem::replace(&mut self._release, Box::new(())));
        self.region
            .0
            .active
            .lock()
            .unwrap()
            .retain(|(id, _)| *id != self.use_id);
        self.record(EventKind::SlabRecycled);
    }
}
/// Exclusive access to a disjoint range; sealing consumes the writable token.
///
/// ```compile_fail
/// use storage_persistence::mapped::WritableRange;
/// fn seal_while_borrowed(mut range: WritableRange) {
///     let bytes = range.bytes_mut();
///     let lease = range.seal(4096);
///     bytes[0] = 1;
/// }
/// ```
#[derive(Debug)]
pub struct WritableRange {
    hold: Arc<Hold>,
}
impl WritableRange {
    pub fn capacity(&self) -> usize {
        self.hold.range.len()
    }
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        // SAFETY: this unique token exclusively owns the reserved range.
        unsafe {
            std::slice::from_raw_parts_mut(
                self.hold.region.ptr().add(self.hold.range.start),
                self.capacity(),
            )
        }
    }
    pub fn seal(self, len: usize) -> Result<ReadLease, Rejected<Self>> {
        if len == 0 || len > self.capacity() {
            return Err(Rejected {
                error: io::ErrorKind::InvalidInput.into(),
                value: self,
            });
        }
        self.hold.record(EventKind::SlabSealed);
        Ok(ReadLease {
            io_lease: false,
            hold: self.hold,
            len,
        })
    }
}
#[derive(Debug)]
pub struct ReadLease {
    io_lease: bool,
    hold: Arc<Hold>,
    len: usize,
}
impl Clone for ReadLease {
    fn clone(&self) -> Self {
        self.hold.record(EventKind::ReaderCloned);
        Self {
            hold: self.hold.clone(),
            len: self.len,
            io_lease: false,
        }
    }
}
impl Drop for ReadLease {
    fn drop(&mut self) {
        self.hold.record(if self.io_lease {
            EventKind::IoLeaseReleased
        } else {
            EventKind::ReaderReleased
        });
    }
}
impl ReadLease {
    pub(crate) fn into_io(mut self) -> Self {
        self.io_lease = true;
        self.hold.record(EventKind::IoLeaseAcquired);
        self
    }

    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: owned seal or unsafe provider contract freezes initialized bytes.
        unsafe { std::slice::from_raw_parts(self.hold.region.ptr().add(self.offset()), self.len) }
    }
    pub fn region(&self) -> &MmapRegion {
        &self.hold.region
    }
    pub fn offset(&self) -> usize {
        self.hold.range.start
    }
    pub fn generation(&self) -> u64 {
        self.hold.generation
    }
    pub fn ids(&self) -> Ids {
        self.hold.ids()
    }
}
