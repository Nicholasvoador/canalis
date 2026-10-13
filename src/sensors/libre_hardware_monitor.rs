//! Temperatures, fan speeds and CPU power on Windows. Windows only exposes these to kernel
//! drivers, so they are read from LibreHardwareMonitor's built-in web server
//! (Options → Remote Web Server → Run, port 8085) when it is running.

use serde_json::Value;

const CPU_TEMPERATURE: SensorQuery = SensorQuery { hardware: is_cpu, kind: "Temperature", preferred: &["Core (Tctl/Tdie)", "CPU Package", "Core (Tctl)", "Core Average"] };
const CPU_POWER: SensorQuery = SensorQuery { hardware: is_cpu, kind: "Power", preferred: &["Package", "CPU Package"] };
const MOTHERBOARD_TEMPERATURE: SensorQuery = SensorQuery { hardware: is_motherboard, kind: "Temperature", preferred: &["Motherboard", "System"] };
const DRIVE_TEMPERATURE: SensorQuery = SensorQuery { hardware: is_drive, kind: "Temperature", preferred: &["Composite Temperature", "Temperature"] };

#[derive(Debug, Default, PartialEq)]
#[cfg_attr(not(windows), allow(dead_code))]
pub struct HardwareReadings {
    pub cpu_temperature: u64,
    pub cpu_power: u64,
    pub motherboard_temperature: u64,
    pub disk_temperature: u64,
    /// (name, RPM) of each motherboard fan that is spinning.
    pub fans: Vec<(String, u64)>,
}

/// Current readings, or zeros when LibreHardwareMonitor is not reachable.
#[cfg(windows)]
pub fn read() -> HardwareReadings {
    web_server::fetch_sensor_tree().map(|tree| parse(&tree)).unwrap_or_default()
}

/// Picks KANALI's readings out of LibreHardwareMonitor's `data.json` sensor tree.
pub fn parse(tree: &Value) -> HardwareReadings {
    let mut sensors = Vec::new();
    collect_sensors(tree, &mut sensors);
    HardwareReadings {
        cpu_temperature: CPU_TEMPERATURE.find(&sensors),
        cpu_power: CPU_POWER.find(&sensors),
        motherboard_temperature: MOTHERBOARD_TEMPERATURE.find(&sensors),
        disk_temperature: DRIVE_TEMPERATURE.find(&sensors),
        fans: sensors
            .iter()
            .filter(|sensor| is_motherboard(sensor.id) && sensor.kind == "Fan" && sensor.value > 0.0)
            .map(|sensor| (sensor.name.to_string(), sensor.value.round() as u64))
            .collect(),
    }
}

struct Sensor<'a> {
    id: &'a str,
    kind: &'a str,
    name: &'a str,
    value: f64,
}

fn collect_sensors<'a>(node: &'a Value, sensors: &mut Vec<Sensor<'a>>) {
    if let (Some(id), Some(kind)) = (node["SensorId"].as_str(), node["Type"].as_str()) {
        let name = node["Text"].as_str().unwrap_or_default();
        sensors.push(Sensor { id, kind, name, value: leading_number(node["Value"].as_str().unwrap_or_default()) });
    }
    for child in node["Children"].as_array().into_iter().flatten() {
        collect_sensors(child, sensors);
    }
}

/// "45.4 °C" or, in some locales, "45,4 °C" → 45.4.
fn leading_number(text: &str) -> f64 {
    let number: String = text.trim().chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',' || *c == '-').collect();
    number.replace(',', ".").parse().unwrap_or(0.0)
}

struct SensorQuery {
    hardware: fn(&str) -> bool,
    kind: &'static str,
    preferred: &'static [&'static str],
}

impl SensorQuery {
    /// The first preferred name that exists, else the first sensor of this kind; rounded.
    fn find(&self, sensors: &[Sensor]) -> u64 {
        let candidates: Vec<&Sensor> = sensors.iter().filter(|sensor| (self.hardware)(sensor.id) && sensor.kind == self.kind).collect();
        let preferred = self.preferred.iter().find_map(|name| candidates.iter().find(|sensor| sensor.name == *name));
        preferred.or(candidates.first()).map_or(0, |sensor| sensor.value.max(0.0).round() as u64)
    }
}

