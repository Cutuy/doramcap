//! Shared recording checks and cooperative recorded-run sequencing.
pub mod files;
/// Form-independent run contracts (`doramcap.launch.v1`).
pub mod launch {
    include!(concat!(env!("OUT_DIR"), "/doramcap.launch.v1.rs"));
}
pub mod recorded_run;
pub mod recording;
