//! Bounded rolling timing windows.
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
#[cfg(test)]
mod tests {
    use super::*;
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
