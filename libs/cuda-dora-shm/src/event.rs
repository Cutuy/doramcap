use cuda_host_memory::{CudaContext, CudaStream};
use cudarc::runtime::sys;
use eyre::{Result, ensure};
use std::sync::Arc;

fn check(op: &str, code: sys::cudaError_t) -> Result<()> {
    ensure!(code == sys::cudaError::cudaSuccess, "{op}: {code:?}");
    Ok(())
}
/// Cached CUDA IPC event, never a memory handle. Exporter outlives all imports.
pub(crate) struct Event {
    raw: sys::cudaEvent_t,
    ctx: Arc<CudaContext>,
    imported: bool,
    close_export: bool,
}
impl Event {
    pub fn new(ctx: Arc<CudaContext>) -> Result<Self> {
        ctx.bind_to_thread()?;
        let mut raw = std::ptr::null_mut();
        // SAFETY: valid out-pointer and CUDA-defined IPC flags.
        check("create IPC event", unsafe {
            sys::cudaEventCreateWithFlags(
                &mut raw,
                sys::cudaEventDisableTiming | sys::cudaEventInterprocess,
            )
        })?;
        Ok(Self {
            raw,
            ctx,
            imported: false,
            close_export: false,
        })
    }
    pub fn open(ctx: Arc<CudaContext>, bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() == 64, "invalid IPC event handle");
        ctx.bind_to_thread()?;
        let handle = sys::cudaIpcEventHandle_t {
            reserved: std::array::from_fn(|i| bytes[i] as _),
        };
        let mut raw = std::ptr::null_mut();
        // SAFETY: trusted peer exports a live IPC event, validated handle length.
        check("open IPC event", unsafe {
            sys::cudaIpcOpenEventHandle(&mut raw, handle)
        })?;
        Ok(Self {
            raw,
            ctx,
            imported: true,
            close_export: false,
        })
    }
    pub fn export(&self) -> Result<Vec<u8>> {
        self.ctx.bind_to_thread()?;
        let mut handle = sys::cudaIpcEventHandle_t { reserved: [0; 64] };
        // SAFETY: owned event was created with IPC flags.
        check("export IPC event", unsafe {
            sys::cudaIpcGetEventHandle(&mut handle, self.raw)
        })?;
        Ok(handle
            .reserved
            .iter()
            .flat_map(|x| x.to_ne_bytes())
            .collect())
    }
    pub fn record(&self, stream: &CudaStream) -> Result<()> {
        ensure!(
            stream.context().as_ref() == self.ctx.as_ref(),
            "foreign stream context"
        );
        self.ctx.bind_to_thread()?;
        // SAFETY: event/stream alive in same context; caller gates generations.
        check("record event", unsafe {
            sys::cudaEventRecord(self.raw, stream.cu_stream().cast())
        })
    }
    pub fn wait(&self, stream: &CudaStream) -> Result<()> {
        ensure!(
            stream.context().as_ref() == self.ctx.as_ref(),
            "foreign stream context"
        );
        self.ctx.bind_to_thread()?;
        // SAFETY: record API completed before descriptor delivery; exporter retained.
        check("wait event", unsafe {
            sys::cudaStreamWaitEvent(stream.cu_stream().cast(), self.raw, 0)
        })
    }
    pub fn complete(&self) -> Result<bool> {
        self.ctx.bind_to_thread()?;
        // SAFETY: event remains live; only query after this generation was recorded.
        let code = unsafe { sys::cudaEventQuery(self.raw) };
        if code == sys::cudaError::cudaErrorNotReady {
            return Ok(false);
        }
        check("query event", code)?;
        Ok(true)
    }
    pub fn synchronize(&self) -> Result<()> {
        self.ctx.bind_to_thread()?;
        // SAFETY: exporter retains the recorded event until every reader retires.
        check("synchronize event", unsafe {
            sys::cudaEventSynchronize(self.raw)
        })
    }
    pub fn allow_close(&mut self) {
        self.close_export = true;
    }
}
impl Drop for Event {
    fn drop(&mut self) {
        if !self.imported && !self.close_export {
            // Abnormal teardown: remote imports may exist. Keep exporter alive.
            std::mem::forget(self.ctx.clone());
            return;
        }
        if self.ctx.bind_to_thread().is_ok() {
            // SAFETY: imports close first; exports close only after shutdown handshake.
            let _ = unsafe { sys::cudaEventDestroy(self.raw) };
        }
    }
}
