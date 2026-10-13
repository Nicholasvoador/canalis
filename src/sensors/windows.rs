//! Windows readings in the same snapshot shape as Linux. Load, clocks, memory, disk and network
//! come from the OS, the GPU from nvidia-smi, and temperatures, fans and CPU power from
//! LibreHardwareMonitor when it is running.

use super::gpu::{nvidia_gpu_name, read_gpu};
use super::libre_hardware_monitor::{self, HardwareReadings};
use crate::link::unix_millis;
use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;
use sysinfo::{CpuRefreshKind, Disk, Disks, MemoryRefreshKind, Networks, RefreshKind, System};
use winreg::RegKey;
use winreg::enums::HKEY_LOCAL_MACHINE;

const KILOBYTE: f64 = 1024.0;
const MEGABYTE: f64 = 1024.0 * 1024.0;
const GIGABYTE: f64 = 1024.0 * 1024.0 * 1024.0;
const DISPLAY_ADAPTERS_KEY: &str = r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}";

pub struct SystemMonitor {
    system: System,
    networks: Networks,
    disks: Disks,
    last_refresh: Instant,
}

impl SystemMonitor {
    pub fn new() -> Self {
        Self {
            system: System::new_with_specifics(cpu_and_memory()),
            networks: Networks::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
            last_refresh: Instant::now(),
        }
    }

    pub fn snapshot(&mut self) -> Value {
        let seconds = self.refresh();
        let hardware = libre_hardware_monitor::read();
        json!({
            "network": self.network(seconds),
            "memory": self.memory(),
            "cpu": self.cpu(&hardware),
            "gpu": read_gpu(Path::new("/")),
            "disk": self.disk(seconds, &hardware),
            "fans": fans(&hardware),
            "motherboard": { "temperature": hardware.motherboard_temperature, "pchTemperature": 0 },
            "timestamp": unix_millis(),
        })
    }

    /// Refreshes every counter and returns the seconds since the previous refresh.
    fn refresh(&mut self) -> f64 {
        self.system.refresh_specifics(cpu_and_memory());
        self.networks.refresh(true);
        self.disks.refresh(true);
        let seconds = self.last_refresh.elapsed().as_secs_f64().max(0.001);
        self.last_refresh = Instant::now();
        seconds
    }

    fn network(&self, seconds: f64) -> Value {
        let (received, sent) = self.networks.iter().fold((0, 0), |(received, sent), (_, data)| (received + data.received(), sent + data.transmitted()));
        json!({ "upload": per_second(sent, seconds, KILOBYTE), "download": per_second(received, seconds, KILOBYTE) })
    }

    fn memory(&self) -> Value {
        let total = self.system.total_memory() as f64 / MEGABYTE;
        let used = self.system.used_memory() as f64 / MEGABYTE;
        json!({ "total": total.round() as u64, "used": used.round() as u64, "load": percent(used, total), "temperature": 0, "speed": 0 })
    }

    fn cpu(&self, hardware: &HardwareReadings) -> Value {
        let load = self.system.global_cpu_usage().round() as u64;
        let cpus = self.system.cpus();
        let average_megahertz = cpus.iter().map(|cpu| cpu.frequency()).sum::<u64>() / cpus.len().max(1) as u64;
        json!({ "load": load, "temperature": hardware.cpu_temperature, "speedAverage": average_megahertz, "power": hardware.cpu_power, "voltage": 0, "usage": load })
    }

    fn disk(&self, seconds: f64, hardware: &HardwareReadings) -> Value {
        let (total, used, read_bytes, written_bytes) = match self.system_disk() {
            Some(disk) => {
                let total = disk.total_space() as f64 / GIGABYTE;
                (total, total - disk.available_space() as f64 / GIGABYTE, disk.usage().read_bytes, disk.usage().written_bytes)
            }
            None => (0.0, 0.0, 0, 0),
        };
        json!({
            "total": total.round() as u64,
            "used": used.round() as u64,
            "load": percent(used, total),
            "activity": 0,
            "temperature": hardware.disk_temperature,
            "readSpeed": per_second(read_bytes, seconds, MEGABYTE),
            "writeSpeed": per_second(written_bytes, seconds, MEGABYTE),
        })
    }

    /// The drive Windows is installed on, like KANALI's disk tile.
    fn system_disk(&self) -> Option<&Disk> {
        let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into()).to_uppercase();
        self.disks.iter().find(|disk| disk.mount_point().to_string_lossy().to_uppercase().starts_with(&drive))
    }
}

fn cpu_and_memory() -> RefreshKind {
    RefreshKind::nothing().with_cpu(CpuRefreshKind::nothing().with_cpu_usage().with_frequency()).with_memory(MemoryRefreshKind::nothing().with_ram())
}

fn fans(hardware: &HardwareReadings) -> Value {
    Value::Array(hardware.fans.iter().map(|(name, rpm)| json!({ "onBoard": true, "name": name, "value": rpm })).collect())
}

fn per_second(bytes: u64, seconds: f64, unit: f64) -> u64 {
    (bytes as f64 / seconds / unit).round() as u64
}

fn percent(part: f64, whole: f64) -> u64 {
    if whole > 0.0 { (100.0 * part / whole).round() as u64 } else { 0 }
}

pub fn cpu_model() -> String {
    let system = System::new_with_specifics(RefreshKind::nothing().with_cpu(CpuRefreshKind::nothing()));
    system.cpus().first().map(|cpu| cpu.brand().trim().to_string()).unwrap_or_default()
}

pub fn gpu_model() -> String {
    nvidia_gpu_name().or_else(registry_gpu_name).unwrap_or_default()
}

/// The first real display adapter Windows has a driver for.
fn registry_gpu_name() -> Option<String> {
    let adapters = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(DISPLAY_ADAPTERS_KEY).ok()?;
    adapters
        .enum_keys()
        .flatten()
        .filter_map(|name| adapters.open_subkey(name).ok()?.get_value::<String, _>("DriverDesc").ok())
        .find(|name| !name.starts_with("Microsoft"))
}
