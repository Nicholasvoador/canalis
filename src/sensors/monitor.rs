//! Builds the once-a-second hardware snapshot the screen displays.
//!
//! The JSON shape mirrors what KANALI sends on Windows (observed in the
//! screen's serial log): network, memory, cpu, gpu, disk, fans, motherboard.

use super::hwmon::HwmonTree;
use super::procfs::{CounterRates, CpuTimes, read, read_cpu_times, read_disk_bytes, read_memory, read_network_bytes};
use crate::link::unix_millis;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::Instant;

const KILOBYTE: f64 = 1024.0;
const MEGABYTE: f64 = 1024.0 * 1024.0;
const RAPL_ENERGY_FILE: &str = "sys/class/powercap/intel-rapl:0/energy_uj";

pub struct SystemMonitor {
    root: PathBuf,
    hwmon: HwmonTree,
    rates: CounterRates,
    previous_cpu_times: CpuTimes,
    gpu_reader: Box<dyn Fn(&Path) -> Value + Send>,
}

impl SystemMonitor {
    #[cfg(not(windows))]
    pub fn new() -> Self {
        Self::with_gpu_reader(Path::new("/"), Box::new(super::gpu::read_gpu))
    }

    pub fn with_gpu_reader(root: &Path, gpu_reader: Box<dyn Fn(&Path) -> Value + Send>) -> Self {
        Self {
            root: root.to_path_buf(),
            hwmon: HwmonTree::new(root),
            rates: CounterRates::default(),
            previous_cpu_times: read_cpu_times(root),
            gpu_reader,
        }
    }

    pub fn snapshot(&mut self) -> Value {
        let now = Instant::now();
        json!({
            "network": self.network(now),
            "memory": self.memory(),
            "cpu": self.cpu(now),
            "gpu": (self.gpu_reader)(&self.root),
            "disk": self.disk(now),
            "fans": self.fans(),
            "motherboard": { "temperature": self.hwmon.motherboard_temperature(), "pchTemperature": 0 },
            "timestamp": unix_millis(),
        })
    }

    fn network(&mut self, now: Instant) -> Value {
        let (received, sent) = read_network_bytes(&self.root);
        let rates = self.rates.update("network", &[received, sent], now);
        json!({ "upload": (rates[1] / KILOBYTE).round() as u64, "download": (rates[0] / KILOBYTE).round() as u64 })
    }

    fn memory(&self) -> Value {
        let memory = read_memory(&self.root);
        json!({ "total": memory.total_megabytes, "used": memory.used_megabytes, "load": memory.load_percent, "temperature": 0, "speed": 0 })
    }

    fn cpu(&mut self, now: Instant) -> Value {
        let load = self.cpu_load();
        json!({
            "load": load,
            "temperature": self.hwmon.cpu_temperature(),
            "speedAverage": self.average_cpu_megahertz(),
            "power": self.cpu_watts(now),
            "voltage": 0,
            "usage": load,
        })
    }

    fn cpu_load(&mut self) -> u64 {
        let current = read_cpu_times(&self.root);
        let load = current.load_since(&self.previous_cpu_times);
        self.previous_cpu_times = current;
        load
    }

    fn average_cpu_megahertz(&self) -> u64 {
        let cpu_directory = self.root.join("sys/devices/system/cpu");
        let kilohertz: Vec<u64> = std::fs::read_dir(cpu_directory)
            .map(|entries| entries.flatten().filter_map(|entry| read(&entry.path(), "cpufreq/scaling_cur_freq").trim().parse().ok()).collect())
            .unwrap_or_default();
        if kilohertz.is_empty() {
            return 0;
        }
        (kilohertz.iter().sum::<u64>() as f64 / kilohertz.len() as f64 / 1000.0).round() as u64
    }

    fn cpu_watts(&mut self, now: Instant) -> u64 {
        let Ok(microjoules) = read(&self.root, RAPL_ENERGY_FILE).trim().parse::<u64>() else {
            return 0;
        };
        (self.rates.update("cpu-energy", &[microjoules], now)[0] / 1_000_000.0).round() as u64
    }

    fn disk(&mut self, now: Instant) -> Value {
        let (total, used) = root_filesystem_gigabytes();
        let (read_bytes, written_bytes) = read_disk_bytes(&self.root);
        let rates = self.rates.update("disk", &[read_bytes, written_bytes], now);
        json!({
            "total": total,
            "used": used,
            "load": (100 * used + total / 2).checked_div(total).unwrap_or(0),
            "activity": 0,
            "temperature": self.hwmon.disk_temperature(),
            "readSpeed": (rates[0] / MEGABYTE).round() as u64,
            "writeSpeed": (rates[1] / MEGABYTE).round() as u64,
        })
    }

    fn fans(&self) -> Value {
        let fans: Vec<Value> = self.hwmon.spinning_fans().into_iter().map(|fan| json!({ "onBoard": true, "name": fan.label, "value": fan.value })).collect();
        Value::Array(fans)
    }
}

/// (total, used) GiB of the root filesystem, from `df` to avoid unsafe statvfs calls.
fn root_filesystem_gigabytes() -> (u64, u64) {
    let Ok(output) = std::process::Command::new("df").args(["-k", "--output=size,used", "/"]).output() else {
        return (0, 0);
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let numbers: Vec<u64> = text.lines().nth(1).unwrap_or_default().split_whitespace().filter_map(|value| value.parse().ok()).collect();
    match numbers.as_slice() {
        [size_kilobytes, used_kilobytes] => (kilobytes_to_gigabytes(*size_kilobytes), kilobytes_to_gigabytes(*used_kilobytes)),
        _ => (0, 0),
    }
}

fn kilobytes_to_gigabytes(kilobytes: u64) -> u64 {
    (kilobytes as f64 / (1024.0 * 1024.0)).round() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensors::gpu::empty_gpu;
    use crate::sensors::tests::FakeSystem;

    #[test]
    fn snapshot_has_the_shape_kanali_sends() {
        let system = FakeSystem::new("monitor");
        let mut monitor = SystemMonitor::with_gpu_reader(&system.root, Box::new(|_| empty_gpu()));
        let snapshot = monitor.snapshot();
        let mut keys: Vec<&String> = snapshot.as_object().unwrap().keys().collect();
        keys.sort();
        assert_eq!(keys, ["cpu", "disk", "fans", "gpu", "memory", "motherboard", "network", "timestamp"]);
        assert_eq!(snapshot["cpu"]["temperature"], 47);
        assert_eq!(snapshot["cpu"]["speedAverage"], 4500);
        assert_eq!(snapshot["memory"]["total"], 31801);
        assert_eq!(snapshot["fans"][0]["name"], "CPU Fan");
    }
}
