//! The screen over ADB: custom images and videos live in /sdcard/pcMedia, and screenshots show
//! what it displays. A local copy of each media file is kept for the window's preview.

use crate::hidden_command::hidden_command;
use crate::paths;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const DEVICE_MEDIA_DIRECTORY: &str = "/sdcard/pcMedia";
pub const SUPPORTED_EXTENSIONS: [&str; 7] = ["png", "jpg", "jpeg", "gif", "webp", "mp4", "avi"];
const MAX_LIBRARY_BYTES: u64 = 500 * 1024 * 1024;
#[cfg(windows)]
const ADB_INSTALL_HINT: &str = "install it with: winget install Google.PlatformTools";
#[cfg(not(windows))]
const ADB_INSTALL_HINT: &str = "install it with: sudo pacman -S android-tools";

/// How the cooler's screen lists itself in `adb devices -l` (firmware V1.0.11).
const COOLER_MODEL: &str = "model:cm01";

pub struct AdbScreen {
    adb: PathBuf,
    local_directory: PathBuf,
    serial: OnceLock<Option<String>>,
}

impl AdbScreen {
    pub fn new() -> Self {
        let adb = std::env::var_os("CANALIS_ADB").map(PathBuf::from).unwrap_or_else(default_adb);
        Self { adb, local_directory: paths::media_directory(), serial: OnceLock::new() }
    }

    /// A PNG of what the screen shows right now (2240×1080).
    pub fn screenshot_png(&self) -> Result<Vec<u8>, String> {
        self.run_binary(&["exec-out", "screencap", "-p"])
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
            eprintln!("canalis: cannot keep a preview copy of {name}: {error}");
        }
    }

    fn run(&self, arguments: &[&str]) -> Result<String, String> {
        self.run_binary(arguments).map(|output| String::from_utf8_lossy(&output).into_owned())
    }

    fn run_binary(&self, arguments: &[&str]) -> Result<Vec<u8>, String> {
        let mut command = hidden_command(&self.adb);
        if let Some(serial) = self.serial() {
            command.args(["-s", serial]);
        }
        let output = command.args(arguments).output().map_err(|error| format!("cannot run adb ({error}); {ADB_INSTALL_HINT}"))?;
        if output.status.success() {
            return Ok(output.stdout);
        }
        let message = String::from_utf8_lossy(if output.stderr.is_empty() { &output.stdout } else { &output.stderr }).trim().to_string();
        Err(format!("adb {} failed: {message}", arguments[0]))
    }

    /// With other Android devices attached, adb must be told which one is the cooler.
    fn serial(&self) -> Option<&str> {
        self.serial.get_or_init(|| self.device_listing().and_then(|listing| cooler_serial(&listing))).as_deref()
    }

    fn device_listing(&self) -> Option<String> {
        let output = hidden_command(&self.adb).args(["devices", "-l"]).output().ok()?;
        Some(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// The cooler's serial from `adb devices -l`, or the only online device when none looks like the cooler.
fn cooler_serial(listing: &str) -> Option<String> {
    let online: Vec<&str> = listing.lines().skip(1).filter(|line| line.split_whitespace().nth(1) == Some("device")).collect();
    let cooler = online.iter().find(|line| line.contains(COOLER_MODEL)).or(online.first().filter(|_| online.len() == 1));
    cooler.and_then(|line| line.split_whitespace().next()).map(str::to_string)
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
    fn picks_the_cooler_among_other_devices() {
        let listing = "List of devices attached\nBYZL25040200101181     device product:cm01 model:cm01 device:cm01 transport_id:1\nemulator-5562          offline transport_id:28\n";
        assert_eq!(cooler_serial(listing).as_deref(), Some("BYZL25040200101181"));
        let phone_and_cooler = "List of devices attached\nR58M  device model:SM_G991B\nBYZL  device model:cm01\n";
        assert_eq!(cooler_serial(phone_and_cooler).as_deref(), Some("BYZL"));
        assert_eq!(cooler_serial("List of devices attached\nA device model:x\nB device model:y\n"), None);
    }

    #[test]
    fn rejects_unsupported_type() {
        let path = std::env::temp_dir().join(format!("canalis-media-{}.txt", std::process::id()));
        std::fs::write(&path, "x").unwrap();
        assert!(require_supported(&path).unwrap_err().contains("unsupported"));
    }
}
