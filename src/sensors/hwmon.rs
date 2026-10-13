//! Temperatures and fan speeds from /sys/class/hwmon.

use std::path::{Path, PathBuf};

const CPU_SENSORS: [(&str, &[&str]); 3] = [
    ("k10temp", &["Tctl", "Tdie"]),
    ("zenpower", &["Tdie", "Tctl"]),
    ("coretemp", &["Package id 0"]),
];
const MOTHERBOARD_CHIP_PREFIXES: [&str; 7] = ["nct", "it8", "f71", "w83", "asus", "gigabyte", "dell_smm"];
const MOTHERBOARD_LABELS: [&str; 4] = ["SYSTIN", "Motherboard", "MB Temp", "System"];

#[derive(Clone, Debug, PartialEq)]
pub struct Reading {
    pub chip: String,
    pub label: String,
    pub value: i64,
}

pub struct HwmonTree {
    hwmon: PathBuf,
}

impl HwmonTree {
    pub fn new(root: &Path) -> Self {
        Self { hwmon: root.join("sys/class/hwmon") }
    }

    pub fn cpu_temperature(&self) -> i64 {
        let readings = self.readings("temp");
        CPU_SENSORS
            .iter()
            .find_map(|(chip, labels)| first_labelled(&readings, chip, labels))
            .map(|reading| millidegrees_to_celsius(reading.value))
            .unwrap_or(0)
    }

    pub fn motherboard_temperature(&self) -> i64 {
        self.readings("temp")
            .iter()
            .find(|reading| MOTHERBOARD_CHIP_PREFIXES.iter().any(|prefix| reading.chip.starts_with(prefix)) && MOTHERBOARD_LABELS.contains(&reading.label.as_str()))
            .map(|reading| millidegrees_to_celsius(reading.value))
            .unwrap_or(0)
    }

    pub fn disk_temperature(&self) -> i64 {
        first_labelled(&self.readings("temp"), "nvme", &["Composite"]).map(|reading| millidegrees_to_celsius(reading.value)).unwrap_or(0)
    }

    pub fn spinning_fans(&self) -> Vec<Reading> {
        self.readings("fan").into_iter().filter(|reading| reading.value > 0).collect()
    }

    fn readings(&self, kind: &str) -> Vec<Reading> {
        let mut devices = list_directory(&self.hwmon);
        devices.retain(|path| file_name(path).starts_with("hwmon"));
        devices.iter().flat_map(|device| device_readings(device, kind)).collect()
    }
}

fn first_labelled<'a>(readings: &'a [Reading], chip: &str, labels: &[&str]) -> Option<&'a Reading> {
    let candidates: Vec<&Reading> = readings.iter().filter(|reading| reading.chip == chip).collect();
    labels
        .iter()
        .find_map(|label| candidates.iter().find(|reading| reading.label == *label).copied())
        .or_else(|| candidates.first().copied())
}

fn device_readings(device: &Path, kind: &str) -> Vec<Reading> {
    let chip = read_trimmed(&device.join("name"));
    let mut inputs = list_directory(device);
    inputs.retain(|path| {
        let name = file_name(path);
        name.starts_with(kind) && name.ends_with("_input")
    });
    inputs.iter().filter_map(|input| reading(&chip, input)).collect()
}

fn reading(chip: &str, input: &Path) -> Option<Reading> {
    let value: i64 = read_trimmed(input).parse().ok()?;
    let sensor = file_name(input).trim_end_matches("_input").to_string();
    let label = Some(read_trimmed(&input.with_file_name(format!("{sensor}_label")))).filter(|label| !label.is_empty()).unwrap_or(sensor);
    Some(Reading { chip: chip.to_string(), label, value })
}

fn millidegrees_to_celsius(value: i64) -> i64 {
    (value as f64 / 1000.0).round() as i64
}

fn list_directory(directory: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(directory).map(|entries| entries.flatten().map(|entry| entry.path()).collect()).unwrap_or_default();
    paths.sort();
    paths
}

fn file_name(path: &Path) -> String {
    path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default()
}

fn read_trimmed(path: &Path) -> String {
    std::fs::read_to_string(path).map(|text| text.trim().to_string()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensors::tests::FakeSystem;

    #[test]
    fn cpu_temperature_from_k10temp() {
        let system = FakeSystem::new("hwmon-cpu");
        assert_eq!(HwmonTree::new(&system.root).cpu_temperature(), 47);
    }

    #[test]
    fn motherboard_temperature_from_superio_chip() {
        let system = FakeSystem::new("hwmon-board");
        assert_eq!(HwmonTree::new(&system.root).motherboard_temperature(), 40);
    }

    #[test]
    fn spinning_fans_with_labels() {
        let system = FakeSystem::new("hwmon-fans");
        let fans = HwmonTree::new(&system.root).spinning_fans();
        assert_eq!(fans, vec![Reading { chip: "nct6799".into(), label: "CPU Fan".into(), value: 1050 }]);
    }
}
