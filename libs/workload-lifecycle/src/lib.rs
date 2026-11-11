//! Shared recording checks and cooperative recorded-run sequencing.
pub mod admission;
pub mod cgroup;
pub mod files;
/// Form-independent run contracts (`doramcap.launch.v1`).
pub mod launch {
    include!(concat!(env!("OUT_DIR"), "/doramcap.launch.v1.rs"));
}
pub mod recorded_run;
pub mod recording;
pub mod run;

pub mod plan;

pub mod dora;

impl launch::AppProcess {
    /// Reject invalid common process settings before backend preparation.
    pub fn validate(&self) -> Result<(), String> {
        let executable = self.argv.first().ok_or("app process needs an executable")?;
        if executable.is_empty()
            || executable == "."
            || executable == ".."
            || executable.contains(['/', '\\'])
        {
            return Err("app executable must be a basename".into());
        }
        if !self.working_directory.is_empty()
            && !std::path::Path::new(&self.working_directory).is_absolute()
        {
            return Err("app working directory must be absolute".into());
        }
        if !(0..=libc::SIGRTMAX()).contains(&self.stop_signal) {
            return Err("invalid app stop signal".into());
        }
        Ok(())
    }
}
