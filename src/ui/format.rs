//! Turns status numbers into display text.

use crate::settings::FAHRENHEIT;
use crate::status::LiveStatus;

pub fn temperature(celsius: f64, unit: &str) -> String {
    if unit == FAHRENHEIT { format!("{:.0}°F", celsius * 9.0 / 5.0 + 32.0) } else { format!("{celsius:.0}°C") }
}

/// Number and unit separately, so the unit can be drawn smaller.
pub fn rate(kilobytes_per_second: f64) -> (String, &'static str) {
    if kilobytes_per_second >= 1024.0 {
        (format!("{:.1}", kilobytes_per_second / 1024.0), "MB/s")
    } else {
        (format!("{kilobytes_per_second:.0}"), "KB/s")
    }
}

pub fn percent(value: f64) -> String {
    format!("{value:.0}%")
}

/// KANALI's "15.45G/31G".
pub fn memory_usage(used_megabytes: f64, total_megabytes: f64) -> String {
    format!("{:.2}G/{:.0}G", used_megabytes / 1024.0, total_megabytes / 1024.0)
}

/// KANALI's "1500G/1861G".
pub fn disk_usage(used_gigabytes: f64, total_gigabytes: f64) -> String {
    format!("{used_gigabytes:.0}G/{total_gigabytes:.0}G")
}

pub fn megabytes_as_gigabytes(megabytes: f64) -> String {
    format!("{:.1} GB", megabytes / 1024.0)
}

pub fn bytes_as_gigabytes(bytes: f64) -> String {
    format!("{:.1} GB", bytes / 1024.0_f64.powi(3))
}

/// Value shown on the screen for one overlay field (the names KANALI uses).
pub fn overlay_value(field: &str, status: &LiveStatus, unit: &str) -> String {
    let number = |pointer: &str| status.number(pointer);
    match field {
        "CPU Temperature" => temperature(number("/snapshot/cpu/temperature"), unit),
        "CPU Usage" => percent(number("/snapshot/cpu/load")),
        "CPU Frequency" => format!("{:.0} MHz", number("/snapshot/cpu/speedAverage")),
        "CPU Voltage" => format!("{:.3} V", number("/snapshot/cpu/voltage")),
        "CPU Power" => format!("{:.0} W", number("/snapshot/cpu/power")),
        "GPU Temperature" => temperature(number("/snapshot/gpu/temperature"), unit),
        "GPU Usage" => percent(number("/snapshot/gpu/load")),
        "GPU Frequency" => format!("{:.0} MHz", number("/snapshot/gpu/speed")),
        "GPU Voltage" => format!("{:.3} V", number("/snapshot/gpu/voltage")),
        "GPU Power" => format!("{:.0} W", number("/snapshot/gpu/power")),
        "Memory Utilization" => percent(number("/snapshot/memory/load")),
        "Memory Frequency" => format!("{:.0} MHz", number("/snapshot/memory/speed")),
        "Motherboard Temperature" => temperature(number("/snapshot/motherboard/temperature"), unit),
        _ => "—".into(),
    }
}

/// "46°C" -> ("46", "°C"), so the number can be drawn large and the unit small.
pub fn split_reading(reading: &str) -> (String, String) {
    let unit_start = reading.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '—')).unwrap_or(reading.len());
    (reading[..unit_start].to_string(), reading[unit_start..].trim().to_string())
}

/// The short caption the screen prints under a stat, e.g. "CPU TEMP".
pub fn overlay_label(field: &str) -> String {
    field.replace("Temperature", "Temp").to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn temperatures_in_both_units() {
        assert_eq!(temperature(46.0, "Celsius"), "46°C");
        assert_eq!(temperature(100.0, FAHRENHEIT), "212°F");
    }

    #[test]
    fn rates_switch_to_megabytes() {
        assert_eq!(rate(512.0), ("512".into(), "KB/s"));
        assert_eq!(rate(2048.0), ("2.0".into(), "MB/s"));
    }

    #[test]
    fn usage_matches_kanali() {
        assert_eq!(memory_usage(15820.8, 31744.0), "15.45G/31G");
        assert_eq!(disk_usage(1500.2, 1861.0), "1500G/1861G");
    }

    #[test]
    fn overlay_values_come_from_the_snapshot() {
        let status = LiveStatus::from_value(json!({"snapshot": {"cpu": {"temperature": 46, "load": 12}, "gpu": {"power": 47.6}}}));
        assert_eq!(overlay_value("CPU Temperature", &status, "Celsius"), "46°C");
        assert_eq!(overlay_value("CPU Usage", &status, "Celsius"), "12%");
        assert_eq!(overlay_value("GPU Power", &status, "Celsius"), "48 W");
    }

    #[test]
    fn readings_split_into_number_and_unit() {
        assert_eq!(split_reading("46°C"), ("46".into(), "°C".into()));
        assert_eq!(split_reading("3500 MHz"), ("3500".into(), "MHz".into()));
        assert_eq!(split_reading("1.250 V"), ("1.250".into(), "V".into()));
        assert_eq!(split_reading("—"), ("—".into(), String::new()));
    }

    #[test]
    fn labels_match_the_screen() {
        assert_eq!(overlay_label("GPU Temperature"), "GPU TEMP");
        assert_eq!(overlay_label("CPU Usage"), "CPU USAGE");
    }
}
