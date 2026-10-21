//! Host storage registered for direct CUDA kernel access.
//!
//! [`HostMappedMemory::new`] creates an anonymous `mmap`, calls
//! `cudaHostRegister(cudaHostRegisterMapped)`, and obtains its device address
//! through `cudaHostGetDevicePointer`. [`HostMappedMemory::new_with_mode`] also
//! supports opt-in `cudaMallocManaged` storage. CPU and GPU addresses refer to the same
//! storage; no device allocation or payload copy is performed.
//!
//! CPU guards and GPU leases enforce whole-buffer reader/writer exclusion.
//! GPU leases retain storage and access until stream completion, including on
//! early drop and unwind. Only arbitrary kernel submission is unsafe.
//! This first implementation waits on GPU-lease drop; it uses no host callbacks.

use std::{
    ffi::c_void,
    fmt, io,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

pub use cudarc::driver::{CudaContext, CudaStream};
use cudarc::{driver::DriverError, runtime::sys};
use memmap2::MmapMut;

/// Mapping, driver, or CUDA runtime failure.
#[derive(Debug)]
pub enum Error {
    /// Invalid size or failed anonymous mapping.
    Mapping(io::Error),
    /// Managed allocation requires managed memory and concurrent CPU/GPU access.
    ManagedMemoryUnsupported,
    /// Another lease has incompatible access.
    Busy,
    /// The stream belongs to another CUDA context.
    WrongContext,
    /// CUDA completion failed; access is permanently disabled.
    Poisoned,
    /// Context binding or synchronization failed.
    Driver(DriverError),
    /// A CUDA runtime operation failed; includes its name and status.
    Runtime(&'static str, sys::cudaError_t),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ManagedMemoryUnsupported => write!(
                f,
                "managed allocation requires managedMemory and concurrentManagedAccess"
            ),
            Self::Busy => write!(f, "memory is leased for incompatible access"),
            Self::WrongContext => write!(f, "stream belongs to another CUDA context"),
            Self::Poisoned => write!(f, "CUDA completion failed; mapping quarantined"),
            Self::Mapping(e) => write!(f, "host mapping: {e}"),
            Self::Driver(e) => write!(f, "CUDA context: {e}"),
            Self::Runtime(op, status) => write!(f, "{op}: {status:?}"),
        }
    }
}
impl std::error::Error for Error {}
impl From<DriverError> for Error {
    fn from(value: DriverError) -> Self {
        Self::Driver(value)
    }
}
fn runtime_result(op: &'static str, status: sys::cudaError_t) -> Result<(), Error> {
    if status == sys::cudaError::cudaSuccess {
        Ok(())
    } else {
        Err(Error::Runtime(op, status))
    }
}

/// Storage allocation strategy; registered anonymous host memory is the default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AllocationMode {
    /// `mmap` → `cudaHostRegister` → `cudaHostGetDevicePointer`.
    #[default]
    Registered,
    /// `cudaMallocManaged`; requires concurrent managed access support.
    /// CUDA may migrate pages between CPU and GPU memory.
    Managed,
}

enum Backing {
    Registered(MmapMut),
    Managed,
}

struct Registration {
    backing: Option<Backing>,
    context: Arc<CudaContext>,
    device_ptr: u64,
    host_addr: usize,
    len: usize,
}

