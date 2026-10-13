//! Recent network rates for the dashboard graphs, one sample per service update.

use crate::status::LiveStatus;

pub const SAMPLE_COUNT: usize = 30;

pub struct NetworkHistory {
    last_update_millis: f64,
    download: Vec<f32>,
    upload: Vec<f32>,
}

impl Default for NetworkHistory {
    fn default() -> Self {
        Self { last_update_millis: 0.0, download: vec![0.0; SAMPLE_COUNT], upload: vec![0.0; SAMPLE_COUNT] }
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
        push_sample(&mut self.download, status.number("/snapshot/network/download"));
        push_sample(&mut self.upload, status.number("/snapshot/network/upload"));
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
}
