//! Custom images and videos live on the screen in /sdcard/pcMedia, pushed over ADB.

use crate::hidden_command::hidden_command;
use std::path::{Path, PathBuf};

const DEVICE_MEDIA_DIRECTORY: &str = "/sdcard/pcMedia";
pub const SUPPORTED_EXTENSIONS: [&str; 7] = ["png", "jpg", "jpeg", "gif", "webp", "mp4", "avi"];
const MAX_LIBRARY_BYTES: u64 = 500 * 1024 * 1024;
#[cfg(windows)]
const ADB_INSTALL_HINT: &str = "install it with: winget install Google.PlatformTools";
#[cfg(not(windows))]
const ADB_INSTALL_HINT: &str = "install it with: sudo pacman -S android-tools";

pub struct AdbMediaLibrary {
    adb: PathBuf,
}

impl AdbMediaLibrary {
    pub fn new() -> Self {
        Self { adb: std::env::var_os("PANORAMA_ADB").map(PathBuf::from).unwrap_or_else(default_adb) }
    }

    /// Copies a file to the screen and returns the name it has there.
    pub fn add(&self, local_file: &Path) -> Result<String, String> {
        require_supported(local_file)?;
        let name = device_file_name(local_file);
        self.run(&["shell", "mkdir", "-p", DEVICE_MEDIA_DIRECTORY])?;
        self.run(&["push", &local_file.to_string_lossy(), &format!("{DEVICE_MEDIA_DIRECTORY}/{name}")])?;
        Ok(name)
    }

    fn run(&self, arguments: &[&str]) -> Result<String, String> {
        let output = hidden_command(&self.adb)
            .args(arguments)
            .output()
            .map_err(|error| format!("cannot run adb ({error}); {ADB_INSTALL_HINT}"))?;
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
        }
        let message = String::from_utf8_lossy(if output.stderr.is_empty() { &output.stdout } else { &output.stderr }).trim().to_string();
        Err(format!("adb {} failed: {message}", arguments[0]))
    }
}

/// adb from PATH, or on Windows where winget puts it (PATH only picks that up after logging in again).
fn default_adb() -> PathBuf {
    adb_install_locations().into_iter().find(|path| path.is_file()).unwrap_or_else(|| PathBuf::from("adb"))
}

#[cfg(windows)]
fn adb_install_locations() -> Vec<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|local| PathBuf::from(local).join(r"Microsoft\WinGet\Links\adb.exe")).into_iter().collect()
}

#[cfg(not(windows))]
fn adb_install_locations() -> Vec<PathBuf> {
    Vec::new()
}

pub fn device_file_name(local_file: &Path) -> String {
    let stem: String = local_file
        .file_stem()
        .map(|stem| stem.to_string_lossy().chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '_' }).collect())
        .unwrap_or_default();
    let stem = stem.trim_matches(|c| c == '.' || c == '_');
    let stem = if stem.is_empty() { "media" } else { stem };
    format!("{stem}.{}", extension(local_file))
}

fn extension(path: &Path) -> String {
    path.extension().map(|extension| extension.to_string_lossy().to_lowercase()).unwrap_or_default()
}

fn require_supported(local_file: &Path) -> Result<(), String> {
    let metadata = std::fs::metadata(local_file).map_err(|error| format!("{}: {error}", local_file.display()))?;
    if !SUPPORTED_EXTENSIONS.contains(&extension(local_file).as_str()) {
        return Err(format!("unsupported file type; use one of: {}", SUPPORTED_EXTENSIONS.join(", ")));
    }
    if metadata.len() > MAX_LIBRARY_BYTES {
        return Err("file is larger than the screen's 500 MB media library".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_names_are_safe() {
        assert_eq!(device_file_name(Path::new("/home/me/My Wallpaper (1).GIF")), "My_Wallpaper__1.gif");
        assert_eq!(device_file_name(Path::new("clip.mp4")), "clip.mp4");
        assert_eq!(device_file_name(Path::new("....png")), "media.png");
    }

    #[test]
    fn rejects_unsupported_type() {
        let path = std::env::temp_dir().join(format!("panorama-media-{}.txt", std::process::id()));
        std::fs::write(&path, "x").unwrap();
        assert!(require_supported(&path).unwrap_err().contains("unsupported"));
    }
}
