//! User settings (~/.config/panorama/settings.json), edited by the UI and the
//! command line and applied to the screen by the background service.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub const PRESETS: [&str; 7] = [
    "Pre-set 1: Cooling delivery",
    "Pre-set 2: Migration",
    "Pre-set 3: Quantum time capsule",
    "Pre-set 4: Exo-Ecologies",
    "Pre-set 5: Racing",
    "Pre-set 6: Shuttle",
    "Pre-set 7: Gift of TRYX",
];
pub const CUSTOMIZATION: &str = "Customization";
pub const ROTATIONS: [u16; 4] = [0, 90, 180, 270];
pub const SMART_MODE: &str = "Smart Mode";
pub const FIXED_MODE: &str = "Fixed Mode";
pub const CELSIUS: &str = "Celsius";
pub const FAHRENHEIT: &str = "Fahrenheit";
pub const ALIGNMENTS: [&str; 3] = ["Left", "Center", "Right"];
pub const MAX_OVERLAY_FIELDS: usize = 3;
pub const OVERLAY_FIELDS: [&str; 13] = [
    "CPU Temperature",
    "CPU Usage",
    "CPU Frequency",
    "CPU Voltage",
    "CPU Power",
    "GPU Temperature",
    "GPU Usage",
    "GPU Frequency",
    "GPU Voltage",
    "GPU Power",
    "Memory Utilization",
    "Memory Frequency",
    "Motherboard Temperature",
];

pub type FanCurve = Vec<[u8; 2]>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub temperature_unit: String,
    pub enable: bool,
    pub display_in_sleep: bool,
    pub brightness: u8,
    pub rotate: u16,
    pub theme: String,
    pub overlay: Vec<String>,
    pub custom: CustomTheme,
    pub fan: Fan,
    pub update_interval_seconds: f64,
    #[serde(flatten)]
    pub unknown_fields: Map<String, Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct CustomTheme {
    pub media: Vec<String>,
    pub ratio: String,
    pub play_mode: String,
    pub color: String,
    pub align: String,
    pub overlay: Vec<String>,
    #[serde(flatten)]
    pub unknown_fields: Map<String, Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Fan {
    pub mode: String,
    pub smart_curve: FanCurve,
    pub fixed_percent: u8,
    #[serde(flatten)]
    pub unknown_fields: Map<String, Value>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            temperature_unit: CELSIUS.into(),
            enable: true,
            display_in_sleep: false,
            brightness: 100,
            rotate: 0,
            theme: PRESETS[0].into(),
            overlay: Vec::new(),
            custom: CustomTheme::default(),
            fan: Fan::default(),
            update_interval_seconds: 1.0,
            unknown_fields: Map::new(),
        }
    }
}

impl Default for CustomTheme {
    fn default() -> Self {
        Self {
            media: Vec::new(),
            ratio: "2:1".into(),
            play_mode: "Single".into(),
            color: "#000000".into(),
            align: "Left".into(),
            overlay: Vec::new(),
            unknown_fields: Map::new(),
        }
    }
}

impl Default for Fan {
    fn default() -> Self {
        Self {
            mode: SMART_MODE.into(),
            smart_curve: FanCurvePreset::Quiet.curve(),
            fixed_percent: 40,
            unknown_fields: Map::new(),
        }
    }
}

impl Settings {
    pub fn is_custom_theme(&self) -> bool {
        self.theme == CUSTOMIZATION
    }

    pub fn active_overlay(&self) -> &Vec<String> {
        if self.is_custom_theme() { &self.custom.overlay } else { &self.overlay }
    }

    pub fn active_overlay_mut(&mut self) -> &mut Vec<String> {
        if self.is_custom_theme() { &mut self.custom.overlay } else { &mut self.overlay }
    }

    pub fn add_custom_media(&mut self, names: Vec<String>, append: bool) {
        if !append {
            self.custom.media.clear();
        }
        self.custom.media.retain(|existing| !names.contains(existing));
        self.custom.media.extend(names);
        self.custom.play_mode = if self.custom.media.len() > 1 { "Loop" } else { "Single" }.into();
        self.theme = CUSTOMIZATION.into();
    }

    pub fn validate(&self) -> Result<(), String> {
        require(self.brightness <= 100, "brightness must be between 0 and 100")?;
        require(ROTATIONS.contains(&self.rotate), "rotation must be 0, 90, 180 or 270")?;
        require(self.theme == CUSTOMIZATION || PRESETS.contains(&self.theme.as_str()), "unknown theme")?;
        require([CELSIUS, FAHRENHEIT].contains(&self.temperature_unit.as_str()), "unknown temperature unit")?;
        require([SMART_MODE, FIXED_MODE].contains(&self.fan.mode.as_str()), "unknown fan mode")?;
        require(self.fan.fixed_percent <= 100, "fixed fan speed must be between 0 and 100")?;
        require(ALIGNMENTS.contains(&self.custom.align.as_str()), "align must be Left, Center or Right")?;
        require(overlay_is_valid(&self.overlay) && overlay_is_valid(&self.custom.overlay), "overlay: up to 3 known fields")?;
        require(self.update_interval_seconds >= 0.2, "update interval must be at least 0.2 s")
    }
}