fn is_cpu(sensor_id: &str) -> bool {
    sensor_id.starts_with("/amdcpu/") || sensor_id.starts_with("/intelcpu/")
}

fn is_motherboard(sensor_id: &str) -> bool {
    sensor_id.starts_with("/lpc/")
}

fn is_drive(sensor_id: &str) -> bool {
    ["/nvme/", "/hdd/", "/ssd/", "/storage/"].iter().any(|prefix| sensor_id.starts_with(prefix))
}

#[cfg(windows)]
mod web_server {
    use serde_json::Value;
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;

    const ADDRESS: &str = "127.0.0.1:8085";
    const TIMEOUT: Duration = Duration::from_millis(400);

    pub fn fetch_sensor_tree() -> Option<Value> {
        let address: SocketAddr = ADDRESS.parse().ok()?;
        let mut stream = TcpStream::connect_timeout(&address, TIMEOUT).ok()?;
        stream.set_read_timeout(Some(TIMEOUT)).ok()?;
        stream.write_all(b"GET /data.json HTTP/1.0\r\nHost: localhost\r\n\r\n").ok()?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response).ok()?;
        let response = String::from_utf8_lossy(&response);
        serde_json::from_str(response.split_once("\r\n\r\n")?.1).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sensor(id: &str, kind: &str, text: &str, value: &str) -> Value {
        json!({ "SensorId": id, "Type": kind, "Text": text, "Value": value, "Children": [] })
    }

    fn sample_tree() -> Value {
        json!({ "Text": "Sensor", "Children": [{ "Text": "DESKTOP", "Children": [
            { "Text": "AMD Ryzen 7 9800X3D", "Children": [
                { "Text": "Temperatures", "Children": [
                    sensor("/amdcpu/0/temperature/3", "Temperature", "CCD1 (Tdie)", "44.0 °C"),
                    sensor("/amdcpu/0/temperature/2", "Temperature", "Core (Tctl/Tdie)", "48,6 °C"),
                ]},
                { "Text": "Powers", "Children": [sensor("/amdcpu/0/power/0", "Power", "Package", "61.2 W")] },
            ]},
            { "Text": "MSI MPG X870E", "Children": [{ "Text": "Nuvoton NCT6687D", "Children": [
                sensor("/lpc/nct6687d/0/temperature/0", "Temperature", "CPU", "47.0 °C"),
                sensor("/lpc/nct6687d/0/temperature/1", "Temperature", "System", "38.0 °C"),
                sensor("/lpc/nct6687d/0/fan/0", "Fan", "CPU Fan", "1050 RPM"),
                sensor("/lpc/nct6687d/0/fan/1", "Fan", "System Fan #1", "0 RPM"),
            ]}]},
            { "Text": "Samsung SSD 990 PRO", "Children": [sensor("/nvme/0/temperature/0", "Temperature", "Composite Temperature", "41.0 °C")] },
        ]}]})
    }

    #[test]
    fn picks_kanalis_readings_from_the_sensor_tree() {
        let readings = parse(&sample_tree());
        assert_eq!(
            readings,
            HardwareReadings {
                cpu_temperature: 49,
                cpu_power: 61,
                motherboard_temperature: 38,
                disk_temperature: 41,
                fans: vec![("CPU Fan".into(), 1050)],
            }
        );
    }

    #[test]
    fn missing_sensors_read_as_zero() {
        assert_eq!(parse(&json!({ "Text": "Sensor", "Children": [] })), HardwareReadings::default());
    }

    #[test]
    fn values_parse_in_any_locale() {
        assert_eq!(leading_number("45.4 °C"), 45.4);
        assert_eq!(leading_number("45,4 °C"), 45.4);
        assert_eq!(leading_number("n/a"), 0.0);
    }
}
