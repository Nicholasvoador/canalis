//! File locations. Linux: settings in the XDG config directory, live status in the runtime
//! directory. Windows: settings in %APPDATA%, live status and the service log in %LOCALAPPDATA%.

use std::path::PathBuf;

const CONFIG_DIRECTORY_VARIABLES: [&str; 2] = ["XDG_CONFIG_HOME", "APPDATA"];
const RUNTIME_DIRECTORY_VARIABLES: [&str; 2] = ["XDG_RUNTIME_DIR", "LOCALAPPDATA"];

pub fn settings_path() -> PathBuf {
    settings_path_from(&environment_variable, home_directory())
}

pub fn status_path() -> PathBuf {
    status_path_from(&environment_variable, home_directory())
}

/// Local copies of the media on the screen, used for the window's preview.
pub fn media_directory() -> PathBuf {
    settings_path().with_file_name("media")
}

/// Pictures of the built-in themes for the carousel (from KANALI, or captured from the screen).
pub fn preset_art_directory() -> PathBuf {
    settings_path().with_file_name("presets")
}

/// Where the Windows service writes its messages (Linux uses the systemd journal).
#[cfg(windows)]
pub fn log_path() -> PathBuf {
    status_path().with_file_name("service.log")
}

fn settings_path_from(lookup: &dyn Fn(&str) -> Option<String>, home: PathBuf) -> PathBuf {
    let config_home = first_variable(lookup, &CONFIG_DIRECTORY_VARIABLES).unwrap_or_else(|| home.join(".config"));
    config_home.join("panorama").join("settings.json")
}

fn status_path_from(lookup: &dyn Fn(&str) -> Option<String>, home: PathBuf) -> PathBuf {
    let runtime = first_variable(lookup, &RUNTIME_DIRECTORY_VARIABLES).unwrap_or_else(|| home.join(".cache"));
    runtime.join("panorama").join("status.json")
}

fn first_variable(lookup: &dyn Fn(&str) -> Option<String>, names: &[&str]) -> Option<PathBuf> {
    names.iter().find_map(|name| lookup(name)).map(PathBuf::from)
}

fn environment_variable(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn home_directory() -> PathBuf {
    std::env::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_variables(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn settings_default_to_dot_config() {
        let path = settings_path_from(&no_variables, PathBuf::from("/home/me"));
        assert_eq!(path, PathBuf::from("/home/me/.config/panorama/settings.json"));
    }

    #[test]
    fn settings_use_appdata_on_windows() {
        let lookup = |name: &str| (name == "APPDATA").then(|| r"C:\Users\me\AppData\Roaming".to_string());
        let path = settings_path_from(&lookup, PathBuf::from(r"C:\Users\me"));
        assert_eq!(path, PathBuf::from(r"C:\Users\me\AppData\Roaming").join("panorama").join("settings.json"));
    }

    #[test]
    fn status_uses_runtime_directory() {
        let lookup = |name: &str| (name == "XDG_RUNTIME_DIR").then(|| "/run/user/1000".to_string());
        let path = status_path_from(&lookup, PathBuf::from("/home/me"));
        assert_eq!(path, PathBuf::from("/run/user/1000/panorama/status.json"));
    }

    #[test]
    fn status_uses_local_appdata_on_windows() {
        let lookup = |name: &str| (name == "LOCALAPPDATA").then(|| r"C:\Users\me\AppData\Local".to_string());
        let path = status_path_from(&lookup, PathBuf::from(r"C:\Users\me"));
        assert_eq!(path, PathBuf::from(r"C:\Users\me\AppData\Local").join("panorama").join("status.json"));
    }

    #[test]
    fn status_falls_back_to_cache() {
        let path = status_path_from(&no_variables, PathBuf::from("/home/me"));
        assert_eq!(path, PathBuf::from("/home/me/.cache/panorama/status.json"));
    }
}
