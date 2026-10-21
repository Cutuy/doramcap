//! Protobuf contracts and checked translation for fault IPC and stored aggregation output.
//! Encoding runs in the host publication path, outside reporter callbacks.
mod convert;
mod publication;
pub use publication::*;
use std::time::Duration;

pub const VERSION: u32 = 1;
pub const REPORTER_SCHEMA_NAME: &str = "doramcap.fault.v1.ReporterPublication";
pub const AGGREGATION_SCHEMA_NAME: &str = "doramcap.fault.v1.AggregationPublication";
pub use fault_schema::{FILE_DESCRIPTOR_SET, pb};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Missing(&'static str),
    Invalid(&'static str),
    UnknownEnum(&'static str, i32),
    UnsupportedVersion(u32),
    DurationOverflow,
    Limit,
    Decode(String),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
/// Checked, lossless mapping. Use publication decoders for frame/cardinality limits.
pub trait Wire: Sized {
    type Proto;
    fn to_proto(&self) -> Result<Self::Proto>;
    fn from_proto(proto: Self::Proto) -> Result<Self>;
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}
fn required<T>(value: Option<T>, field: &'static str) -> Result<T> {
    value.ok_or(Error::Missing(field))
}
fn ns(value: Duration) -> Result<u64> {
    value
        .as_nanos()
        .try_into()
        .map_err(|_| Error::DurationOverflow)
}
fn id(value: u128) -> Result<Vec<u8>> {
    if value == 0 {
        return Err(Error::Invalid("zero instance ID"));
    }
    Ok(value.to_be_bytes().to_vec())
}
fn parse_id(value: Vec<u8>) -> Result<u128> {
    let bytes: [u8; 16] = value
        .try_into()
        .map_err(|_| Error::Invalid("instance ID length"))?;
    let value = u128::from_be_bytes(bytes);
    if value == 0 {
        return Err(Error::Invalid("zero instance ID"));
    }
    Ok(value)
}
fn ensure(condition: bool, field: &'static str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::Invalid(field))
    }
}
