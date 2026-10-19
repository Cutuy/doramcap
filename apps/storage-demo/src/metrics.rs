//! Bounded rolling timing windows. Sampling timestamps and arrival timestamps
//! are measured separately; missing sequence numbers remain explicit.
use serde_json::{Value, json};
pub struct Window {
    values: [f64; 256],
    count: usize,
    next: usize,
}
impl Default for Window {
    fn default() -> Self {
        Self {
            values: [0.0; 256],
            count: 0,
            next: 0,
        }
    }
}
impl Window {
    pub fn push(&mut self, v: f64) {
        self.values[self.next] = v;
        self.next = (self.next + 1) % 256;
        self.count = (self.count + 1).min(256);
    }
    pub fn json(&self) -> Value {
        if self.count == 0 {
            return json!({"count":0,"lastMs":null,"p99Ms":null,"maxMs":null});
        }
        let mut sorted = self.values;
        sorted[..self.count].sort_unstable_by(f64::total_cmp);
        json!({"count":self.count,"lastMs":self.values[(self.next+255)%256],"p99Ms":sorted[(self.count*99).div_ceil(100)-1],"maxMs":sorted[self.count-1]})
    }
}
#[derive(Default)]
pub struct CpuTiming {
    previous: Option<(u64, u64, u64)>,
    sample: Window,
    arrival: Window,
    gaps: u64,
    interval_ms: Option<f64>,
}
impl CpuTiming {
    pub fn observe(&mut self, seq: u64, capture: u64, arrival: u64) {
        if let Some((prev_seq, prev_capture, prev_arrival)) = self.previous
            && seq > prev_seq
            && capture >= prev_capture
            && arrival >= prev_arrival
        {
            let delta = seq - prev_seq;
            self.gaps += delta - 1;
            let sample_ms = (capture - prev_capture) as f64 / 1e6;
            self.interval_ms = Some(sample_ms);
            self.sample.push((sample_ms - 100.0 * delta as f64).abs());
            self.arrival
                .push(((arrival - prev_arrival) as f64 / 1e6 - 100.0 * delta as f64).abs());
        }
        self.previous = Some((seq, capture, arrival));
    }
    pub fn json(&self) -> Value {
        json!({"sample":self.sample.json(),"arrival":self.arrival.json(),"gaps":self.gaps,"intervalMs":self.interval_ms})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn jitter_and_gaps_are_separate() {
        let mut t = CpuTiming::default();
        t.observe(0, 0, 0);
        t.observe(1, 103_000_000, 150_000_000);
        assert_eq!(t.json()["sample"]["lastMs"], 3.0);
        assert_eq!(t.json()["arrival"]["lastMs"], 50.0);
        t.observe(3, 303_000_000, 350_000_000);
        assert_eq!(t.json()["gaps"], 1);
        assert_eq!(t.json()["sample"]["lastMs"], 0.0);
    }
    #[test]
    fn window_evicts_old_spikes() {
        let mut w = Window::default();
        w.push(999.0);
        for _ in 0..256 {
            w.push(2.0);
        }
        assert_eq!(w.json()["count"], 256);
        assert_eq!(w.json()["maxMs"], 2.0);
        assert_eq!(w.json()["p99Ms"], 2.0);
    }
}
