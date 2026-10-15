//! Readings that glide to their new value instead of jumping, like the dashboard's gauges.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// How long a reading takes to reach its new value.
const GLIDE_TIME: Duration = Duration::from_millis(600);

#[derive(Default)]
pub struct Easing {
    glides: HashMap<&'static str, Glide>,
}

struct Glide {
    from: f64,
    to: f64,
    started: Instant,
}

impl Easing {
    /// The reading `key` on its way to `target`.
    pub fn value(&mut self, key: &'static str, target: f64) -> f64 {
        self.value_at(key, target, Instant::now())
    }

    fn value_at(&mut self, key: &'static str, target: f64, now: Instant) -> f64 {
        let glide = self.glides.entry(key).or_insert(Glide { from: target, to: target, started: now });
        if glide.to != target {
            *glide = Glide { from: glide.position(now), to: target, started: now };
        }
        glide.position(now)
    }
}

impl Glide {
    /// Eases out: quick at first, settling gently on the target.
    fn position(&self, now: Instant) -> f64 {
        let progress = (now.saturating_duration_since(self.started).as_secs_f64() / GLIDE_TIME.as_secs_f64()).min(1.0);
        let eased = 1.0 - (1.0 - progress).powi(3);
        self.from + (self.to - self.from) * eased
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_reading_is_shown_as_it_is() {
        let mut easing = Easing::default();
        assert_eq!(easing.value_at("load", 40.0, Instant::now()), 40.0);
    }

    #[test]
    fn a_new_reading_glides_from_the_old_one() {
        let mut easing = Easing::default();
        let start = Instant::now();
        easing.value_at("load", 0.0, start);
        assert_eq!(easing.value_at("load", 100.0, start), 0.0);
        let halfway = easing.value_at("load", 100.0, start + GLIDE_TIME / 2);
        assert!(halfway > 50.0 && halfway < 100.0);
        assert_eq!(easing.value_at("load", 100.0, start + GLIDE_TIME), 100.0);
    }
}
