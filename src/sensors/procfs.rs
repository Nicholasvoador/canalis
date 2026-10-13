//! Readers for /proc counters and gauges. Missing files read as zero.

use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

const DISK_SECTOR_BYTES: u64 = 512;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CpuTimes {
    pub busy: u64,
    pub total: u64,
}

impl CpuTimes {
    pub fn load_since(&self, earlier: &CpuTimes) -> u64 {
        let elapsed = self.total.saturating_sub(earlier.total);
        if elapsed == 0 {
            return 0;
        }
        (100 * self.busy.saturating_sub(earlier.busy) + elapsed / 2) / elapsed
    }
}

pub fn read_cpu_times(root: &Path) -> CpuTimes {
    let text = read(root, "proc/stat");
    let values: Vec<u64> = text.lines().next().unwrap_or_default().split_whitespace().skip(1).filter_map(|value| value.parse().ok()).collect();
    if values.len() < 4 {
        return CpuTimes::default();
    }
    let idle = values[3] + values.get(4).copied().unwrap_or(0);
    let total: u64 = values.iter().take(8).sum();
    CpuTimes { busy: total - idle, total }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Memory {
    pub total_megabytes: u64,
    pub used_megabytes: u64,
    pub load_percent: u64,
}

pub fn read_memory(root: &Path) -> Memory {
    let fields = meminfo_kilobytes(root);
    let total = fields.get("MemTotal").copied().unwrap_or(0) / 1024;
    let available = fields.get("MemAvailable").or(fields.get("MemFree")).copied().unwrap_or(0) / 1024;
    let used = total.saturating_sub(available);
    let load = if total > 0 { (100 * used + total / 2) / total } else { 0 };
    Memory { total_megabytes: total, used_megabytes: used, load_percent: load }
}

fn meminfo_kilobytes(root: &Path) -> HashMap<String, u64> {
    read(root, "proc/meminfo")
        .lines()
        .filter_map(|line| {
            let (name, rest) = line.split_once(':')?;
            Some((name.to_string(), rest.split_whitespace().next()?.parse().ok()?))
        })
        .collect()
}

/// (received, sent) bytes across all interfaces except loopback.
pub fn read_network_bytes(root: &Path) -> (u64, u64) {
    read(root, "proc/net/dev")
        .lines()
        .skip(2)
        .filter_map(|line| line.split_once(':'))
        .filter(|(interface, _)| interface.trim() != "lo")
        .map(|(_, counters)| counters.split_whitespace().filter_map(|value| value.parse::<u64>().ok()).collect::<Vec<_>>())
        .filter(|values| values.len() > 8)
        .fold((0, 0), |(received, sent), values| (received + values[0], sent + values[8]))
}

/// (read, written) bytes across whole disks (partitions excluded).
pub fn read_disk_bytes(root: &Path) -> (u64, u64) {
    read(root, "proc/diskstats")
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .filter(|fields| fields.len() >= 10 && is_whole_disk(fields[2]))
        .fold((0, 0), |(read_bytes, written_bytes), fields| {
            let sectors = |index: usize| fields[index].parse::<u64>().unwrap_or(0) * DISK_SECTOR_BYTES;
            (read_bytes + sectors(5), written_bytes + sectors(9))
        })
}

pub fn is_whole_disk(name: &str) -> bool {
    let letters_after = |prefix: &str| name.strip_prefix(prefix).is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_lowercase()));
    let nvme_namespace = name
        .strip_prefix("nvme")
        .and_then(|rest| rest.split_once('n'))
        .is_some_and(|(controller, namespace)| is_number(controller) && is_number(namespace));
    let mmc = name.strip_prefix("mmcblk").is_some_and(is_number);
    letters_after("sd") || letters_after("vd") || letters_after("xvd") || nvme_namespace || mmc
}

fn is_number(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_digit())
}

pub fn read(root: &Path, relative: &str) -> String {
    std::fs::read_to_string(root.join(relative)).unwrap_or_default()
}

/// Turns monotonically increasing counters into per-second rates.
#[derive(Default)]
pub struct CounterRates {
    previous: HashMap<&'static str, (Vec<u64>, Instant)>,
}

impl CounterRates {
    pub fn update(&mut self, name: &'static str, counters: &[u64], now: Instant) -> Vec<f64> {
        let previous = self.previous.insert(name, (counters.to_vec(), now));
        let Some((old_counters, then)) = previous else {
            return vec![0.0; counters.len()];
        };
        let elapsed = now.duration_since(then).as_secs_f64();
        if elapsed <= 0.0 {
            return vec![0.0; counters.len()];
        }
        counters.iter().zip(old_counters).map(|(current, old)| current.saturating_sub(old) as f64 / elapsed).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensors::tests::FakeSystem;
    use std::time::Duration;

    #[test]
    fn memory_in_megabytes() {
        let system = FakeSystem::new("memory");
        assert_eq!(read_memory(&system.root), Memory { total_megabytes: 31801, used_megabytes: 13649, load_percent: 43 });
    }

    #[test]
    fn cpu_load_from_two_samples() {
        let system = FakeSystem::new("cpu");
        let first = read_cpu_times(&system.root);
        system.write("proc/stat", "cpu  130 0 130 840 0 0 0 0 0 0\n");
        assert_eq!(read_cpu_times(&system.root).load_since(&first), 60);
    }

    #[test]
    fn network_excludes_loopback() {
        let system = FakeSystem::new("network");
        assert_eq!(read_network_bytes(&system.root), (204800, 102400));
    }

    #[test]
    fn whole_disks_only() {
        for name in ["sda", "nvme0n1", "vdb", "mmcblk0"] {
            assert!(is_whole_disk(name), "{name}");
        }
        for name in ["sda1", "nvme0n1p2", "loop0", "mmcblk0p1", "dm-0"] {
            assert!(!is_whole_disk(name), "{name}");
        }
    }

    #[test]
    fn counter_rates_per_second() {
        let mut rates = CounterRates::default();
        let start = Instant::now();
        assert_eq!(rates.update("net", &[1000, 0], start), vec![0.0, 0.0]);
        assert_eq!(rates.update("net", &[3048, 1024], start + Duration::from_secs(2)), vec![1024.0, 512.0]);
    }

    #[test]
    fn missing_files_read_as_zero() {
        let empty = std::env::temp_dir().join("panorama-no-such-root");
        assert_eq!(read_memory(&empty), Memory::default());
        assert_eq!(read_cpu_times(&empty), CpuTimes::default());
    }
}
