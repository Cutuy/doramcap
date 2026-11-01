//! Host CPU counters collected alongside the monitor's cgroup snapshots.
use monitor_core::CpuTotals;
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
};

pub(super) struct Reader {
    file: File,
    buffer: String,
}
impl Reader {
    pub(super) fn open() -> io::Result<Self> {
        Ok(Self {
            file: File::open("/proc/stat")?,
            buffer: String::with_capacity(16 * 1024),
        })
    }
    pub(super) fn sample(&mut self) -> io::Result<CpuTotals> {
        self.file.seek(SeekFrom::Start(0))?;
        self.buffer.clear();
        self.file.read_to_string(&mut self.buffer)?;
        parse(&self.buffer)
    }
}
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "invalid /proc/stat CPU counters",
    )
}
fn parse(text: &str) -> io::Result<CpuTotals> {
    let mut lines = text.lines();
    let mut aggregate = lines.next().ok_or_else(invalid)?.split_whitespace();
    if aggregate.next() != Some("cpu") {
        return Err(invalid());
    }
    let mut ticks = [0u64; 8];
    // Guest time is already included in user/nice; count the first eight fields.
    for tick in &mut ticks {
        *tick = aggregate
            .next()
            .ok_or_else(invalid)?
            .parse()
            .map_err(|_| invalid())?;
    }
    let total_ticks = ticks
        .iter()
        .try_fold(0u64, |sum, n| sum.checked_add(*n))
        .ok_or_else(invalid)?;
    let idle = ticks[3].checked_add(ticks[4]).ok_or_else(invalid)?;
    let cpu_cores = lines
        .filter(|line| {
            line.split_whitespace()
                .next()
                .and_then(|name| name.strip_prefix("cpu"))
                .is_some_and(|suffix| {
                    !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit())
                })
        })
        .count()
        .try_into()
        .map_err(|_| invalid())?;
    if cpu_cores == 0 {
        return Err(invalid());
    }
    Ok(CpuTotals {
        busy_ticks: total_ticks - idle,
        total_ticks,
        cpu_cores,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_counters_exclude_idle_iowait_and_duplicate_guest_time() {
        // Arrange
        let source = "cpu 10 20 30 40 50 60 70 80 90 100\ncpu0 1 2 3 4 5 6 7 8\ncpu1 1 2 3 4 5 6 7 8\nintr 123\n";
        // Act
        let totals = parse(source).unwrap();
        // Assert
        assert_eq!(
            (totals.busy_ticks, totals.total_ticks, totals.cpu_cores),
            (270, 360, 2)
        );
    }
    #[test]
    fn malformed_or_missing_counters_are_unavailable() {
        // Arrange / Act / Assert
        for source in [
            "",
            "cpu 1 2 3\n",
            "cpu x 0 0 0 0 0 0 0\ncpu0 0\n",
            "cpu 1 2 3 4 5 6 7 8\n",
            "cpu 18446744073709551615 1 0 0 0 0 0 0\ncpu0 0\n",
        ] {
            assert!(parse(source).is_err());
        }
    }
}
