//! Optional source-time spans. Applications own the subscriber and capture runtime.
use crate::{
    file_offsets::FileRange,
    observability::{EventKind, Ids},
};

#[cfg_attr(not(feature = "activity-tracing"), derive(Default))]
pub(crate) struct Timeline {
    #[cfg(feature = "activity-tracing")]
    span: tracing::Span,
    #[cfg(feature = "activity-tracing")]
    operation: Option<tracing::Span>,
}
#[cfg(feature = "activity-tracing")]
impl Default for Timeline {
    fn default() -> Self {
        Self {
            #[cfg(feature = "activity-tracing")]
            span: tracing::Span::none(),
            #[cfg(feature = "activity-tracing")]
            operation: None,
        }
    }
}
impl Timeline {
    pub fn write(
        _ids: Ids,
        _path: &'static str,
        _range: FileRange,
        _direct: bool,
        _cause: &Self,
    ) -> Self {
        #[cfg(feature = "activity-tracing")]
        {
            let span = tracing::info_span!(target: "activity::storage", parent: None, "storage.write",
                writer = _ids.writer, file = _ids.file, write = _ids.write,
                path = _path, region = _ids.region, offset = _range.offset, len = _range.len, direct = _direct);
            span.follows_from(_cause.span.id());
            Self {
                span,
                operation: None,
            }
        }
        #[cfg(not(feature = "activity-tracing"))]
        Self::default()
    }
    pub fn combining(_writer: u64, _file: u64) -> Self {
        #[cfg(feature = "activity-tracing")]
        {
            Self {
                span: tracing::info_span!(target: "activity::storage", parent: None, "storage.combining", writer = _writer, file = _file),
                operation: None,
            }
        }
        #[cfg(not(feature = "activity-tracing"))]
        Self::default()
    }
    pub fn queued(&mut self, _operation: u64, _offset: u64, _len: usize) {
        #[cfg(feature = "activity-tracing")]
        {
            self.operation = Some(
                tracing::info_span!(target: "activity::storage", parent: &self.span, "storage.io", operation = _operation, offset = _offset, len = _len),
            );
        }
    }
    pub fn completed(&mut self, _result: i32) {
        #[cfg(feature = "activity-tracing")]
        if let Some(operation) = self.operation.take() {
            tracing::info!(target: "activity::storage", parent: &operation, result = _result, "cqe_observed");
        }
    }
    pub fn outcome(&self, _kind: EventKind) {
        #[cfg(feature = "activity-tracing")]
        tracing::info!(target: "activity::storage", parent: &self.span, outcome = ?_kind, "write_state");
    }
}
