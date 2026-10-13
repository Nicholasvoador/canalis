//! Command line: `panorama` opens the window; subcommands change settings or run the service.

use crate::media::AdbScreen;
use crate::settings::{ALIGNMENTS, CELSIUS, CUSTOMIZATION, FAHRENHEIT, FIXED_MODE, PRESETS, SMART_MODE, Settings, SettingsStore};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "panorama", version, about = "Control the TRYX Panorama cooler screen (run without arguments for the window)")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Open the window (the default)
    Ui {
        /// Page to open on
        #[arg(long, value_enum, default_value = "home")]
        page: crate::ui::StartPage,
    },
    /// Run the background service (normally started by systemd)
    Service {
        /// Serial device, e.g. /dev/ttyACM0 (auto-detected by default)
        #[arg(long)]
        port: Option<String>,
    },
    /// Write the app icon as a PNG (used by install.sh)
    #[command(hide = true)]
    Icon { path: std::path::PathBuf },
    /// Print the current settings
    Show,
    /// List the built-in themes
    Presets,
    /// Screen brightness, 0-100
    Brightness { value: u8 },
    /// Screen rotation in degrees: 0, 90, 180 or 270
    Rotate { degrees: u16 },
    /// Turn the screen on or off
    Screen { state: Switch },
    /// on: black screen while the PC sleeps; off: sleep animation
    SleepDisplay { state: Switch },
    /// Temperature units
    Units { unit: Unit },
    /// Choose a built-in theme (1-7) or "custom"
    Preset { choice: String },
    /// Show your own images/GIFs/videos (copied to the screen over adb)
    Custom {
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// Overlay text colour, e.g. #ffffff
        #[arg(long)]
        color: Option<String>,
        /// Overlay alignment: Left, Center or Right
        #[arg(long)]
        align: Option<String>,
        /// Add to the current media instead of replacing it
        #[arg(long)]
        append: bool,
    },
    /// Live stats shown over the theme (up to 3), e.g. "CPU Temperature"
    Overlay { fields: Vec<String> },
    /// Fan on the pump head: smart curve, or fixed percent
    Fan { mode: FanMode, percent: Option<u8> },
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Switch {
    On,
    Off,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Unit {
    Celsius,
    Fahrenheit,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum FanMode {
    Smart,
    Fixed,
}

/// Runs a settings subcommand; returns a message for stdout.
pub fn run_settings_command(command: Command, store: &SettingsStore) -> Result<String, String> {
    match command {
        Command::Show => Ok(format!("{}\n# file: {}", serde_json::to_string_pretty(&store.load()?).unwrap_or_default(), store.path().display())),
        Command::Presets => Ok(presets_listing()),
        Command::Custom { files, color, align, append } => add_custom_media(store, &files, color, align, append),
        other => update(store, |settings| apply_change(other, settings)).map(|()| "saved".into()),
    }
}

fn presets_listing() -> String {
    let presets: Vec<String> = PRESETS.iter().enumerate().map(|(index, name)| format!("{}  {name}", index + 1)).collect();
    presets.join("\n") + "\ncustom  your own media (panorama custom FILE)"
}

fn update(store: &SettingsStore, change: impl FnOnce(&mut Settings) -> Result<(), String>) -> Result<(), String> {
    let mut settings = store.load()?;
    change(&mut settings)?;
    store.save(&settings)
}

fn apply_change(command: Command, settings: &mut Settings) -> Result<(), String> {
    match command {
        Command::Brightness { value } => settings.brightness = value,
        Command::Rotate { degrees } => settings.rotate = degrees,
        Command::Screen { state } => settings.enable = matches!(state, Switch::On),
        Command::SleepDisplay { state } => settings.display_in_sleep = matches!(state, Switch::On),
        Command::Units { unit } => settings.temperature_unit = if matches!(unit, Unit::Celsius) { CELSIUS } else { FAHRENHEIT }.into(),
        Command::Preset { choice } => settings.theme = theme_from_choice(&choice)?,
        Command::Overlay { fields } => *settings.active_overlay_mut() = fields,
        Command::Fan { mode, percent } => set_fan(settings, mode, percent),
        _ => return Err("not a settings command".into()),
    }
    Ok(())
}

fn theme_from_choice(choice: &str) -> Result<String, String> {
    if choice.eq_ignore_ascii_case("custom") {
        return Ok(CUSTOMIZATION.into());
    }
    match choice.parse::<usize>() {
        Ok(number) if (1..=PRESETS.len()).contains(&number) => Ok(PRESETS[number - 1].into()),
        _ => Err(format!("choose 1-{} or 'custom' (see: panorama presets)", PRESETS.len())),
    }
}

fn set_fan(settings: &mut Settings, mode: FanMode, percent: Option<u8>) {
    settings.fan.mode = if matches!(mode, FanMode::Fixed) { FIXED_MODE } else { SMART_MODE }.into();
    if let Some(percent) = percent {
        settings.fan.fixed_percent = percent;
    }
}

fn add_custom_media(store: &SettingsStore, files: &[PathBuf], color: Option<String>, align: Option<String>, append: bool) -> Result<String, String> {
    if let Some(align) = &align {
        if !ALIGNMENTS.contains(&align.as_str()) {
            return Err("align must be Left, Center or Right".into());
        }
    }
    let library = AdbScreen::new();
    let names = files.iter().map(|file| library.add(file)).collect::<Result<Vec<_>, _>>()?;
    let message = format!("copied {} to the screen", names.join(", "));
    update(store, |settings| {
        settings.add_custom_media(names, append);
        let style = &mut settings.custom.style;
        style.color = color.unwrap_or(style.color.clone());
        style.align = align.unwrap_or(style.align.clone());
        Ok(())
    })?;
    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::tests::temporary_path;

    fn run(store: &SettingsStore, arguments: &[&str]) -> Result<String, String> {
        let cli = Cli::try_parse_from(std::iter::once("panorama").chain(arguments.iter().copied())).map_err(|error| error.to_string())?;
        run_settings_command(cli.command.unwrap(), store)
    }

    #[test]
    fn brightness_and_rotation() {
        let store = SettingsStore::new(temporary_path("cli-basic"));
        run(&store, &["brightness", "40"]).unwrap();
        run(&store, &["rotate", "270"]).unwrap();
        let settings = store.load().unwrap();
        assert_eq!((settings.brightness, settings.rotate), (40, 270));
    }

    #[test]
    fn invalid_values_are_not_saved() {
        let store = SettingsStore::new(temporary_path("cli-invalid"));
        assert!(run(&store, &["brightness", "150"]).is_err());
        assert!(run(&store, &["rotate", "45"]).is_err());
        assert!(!store.path().exists());
    }

    #[test]
    fn preset_by_number_and_custom() {
        let store = SettingsStore::new(temporary_path("cli-preset"));
        run(&store, &["preset", "3"]).unwrap();
        assert_eq!(store.load().unwrap().theme, PRESETS[2]);
        run(&store, &["preset", "custom"]).unwrap();
        run(&store, &["overlay", "CPU Temperature", "GPU Temperature"]).unwrap();
        assert_eq!(store.load().unwrap().custom.overlay, ["CPU Temperature", "GPU Temperature"]);
    }

    #[test]
    fn fan_fixed_and_screen_off() {
        let store = SettingsStore::new(temporary_path("cli-fan"));
        run(&store, &["fan", "fixed", "70"]).unwrap();
        run(&store, &["screen", "off"]).unwrap();
        let settings = store.load().unwrap();
        assert_eq!((settings.fan.mode.as_str(), settings.fan.fixed_percent, settings.enable), (FIXED_MODE, 70, false));
    }
}
