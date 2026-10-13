//! Body of "POST config", the message that applies every setting at once.
//!
//! Mirrors what KANALI 1.2.1 sends (observed in its logs, for a custom theme and for a
//! built-in one with a filter, a badge and Date & Time). KANALI also sends
//! "turboPump": false, which the firmware rejects (it expects an object of
//! unknown shape), so it is left out; the rest of the config still applies.

use crate::settings::{CustomTheme, Fan, OverlayStyle, Settings};
use serde_json::{Value, json};

pub fn build_device_config(settings: &Settings, spec: &Value) -> Value {
    json!({
        "temperature": settings.temperature_unit,
        "waterBlockScreen": {
            "enable": settings.enable,
            "displayInSleep": settings.display_in_sleep,
            "brightness": settings.brightness,
            "rotate": settings.rotate,
            "id": theme(settings),
            "fanLCD": fan(&settings.fan),
        },
        "spec": spec,
    })
}

fn theme(settings: &Settings) -> Value {
    if settings.is_custom_theme() {
        custom_theme(&settings.custom)
    } else {
        json!({ "id": settings.theme, "settings": style(&settings.overlay_style, &settings.overlay), "sysinfoDisplay": settings.overlay })
    }
}

fn custom_theme(custom: &CustomTheme) -> Value {
    json!({
        "id": crate::settings::CUSTOMIZATION,
        "screenMode": "Full Screen",
        "playMode": custom.play_mode,
        "ratio": custom.ratio,
        "media": custom.media,
        "settings": style(&custom.style, &custom.overlay),
        "sysinfoDisplay": custom.overlay,
    })
}

fn style(style: &OverlayStyle, overlay: &[String]) -> Value {
    json!({
        "color": style.color,
        "align": style.align,
        "filter": { "value": style.filter.value, "opacity": style.filter.opacity },
        "badges": style.shown_badges(overlay),
    })
}

fn fan(fan: &Fan) -> Value {
    json!({ "mode": fan.mode, "smartMode": fan.smart_curve, "fixedMode": fan.fixed_percent })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{CPU_BADGE, DATE_AND_TIME, FIXED_MODE, GPU_BADGE, PRESETS, tests::KANALI_SETTINGS};

    // Body KANALI 1.2.1 sent with "POST config 1" on this machine, minus "turboPump".
    const KANALI_CONFIG: &str = r##"{"temperature":"Celsius","waterBlockScreen":{"enable":true,"displayInSleep":false,
        "brightness":100,"rotate":270,"id":{"id":"Customization","screenMode":"Full Screen","playMode":"Single",
        "ratio":"2:1","media":["2025-09-24_10-13-29-032.gif"],"settings":{"color":"#dcdcdc","align":"Left",
        "filter":{"value":null,"opacity":100},"badges":[]},"sysinfoDisplay":["CPU Temperature","GPU Temperature"]},
        "fanLCD":{"mode":"Smart Mode","smartMode":[[0,10],[10,20],[30,30],[50,40],[65,55],[80,70],[90,100],[100,100]],
        "fixedMode":40}},"spec":{"cpu":"AMD Ryzen 7 9800X3D","gpu":"NVIDIA GeForce RTX 5090"}}"##;

    fn spec() -> Value {
        json!({"cpu": "AMD Ryzen 7 9800X3D", "gpu": "NVIDIA GeForce RTX 5090"})
    }

    #[test]
    fn matches_kanali_payload() {
        let settings: Settings = serde_json::from_str(KANALI_SETTINGS).unwrap();
        let expected: Value = serde_json::from_str(KANALI_CONFIG).unwrap();
        assert_eq!(build_device_config(&settings, &spec()), expected);
    }

    /// The "id" KANALI 1.2.1 sent after choosing Pre-set 12, the Rain filter, the CPU badge and Date & Time.
    const KANALI_PRESET_ID: &str = r##"{"id":"Pre-set 12: Cyber Bunker","settings":{"color":"#dcdcdc","align":"Left",
        "filter":{"value":"Rain","opacity":100},"badges":["CPU Badge"]},"sysinfoDisplay":["CPU Temperature","GPU Temperature","Date&Time"]}"##;

    #[test]
    fn preset_theme_matches_kanali_payload() {
        let mut settings = Settings { theme: PRESETS[11].into(), ..Settings::default() };
        settings.overlay = vec!["CPU Temperature".into(), "GPU Temperature".into(), DATE_AND_TIME.into()];
        settings.overlay_style.filter.value = Some("Rain".into());
        settings.overlay_style.badges = vec![CPU_BADGE.into()];
        let expected: Value = serde_json::from_str(KANALI_PRESET_ID).unwrap();
        assert_eq!(build_device_config(&settings, &spec())["waterBlockScreen"]["id"], expected);
    }

    #[test]
    fn badges_without_a_matching_stat_are_not_sent() {
        let mut settings = Settings { theme: PRESETS[0].into(), ..Settings::default() };
        settings.overlay_style.badges = vec![GPU_BADGE.into()];
        settings.overlay = vec!["CPU Usage".into()];
        assert_eq!(build_device_config(&settings, &spec())["waterBlockScreen"]["id"]["settings"]["badges"], json!([]));
    }

    #[test]
    fn fixed_fan_mode() {
        let mut settings = Settings::default();
        settings.fan.mode = FIXED_MODE.into();
        settings.fan.fixed_percent = 60;
        let fan = &build_device_config(&settings, &spec())["waterBlockScreen"]["fanLCD"];
        assert_eq!((fan["mode"].as_str(), fan["fixedMode"].as_u64()), (Some(FIXED_MODE), Some(60)));
    }
}
