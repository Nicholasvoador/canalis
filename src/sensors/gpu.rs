//! GPU readings from nvidia-smi or the amdgpu hwmon driver.

use crate::hidden_command::hidden_command;
use serde_json::{Value, json};
use std::path::Path;

const NVIDIA_QUERY: &str = "utilization.gpu,temperature.gpu,fan.speed,clocks.gr,power.draw,voltage.graphics";
const NVIDIA_FALLBACK_QUERY: &str = "utilization.gpu,temperature.gpu,fan.speed,clocks.gr,power.draw";
const GPU_FIELDS: [&str; 6] = ["load", "temperature", "fan", "speed", "power", "voltage"];

pub fn read_gpu(root: &Path) -> Value {
    read_nvidia().or_else(|| read_amdgpu(root)).unwrap_or_else(empty_gpu)
}

pub fn empty_gpu() -> Value {
    json!({"load": 0, "temperature": 0, "fan": 0, "speed": 0, "power": 0, "voltage": 0})
}

pub fn parse_nvidia_smi(output: &str) -> Value {
    let values: Vec<f64> = output.lines().next().unwrap_or_default().split(',').map(|value| value.trim().parse().unwrap_or(0.0)).collect();
    let mut gpu = serde_json::Map::new();
    for (index, name) in GPU_FIELDS.iter().enumerate() {
        let number = values.get(index).copied().unwrap_or(0.0);
        let value = if *name == "voltage" { json!((number * 1000.0).round() / 1000.0) } else { json!(number.round() as i64) };
        gpu.insert(name.to_string(), value);
    }
    Value::Object(gpu)
}

fn read_nvidia() -> Option<Value> {
    [NVIDIA_QUERY, NVIDIA_FALLBACK_QUERY].iter().find_map(|query| run_nvidia_smi(query)).map(|output| parse_nvidia_smi(&output))
}

fn run_nvidia_smi(query: &str) -> Option<String> {
    let output = hidden_command("nvidia-smi").arg(format!("--query-gpu={query}")).arg("--format=csv,noheader,nounits").output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (output.status.success() && !text.is_empty()).then_some(text)
}

pub fn nvidia_gpu_name() -> Option<String> {
    let output = hidden_command("nvidia-smi").args(["--query-gpu=name", "--format=csv,noheader"]).output().ok()?;
    let name = String::from_utf8_lossy(&output.stdout).lines().next()?.trim().to_string();
    (output.status.success() && !name.is_empty()).then_some(name)
}

fn read_amdgpu(root: &Path) -> Option<Value> {
    let devices = std::fs::read_dir(root.join("sys/class/hwmon")).ok()?;
    let hwmon = devices.flatten().map(|entry| entry.path()).find(|path| read_trimmed(&path.join("name")) == "amdgpu")?;
    Some(amdgpu_readings(&hwmon))
}

fn amdgpu_readings(hwmon: &Path) -> Value {
    let number = |relative: &str| read_trimmed(&hwmon.join(relative)).parse::<f64>().unwrap_or(0.0);
    json!({
        "load": number("device/gpu_busy_percent").round() as i64,
        "temperature": (number("temp1_input") / 1000.0).round() as i64,
        "fan": (100.0 * number("pwm1") / 255.0).round() as i64,
        "speed": (number("freq1_input") / 1_000_000.0).round() as i64,
        "power": (number("power1_average") / 1_000_000.0).round() as i64,
        "voltage": (number("in0_input")).round() / 1000.0,
    })
}

fn read_trimmed(path: &Path) -> String {
    std::fs::read_to_string(path).map(|text| text.trim().to_string()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nvidia_smi_csv() {
        let gpu = parse_nvidia_smi("12, 39, 30, 1995, 47.52, 0.805\n");
        assert_eq!(gpu, json!({"load": 12, "temperature": 39, "fan": 30, "speed": 1995, "power": 48, "voltage": 0.805}));
    }

    #[test]
    fn unsupported_fields_become_zero() {
        let gpu = parse_nvidia_smi("5, 40, [N/A], 300, [N/A]\n");
        assert_eq!((gpu["fan"].as_i64(), gpu["power"].as_i64(), gpu["voltage"].as_f64()), (Some(0), Some(0), Some(0.0)));
    }
}
