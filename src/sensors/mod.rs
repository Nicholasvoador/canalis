//! Hardware readings for the screen's live stats and specifications page.
//! Linux reads /proc and /sys; Windows asks the OS and LibreHardwareMonitor.

pub mod gpu;
#[cfg(any(not(windows), test))]
pub mod hwmon;
#[cfg(any(windows, test))]
mod libre_hardware_monitor;
#[cfg(any(not(windows), test))]
pub mod monitor;
#[cfg(any(not(windows), test))]
pub mod procfs;
#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
pub use monitor::SystemMonitor;
#[cfg(windows)]
pub use windows::SystemMonitor;

use serde_json::{Value, json};

/// CPU and GPU model names shown on the screen's specifications page.
pub fn read_spec() -> Value {
    json!({ "cpu": platform::cpu_model(), "gpu": platform::gpu_model() })
}

#[cfg(windows)]
use windows as platform;

#[cfg(not(windows))]
mod platform {
    use super::gpu;
    use crate::hidden_command::hidden_command;
    use std::path::Path;

    pub fn cpu_model() -> String {
        super::cpu_model_from_cpuinfo(Path::new("/"))
    }

    pub fn gpu_model() -> String {
        gpu::nvidia_gpu_name().or_else(lspci_gpu_name).unwrap_or_default()
    }

    fn lspci_gpu_name() -> Option<String> {
        let output = hidden_command("lspci").output().ok()?;
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .find(|line| line.contains("VGA compatible controller") || line.contains("3D controller"))
            .and_then(|line| line.split_once(": "))
            .map(|(_, name)| name.trim().to_string())
    }
}

#[cfg(any(not(windows), test))]
fn cpu_model_from_cpuinfo(root: &std::path::Path) -> String {
    procfs::read(root, "proc/cpuinfo")
        .lines()
        .find_map(|line| line.split_once(':').filter(|(name, _)| name.trim() == "model name").map(|(_, value)| value.trim().to_string()))
        .unwrap_or_default()
}

#[cfg(test)]
pub(crate) mod tests {
    use std::path::PathBuf;

    /// A throwaway directory laid out like / with /proc and /sys files.
    pub struct FakeSystem {
        pub root: PathBuf,
    }

    impl FakeSystem {
        pub fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!("panorama-fake-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            let system = Self { root };
            system.populate();
            system
        }

        pub fn write(&self, relative: &str, content: &str) {
            let path = self.root.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }

        fn populate(&self) {
            self.write("proc/stat", "cpu  100 0 100 800 0 0 0 0 0 0\ncpu0 1 0 1 8 0 0 0 0 0 0\n");
            self.write("proc/meminfo", "MemTotal:       32564224 kB\nMemFree:  1000 kB\nMemAvailable:   18587648 kB\n");
            self.write("proc/cpuinfo", "processor\t: 0\nmodel name\t: AMD Ryzen 7 9800X3D 8-Core Processor\n");
            self.write(
                "proc/net/dev",
                "Inter-|   Receive |  Transmit\n face |bytes packets|bytes\n    lo: 5000 0 0 0 0 0 0 0 5000 0 0 0 0 0 0 0\n  eth0: 204800 0 0 0 0 0 0 0 102400 0 0 0 0 0 0 0\n",
            );
            self.write("proc/diskstats", "");
            self.write("sys/class/hwmon/hwmon0/name", "k10temp\n");
            self.write("sys/class/hwmon/hwmon0/temp1_label", "Tctl\n");
            self.write("sys/class/hwmon/hwmon0/temp1_input", "46500\n");
            self.write("sys/class/hwmon/hwmon1/name", "nct6799\n");
            self.write("sys/class/hwmon/hwmon1/temp1_label", "SYSTIN\n");
            self.write("sys/class/hwmon/hwmon1/temp1_input", "40000\n");
            self.write("sys/class/hwmon/hwmon1/fan2_label", "CPU Fan\n");
            self.write("sys/class/hwmon/hwmon1/fan2_input", "1050\n");
            self.write("sys/class/hwmon/hwmon1/fan3_input", "0\n");
            self.write("sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq", "4000000\n");
            self.write("sys/devices/system/cpu/cpu1/cpufreq/scaling_cur_freq", "5000000\n");
        }
    }

    #[test]
    fn cpu_model_from_cpuinfo() {
        let system = FakeSystem::new("spec");
        assert_eq!(super::cpu_model_from_cpuinfo(&system.root), "AMD Ryzen 7 9800X3D 8-Core Processor");
    }
}
