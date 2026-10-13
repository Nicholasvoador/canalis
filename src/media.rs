//! Custom images and videos live on the screen in /sdcard/pcMedia, pushed over ADB.
//! A local copy of each file is kept for the window's preview.

use crate::hidden_command::hidden_command;
use crate::paths;
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
    local_directory: PathBuf,
}

impl AdbMediaLibrary {
    pub fn new() -> Self {
        let adb = std::env::var_os("PANORAMA_ADB").map(PathBuf::from).unwrap_or_else(default_adb);
        Self { adb, local_directory: paths::media_directory() }
    }

    /// Copies a file to the screen and returns the name it has there.
    pub fn add(&self, local_file: &Path) -> Result<String, String> {
        require_supported(local_file)?;
        let name = device_file_name(local_file);
        self.run(&["shell", "mkdir", "-p", DEVICE_MEDIA_DIRECTORY])?;
        self.run(&["push", &local_file.to_string_lossy(), &format!("{DEVICE_MEDIA_DIRECTORY}/{name}")])?;
        self.keep_local_copy(local_file, &name);
        Ok(name)
    }

    /// The local copy of a file on the screen, pulled from the screen the first time.
    pub fn local_copy(&self, name: &str) -> Result<PathBuf, String> {
        let local_file = self.local_directory.join(name);
        if local_file.is_file() {
            return Ok(local_file);
        }
        std::fs::create_dir_all(&self.local_directory).map_err(|error| format!("{}: {error}", self.local_directory.display()))?;
        self.run(&["pull", &format!("{DEVICE_MEDIA_DIRECTORY}/{name}"), &local_file.to_string_lossy()])?;
        Ok(local_file)
    }

    /// The preview is a convenience, so a failed copy doesn't fail the upload.
    fn keep_local_copy(&self, local_file: &Path, name: &str) {
        let copied = std::fs::create_dir_all(&self.local_directory).and_then(|()| std::fs::copy(local_file, self.local_directory.join(name)));
        if let Err(error) = copied {
            eprintln!("panorama: cannot keep a preview copy of {name}: {error}");
        }
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

/// adb from where winget puts it on Windows (PATH only picks that up after logging in again), or from PATH.
fn default_adb() -> PathBuf {
    adb_install_locations().into_iter().find(|path| path.is_file()).unwrap_or_else(|| PathBuf::from("adb"))
}

#[cfg(windows)]
fn adb_install_locations() -> Vec<PathBuf> {
    let Some(winget) = std::env::var_os("LOCALAPPDATA").map(|local| PathBuf::from(local).join(r"Microsoft\WinGet")) else {
        return Vec::new();
    };
    let packages = std::fs::read_dir(winget.join("Packages")).into_iter().flatten().flatten();
    let platform_tools = packages
        .filter(|package| package.file_name().to_string_lossy().starts_with("Google.PlatformTools"))
        .map(|package| package.path().join(r"platform-tools\adb.exe"));
    std::iter::once(winget.join(r"Links\adb.exe")).chain(platform_tools).collect()
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
