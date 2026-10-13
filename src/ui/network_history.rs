//! Recent network rates for the dashboard graphs, one sample per service update, and how far
//! the graphs have scrolled towards the next sample.

use crate::status::LiveStatus;
use std::time::{Duration, Instant};

pub const SAMPLE_COUNT: usize = 30;
/// Assumed until two updates have arrived; the service sends one about every second.
const DEFAULT_SAMPLE_INTERVAL: Duration = Duration::from_secs(1);
const MIN_SAMPLE_INTERVAL: Duration = Duration::from_millis(250);
const MAX_SAMPLE_INTERVAL: Duration = Duration::from_secs(3);

pub struct NetworkHistory {
    last_update_millis: f64,
    last_sample_at: Instant,
    sample_interval: Duration,
    download: Vec<f32>,
    upload: Vec<f32>,
}

impl Default for NetworkHistory {
    fn default() -> Self {
        Self {
            last_update_millis: 0.0,
            last_sample_at: Instant::now(),
            sample_interval: DEFAULT_SAMPLE_INTERVAL,
            download: vec![0.0; SAMPLE_COUNT],
            upload: vec![0.0; SAMPLE_COUNT],
        }
    }
}

impl NetworkHistory {
    /// Adds the rates from a new service update; frames that see the same update again are ignored.
    pub fn record(&mut self, status: &LiveStatus) {
        let updated = status.number("/updated_ms");
        if updated <= self.last_update_millis {
            return;
        }
        self.last_update_millis = updated;
        self.note_sample_time(Instant::now());
        push_sample(&mut self.download, status.number("/snapshot/network/download"));
        push_sample(&mut self.upload, status.number("/snapshot/network/upload"));
    }

    /// From 0 when a sample has just arrived to 1 when the next one is due.
    pub fn scroll(&self) -> f32 {
        self.scroll_at(Instant::now())
    }

    /// KB/s, oldest first.
    pub fn download(&self) -> &[f32] {
        &self.download
    }

    /// KB/s, oldest first.
    pub fn upload(&self) -> &[f32] {
        &self.upload
    }
}

impl NetworkHistory {
    fn note_sample_time(&mut self, now: Instant) {
        let since_last = now.saturating_duration_since(self.last_sample_at);
        self.sample_interval = since_last.clamp(MIN_SAMPLE_INTERVAL, MAX_SAMPLE_INTERVAL);
        self.last_sample_at = now;
    }

    fn scroll_at(&self, now: Instant) -> f32 {
        let elapsed = now.saturating_duration_since(self.last_sample_at);
        (elapsed.as_secs_f32() / self.sample_interval.as_secs_f32()).min(1.0)
    }
}

fn push_sample(samples: &mut Vec<f32>, value: f64) {
    samples.remove(0);
    samples.push(value as f32);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn status(updated_ms: u64, download: f64) -> LiveStatus {
        LiveStatus::from_value(json!({"updated_ms": updated_ms, "snapshot": {"network": {"download": download, "upload": 1}}}))
    }

    #[test]
    fn starts_flat_at_zero() {
        let history = NetworkHistory::default();
        assert_eq!(history.download().len(), SAMPLE_COUNT);
        assert!(history.upload().iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn records_each_update_once() {
        let mut history = NetworkHistory::default();
        history.record(&status(1000, 5.0));
        history.record(&status(1000, 5.0));
        history.record(&status(2000, 7.0));
        assert_eq!(&history.download()[SAMPLE_COUNT - 3..], &[0.0, 5.0, 7.0]);
        assert_eq!(history.download().len(), SAMPLE_COUNT);
    }

    #[test]
    fn scrolls_across_one_sample_interval() {
        let mut history = NetworkHistory::default();
        let start = Instant::now();
        history.note_sample_time(start);
        history.note_sample_time(start + Duration::from_secs(2));
        let last = start + Duration::from_secs(2);
        assert_eq!(history.scroll_at(last), 0.0);
        assert_eq!(history.scroll_at(last + Duration::from_secs(1)), 0.5);
        assert_eq!(history.scroll_at(last + Duration::from_secs(5)), 1.0);
    }
}