impl Registration {
    /// Allocate `len` zero-initialized bytes in `context` using the selected backend.
    ///
    /// Binds the context on the calling thread. Requires CUDA host-registration
    /// and mapped-memory support. CUDA errors are returned without a fallback.
    /// `len` must be nonzero and at most `isize::MAX`. Anonymous mappings are
    /// page-aligned; the logical length need not be a page multiple.
    fn new(context: Arc<CudaContext>, len: usize, mode: AllocationMode) -> Result<Self, Error> {
        validate_len(len)?;
        context.bind_to_thread()?;
        if mode == AllocationMode::Managed {
            use cudarc::driver::sys::CUdevice_attribute as Attr;
            check_managed_support(
                context.attribute(Attr::CU_DEVICE_ATTRIBUTE_MANAGED_MEMORY)?,
                context.attribute(Attr::CU_DEVICE_ATTRIBUTE_CONCURRENT_MANAGED_ACCESS)?,
            )?;
            let mut ptr = std::ptr::null_mut();
            // SAFETY: CUDA initializes the out-pointer; this owner exclusively
            // owns the returned allocation and frees it only after completion.
            runtime_result("cudaMallocManaged", unsafe {
                sys::cudaMallocManaged(&mut ptr, len, sys::cudaMemAttachGlobal)
            })?;
            // SAFETY: allocation is live, exclusive, CPU accessible, and len
            // bytes long. Initialize every byte before exposing safe CPU reads.
            unsafe {
                ptr.cast::<u8>().write_bytes(0, len);
            }
            return Ok(Self {
                backing: Some(Backing::Managed),
                context,
                device_ptr: ptr as u64,
                host_addr: ptr as usize,
                len,
            });
        }
        let mapping = MmapMut::map_anon(len).map_err(Error::Mapping)?;
        Self::register_mapping(context, mapping)
    }

    fn register_mapping(context: Arc<CudaContext>, mut mapping: MmapMut) -> Result<Self, Error> {
        let len = mapping.len();
        validate_len(len)?;
        context.bind_to_thread()?;
        let host_ptr = mapping.as_mut_ptr().cast::<c_void>();
        // SAFETY: the owned mmap is valid, writable and remains mapped until
        // successful unregister. No pointer has escaped to another user.
        runtime_result("cudaHostRegister", unsafe {
            sys::cudaHostRegister(host_ptr, len, sys::cudaHostRegisterMapped)
        })?;
        // Construct the owner before querying the alias, so query failure also
        // unregisters (or retains storage if cleanup fails).
        let mut owner = Self {
            backing: Some(Backing::Registered(mapping)),
            context,
            device_ptr: 0,
            host_addr: host_ptr as usize,
            len,
        };
        let mut device_ptr = std::ptr::null_mut();
        // SAFETY: host_ptr refers to the registered mapping in the bound context.
        runtime_result("cudaHostGetDevicePointer", unsafe {
            sys::cudaHostGetDevicePointer(&mut device_ptr, host_ptr, 0)
        })?;
        owner.device_ptr = device_ptr as u64;
        Ok(owner)
    }

    fn release(&mut self) -> Result<(), Error> {
        if let Some(backing) = &mut self.backing {
            self.context.synchronize()?;
            match backing {
                Backing::Registered(mapping) => {
                    // SAFETY: owning context is synchronized; registration is ours.
                    runtime_result("cudaHostUnregister", unsafe {
                        sys::cudaHostUnregister(mapping.as_mut_ptr().cast())
                    })?;
                }
                Backing::Managed => {
                    // SAFETY: all permitted accesses completed, and this pointer
                    // was returned by cudaMallocManaged in the retained context.
                    runtime_result("cudaFree", unsafe {
                        sys::cudaFree(self.host_addr as *mut c_void)
                    })?;
                }
            }
            self.backing.take(); // Registered mmap is unmapped after unregister.
        }
        Ok(())
    }

    fn retain_on_failure(&mut self) {
        if let Some(backing) = self.backing.take() {
            // Managed storage remains allocated because cudaFree was not successful.
            std::mem::forget(backing);
            std::mem::forget(Arc::clone(&self.context));
        }
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        if self.release().is_err() {
            self.retain_on_failure();
        }
    }
}

/// Access requested for a CUDA submission. Readers can overlap CPU/GPU readers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    /// Kernels may only read this mapping.
    Read,
    /// Kernels may read and write; excludes every other lease.
    Write,
}

#[derive(Default)]
struct AccessState {
    readers: usize,
    writer: bool,
    poisoned: bool,
}
impl AccessState {
    fn acquire(&mut self, access: Access) -> Result<(), Error> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        match access {
            Access::Read if !self.writer => {
                self.readers = self.readers.checked_add(1).ok_or(Error::Busy)?;
            }
            Access::Write if !self.writer && self.readers == 0 => self.writer = true,
            _ => return Err(Error::Busy),
        }
        Ok(())
    }
    fn release(&mut self, access: Access) {
        match access {
            Access::Read => self.readers -= 1,
            Access::Write => self.writer = false,
        }
    }
}