fn overlay_is_valid(overlay: &[String]) -> bool {
    overlay.len() <= MAX_OVERLAY_FIELDS && overlay.iter().all(|field| OVERLAY_FIELDS.contains(&field.as_str()))
}

fn require(condition: bool, message: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(message.to_string()) }
}

/// Fan curves KANALI ships as Low / Mid / High / Full speed: [temperature °C, fan %].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FanCurvePreset {
    Quiet,
    Balanced,
    Performance,
    Full,
}

impl FanCurvePreset {
    pub const ALL: [FanCurvePreset; 4] = [Self::Quiet, Self::Balanced, Self::Performance, Self::Full];

    pub fn label(self) -> &'static str {
        match self {
            Self::Quiet => "Quiet",
            Self::Balanced => "Balanced",
            Self::Performance => "Performance",
            Self::Full => "Full speed",
        }
    }

    pub fn curve(self) -> FanCurve {
        let points: [[u8; 2]; 8] = match self {
            Self::Quiet => [[0, 10], [10, 20], [30, 30], [50, 40], [65, 55], [80, 70], [90, 100], [100, 100]],
            Self::Balanced => [[0, 10], [10, 20], [30, 35], [50, 50], [65, 75], [80, 80], [90, 100], [100, 100]],
            Self::Performance => [[0, 10], [10, 20], [30, 50], [40, 70], [55, 85], [70, 90], [90, 100], [100, 100]],
            Self::Full => [[0, 10], [10, 20], [30, 70], [40, 100], [65, 100], [80, 100], [90, 100], [100, 100]],
        };
        points.to_vec()
    }
}

pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<Settings, String> {
        match fs::read_to_string(&self.path) {
            Ok(text) => serde_json::from_str(&text).map_err(|error| format!("{}: {error}", self.path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
            Err(error) => Err(format!("{}: {error}", self.path.display())),
        }
    }

    pub fn save(&self, settings: &Settings) -> Result<(), String> {
        settings.validate()?;
        let text = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())? + "\n";
        write_atomically(&self.path, &text).map_err(|error| format!("{}: {error}", self.path.display()))
    }

    pub fn modified_time(&self) -> Option<SystemTime> {
        fs::metadata(&self.path).and_then(|metadata| metadata.modified()).ok()
    }
}

pub fn write_atomically(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, text)?;
    fs::rename(&temporary, path)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub const KANALI_SETTINGS: &str = include_str!("../packaging/settings-from-kanali.json");

    pub fn temporary_path(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!("panorama-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        directory.join("settings.json")
    }

    #[test]
    fn reads_the_settings_copied_from_kanali() {
        let settings: Settings = serde_json::from_str(KANALI_SETTINGS).unwrap();
        assert_eq!(settings.rotate, 270);
        assert!(settings.is_custom_theme());
        assert_eq!(settings.custom.overlay, ["CPU Temperature", "GPU Temperature"]);
        assert_eq!(settings.fan.smart_curve, FanCurvePreset::Quiet.curve());
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn missing_file_gives_defaults() {
        let store = SettingsStore::new(temporary_path("missing"));
        assert_eq!(store.load().unwrap(), Settings::default());
    }

    #[test]
    fn round_trip_keeps_unknown_fields() {
        let store = SettingsStore::new(temporary_path("round-trip"));
        let mut settings: Settings = serde_json::from_str(r#"{"brightness": 20, "future_option": 7}"#).unwrap();
        settings.brightness = 55;
        store.save(&settings).unwrap();
        let reloaded = store.load().unwrap();
        assert_eq!(reloaded.brightness, 55);
        assert_eq!(reloaded.unknown_fields["future_option"], 7);
    }

    #[test]
    fn rejects_invalid_rotation() {
        let settings = Settings { rotate: 45, ..Settings::default() };
        assert!(SettingsStore::new(temporary_path("invalid")).save(&settings).is_err());
    }

    #[test]
    fn overlay_target_follows_theme() {
        let mut settings = Settings { theme: CUSTOMIZATION.into(), ..Settings::default() };
        settings.active_overlay_mut().push("CPU Usage".into());
        assert_eq!(settings.custom.overlay, ["CPU Usage"]);
        assert!(settings.overlay.is_empty());
    }

    #[test]
    fn appending_media_switches_to_loop() {
        let mut settings = Settings::default();
        settings.add_custom_media(vec!["a.gif".into()], false);
        settings.add_custom_media(vec!["b.mp4".into()], true);
        assert_eq!(settings.custom.media, ["a.gif", "b.mp4"]);
        assert_eq!(settings.custom.play_mode, "Loop");
        assert!(settings.is_custom_theme());
    }
}