struct Inner {
    registration: Registration,
    state: Mutex<AccessState>,
}
impl Inner {
    fn state(&self) -> std::sync::MutexGuard<'_, AccessState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Cloneable owner of CPU/GPU-accessible memory with checked leases.
///
/// # Concurrency and ownership
/// [`Self::read`] and [`Self::write`] return safe CPU slices through guards.
/// [`Self::gpu`] reserves access before CUDA submission. Multiple readers may
/// coexist; a writer excludes all readers and writers across CPU and GPU.
/// Conflicting acquisitions return [`Error::Busy`] without blocking.
///
/// Each guard owns a reference to the mapping. Dropping this handle never revokes
/// outstanding guards. A forgotten guard leaks its lease, preventing unsafe reuse.
/// GPU completion errors quarantine the allocation and reject subsequent access.
/// These guarantees cover this process and kernels satisfying [`GpuLease::enqueue`].
///
/// # Cleanup and environment
/// GPU-lease drop waits for its stream. Final allocation cleanup synchronizes its
/// context, then unregisters/unmaps or frees managed storage. Do not drop GPU leases or final owners in a
/// CUDA host callback. Cleanup failure retains storage and context indefinitely.
/// CUDA libraries load dynamically; cudarc panics if required libraries are absent.
#[derive(Clone)]
pub struct HostMappedMemory(Arc<Inner>);
impl HostMappedMemory {
    /// Create `len` zeroed bytes with `mmap`, register them and obtain a GPU alias.
    ///
    /// Requires mapped-host-memory support in `context`. Returns mapping or CUDA
    /// errors; rejects zero and lengths above `isize::MAX`. Binds `context` on the
    /// calling thread. Registration uses `cudaHostRegisterMapped`.
    pub fn new(context: Arc<CudaContext>, len: usize) -> Result<Self, Error> {
        Self::new_with_mode(context, len, AllocationMode::Registered)
    }

    /// Allocate zeroed storage with an explicit backing strategy.
    ///
    /// `Registered` follows [`Self::new`]. `Managed` uses `cudaMallocManaged`
    /// with global attachment and frees with `cudaFree`; it does not register
    /// or unmap host pages. Managed mode requires both CUDA managed-memory and
    /// concurrent-managed-access capability, otherwise returns
    /// [`Error::ManagedMemoryUnsupported`]. This preserves simultaneous CPU/GPU
    /// read leases. CUDA controls page residency/migration; guards are unchanged.
    /// All GPU uses remain restricted to the retained context.
    pub fn new_with_mode(
        context: Arc<CudaContext>,
        len: usize,
        mode: AllocationMode,
    ) -> Result<Self, Error> {
        Ok(Self(Arc::new(Inner {
            registration: Registration::new(context, len, mode)?,
            state: Mutex::new(AccessState::default()),
        })))
    }

    /// Register an existing owned mmap, preserving its contents.
    ///
    /// # Safety
    /// External mappings/processes must not truncate the backing file, alter its
    /// registration, or access bytes in conflict with this owner's guards. The
    /// caller must enforce cross-process lifetimes and read/write synchronization.
    /// The mapping must be writable ordinary RAM (not MMIO), and not already
    /// CUDA-registered in this process. This owner unregisters before unmapping.
    pub unsafe fn from_mapping(context: Arc<CudaContext>, mapping: MmapMut) -> Result<Self, Error> {
        Ok(Self(Arc::new(Inner {
            registration: Registration::register_mapping(context, mapping)?,
            state: Mutex::new(AccessState::default()),
        })))
    }

    /// Logical size in bytes.
    pub fn len(&self) -> usize {
        self.0.registration.len
    }
    /// Always false; zero-sized registrations are rejected.
    pub fn is_empty(&self) -> bool {
        false
    }

    fn acquire(&self, access: Access) -> Result<Permit, Error> {
        self.0.state().acquire(access)?;
        Ok(Permit {
            inner: self.0.clone(),
            access,
        })
    }

    /// Acquire CPU read access; fails with `Busy` while a writer holds a lease.
    pub fn read(&self) -> Result<CpuReadGuard, Error> {
        Ok(CpuReadGuard(self.acquire(Access::Read)?))
    }
    /// Acquire exclusive CPU access; fails with `Busy` while any lease exists.
    pub fn write(&self) -> Result<CpuWriteGuard, Error> {
        Ok(CpuWriteGuard(self.acquire(Access::Write)?))
    }
    /// Reserve access for `stream` before submitting any work.
    ///
    /// Returns `WrongContext` for a foreign context, `Busy` for conflicting use,
    /// or `Poisoned` after a completion failure. The returned lease owns the
    /// mapping and stream. CPU readers and GPU readers may overlap.
    pub fn gpu(&self, stream: Arc<CudaStream>, access: Access) -> Result<GpuLease, Error> {
        if stream.context().as_ref() != self.0.registration.context.as_ref() {
            return Err(Error::WrongContext);
        }
        Ok(GpuLease {
            permit: Some(self.acquire(access)?),
            stream,
            submitted: false,
        })
    }
}

struct Permit {
    inner: Arc<Inner>,
    access: Access,
}
impl Drop for Permit {
    fn drop(&mut self) {
        self.inner.state().release(self.access);
    }
}
impl Permit {
    fn bytes(&self) -> &[u8] {
        let reg = &self.inner.registration;
        // SAFETY: permit keeps mapping alive and excludes writers. With a write
        // permit, borrowing self also excludes mutable CPU access through it.
        unsafe { std::slice::from_raw_parts(reg.host_addr as *const u8, reg.len) }
    }
}

/// CPU read access. Dropping the guard releases its runtime read reservation.
///
/// Read guards cannot mutate the bytes:
/// ```compile_fail
/// fn mutate(mut read: cuda_host_memory::CpuReadGuard) { read[0] = 1; }
/// ```
/// A slice cannot outlive its guard:
/// ```compile_fail
/// fn escape(memory: &cuda_host_memory::HostMappedMemory) -> &[u8] {
///     let guard = memory.read().unwrap();
///     &guard[..]
/// }
/// ```
pub struct CpuReadGuard(Permit);
impl Deref for CpuReadGuard {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.0.bytes()
    }
}

/// Exclusive CPU access. Dropping the guard releases its write reservation.
pub struct CpuWriteGuard(Permit);
impl Deref for CpuWriteGuard {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.0.bytes()
    }
}
impl DerefMut for CpuWriteGuard {
    fn deref_mut(&mut self) -> &mut [u8] {
        let reg = &self.0.inner.registration;
        // SAFETY: write permit excludes every other CPU/GPU lease; &mut self
        // excludes simultaneous references created through this same guard.
        unsafe { std::slice::from_raw_parts_mut(reg.host_addr as *mut u8, reg.len) }
    }
}

/// Stream-use reservation retaining storage until GPU completion.
///
/// `wait()` reports completion errors. Drop also waits, including during panic
/// unwinding, so an early return cannot release memory still used by the stream.
/// Forgetting a lease retains its allocation and reservation indefinitely.
/// Waiting covers all work currently queued on the associated stream.
///
/// Leases cannot be cloned into untracked stream uses:
/// ```compile_fail
/// fn duplicate(lease: cuda_host_memory::GpuLease) { let other = lease.clone(); }
/// ```
/// Arbitrary kernel submission requires an explicit unsafe boundary:
/// ```compile_fail
/// fn submit(mut lease: cuda_host_memory::GpuLease) {
///     lease.enqueue(|_, _| Ok(())).unwrap();
/// }
/// ```
#[must_use = "keep the lease while submitting; wait or drop to retire GPU use"]
pub struct GpuLease {
    permit: Option<Permit>,
    stream: Arc<CudaStream>,
    submitted: bool,
}
impl GpuLease {
    /// Submit work using this lease's device pointer and stream.
    ///
    /// May be called repeatedly to enqueue a series of operations. The access
    /// reservation remains held after this function returns, even on an error.
    /// Unwinding also preserves it until stream synchronization during drop.
    ///
    /// # Safety
    /// The closure may use the supplied address only to enqueue bounded accesses
    /// to this allocation on the supplied stream. It must not retain the pointer,
    /// submit its use from another thread/stream, free/unregister it, or arrange
    /// later submissions after the closure returns. `Access::Read` forbids writes.
    /// All kernel arguments, launch dimensions and accesses to other allocations
    /// must independently be valid. CUDA host callbacks must not acquire these
    /// guards or wait on this lease. Arbitrary kernel code is not bounds-checked.
    pub unsafe fn enqueue(
        &mut self,
        submit: impl FnOnce(u64, &CudaStream) -> Result<(), DriverError>,
    ) -> Result<(), DriverError> {
        self.submitted = true; // before the closure: errors/panics may follow a launch.
        submit(
            self.permit
                .as_ref()
                .expect("live lease")
                .inner
                .registration
                .device_ptr,
            &self.stream,
        )
    }

    /// Wait for the stream, then release access. On failure quarantine storage.
    pub fn wait(mut self) -> Result<(), Error> {
        self.complete()
    }

    fn complete(&mut self) -> Result<(), Error> {
        if self.permit.is_none() {
            return Ok(());
        }
        if self.submitted
            && let Err(error) = self.stream.synchronize()
        {
            let permit = self.permit.take().expect("live lease");
            permit.inner.state().poisoned = true;
            // Completion is unknown. Keep storage, registration and access
            // reserved permanently; a later safe acquisition must fail.
            std::mem::forget(permit);
            return Err(error.into());
        }
        self.permit.take();
        Ok(())
    }
}
impl Drop for GpuLease {
    fn drop(&mut self) {
        let _ = self.complete();
    }
}

fn check_managed_support(managed: i32, concurrent: i32) -> Result<(), Error> {
    if managed != 0 && concurrent != 0 {
        Ok(())
    } else {
        Err(Error::ManagedMemoryUnsupported)
    }
}

fn validate_len(len: usize) -> Result<(), Error> {
    if len == 0 || len > isize::MAX as usize {
        Err(Error::Mapping(io::Error::new(
            io::ErrorKind::InvalidInput,
            "length must be in 1..=isize::MAX",
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reader_writer_exclusion_and_quarantine() {
        // Arrange
        let mut state = AccessState::default();
        state.acquire(Access::Read).unwrap();
        state.acquire(Access::Read).unwrap();

        // Act
        let while_two_readers = state.acquire(Access::Write);
        state.release(Access::Read);
        let while_one_reader = state.acquire(Access::Write);

        // Assert
        assert!(matches!(while_two_readers, Err(Error::Busy)));
        assert!(matches!(while_one_reader, Err(Error::Busy)));

        // Act
        state.release(Access::Read);
        state.acquire(Access::Write).unwrap();
        let read_while_writer = state.acquire(Access::Read);
        let write_while_writer = state.acquire(Access::Write);

        // Assert
        assert!(matches!(read_while_writer, Err(Error::Busy)));
        assert!(matches!(write_while_writer, Err(Error::Busy)));

        // Act
        state.release(Access::Write);
        state.poisoned = true;
        let poisoned_read = state.acquire(Access::Read);
        let poisoned_write = state.acquire(Access::Write);

        // Assert
        assert!(matches!(poisoned_read, Err(Error::Poisoned)));
        assert!(matches!(poisoned_write, Err(Error::Poisoned)));
    }

    #[test]
    fn reject_invalid_sizes() {
        // Arrange
        let cases = [(0, false), (usize::MAX, false), (4097, true)];
        for (len, expected_valid) in cases {
            // Act
            let result = validate_len(len);

            // Assert
            assert_eq!(result.is_ok(), expected_valid, "length {len}");
        }
    }

    #[test]
    fn managed_capability_requirements() {
        // Arrange
        let cases = [(0, 0, false), (1, 0, false), (0, 1, false), (1, 1, true)];
        for (managed, concurrent, supported) in cases {
            // Act
            let result = check_managed_support(managed, concurrent);
            // Assert
            assert_eq!(result.is_ok(), supported);
            if !supported {
                assert!(matches!(result, Err(Error::ManagedMemoryUnsupported)));
            }
        }
    }
}
