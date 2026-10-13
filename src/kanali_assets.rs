//! Built-in theme pictures from an installed KANALI. Panorama doesn't ship TRYX's artwork; when
//! KANALI is installed, its app.asar holds a poster for each theme in out/renderer/assets.
//!
//! An asar file is a 16-byte prefix, a JSON index of every file (size and offset), then the files.

use serde_json::Value;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const ASSETS_DIRECTORY: [&str; 3] = ["out", "renderer", "assets"];
const POSTER_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

pub struct KanaliArchive {
    path: PathBuf,
    index: Value,
    data_start: u64,
}

impl KanaliArchive {
    /// The installed KANALI's archive, if there is one (KANALI is Windows-only).
    pub fn installed() -> Option<Self> {
        let program_files = std::env::var_os("ProgramFiles")?;
        Self::open(&PathBuf::from(program_files).join(r"KANALI\resources\app.asar")).ok()
    }

    pub fn open(path: &Path) -> std::io::Result<Self> {
        let mut file = File::open(path)?;
        let mut prefix = [0u8; 16];
        file.read_exact(&mut prefix)?;
        let header_size = u32_at(&prefix, 4);
        let mut index_text = vec![0u8; u32_at(&prefix, 12) as usize];
        file.read_exact(&mut index_text)?;
        let index = serde_json::from_slice(&index_text).map_err(std::io::Error::other)?;
        Ok(Self { path: path.to_path_buf(), index, data_start: 8 + header_size as u64 })
    }

    /// The poster for a theme name like "Cyber Bunker", matched loosely against file names
    /// such as "Cyber Bunker-70b54cb4.png" or "CoolingDelivery-0d5484ec.png".
    pub fn preset_poster(&self, theme_name: &str) -> Option<Vec<u8>> {
        let assets = ASSETS_DIRECTORY.iter().try_fold(&self.index, |entry, name| entry.get("files")?.get(name))?;
        let wanted = loose_name(theme_name);
        let (_, entry) = assets.get("files")?.as_object()?.iter().find(|(file_name, _)| is_poster_for(file_name, &wanted))?;
        self.read_entry(entry).ok()
    }

    fn read_entry(&self, entry: &Value) -> std::io::Result<Vec<u8>> {
        let missing = || std::io::Error::other("asar entry without offset or size");
        let offset: u64 = entry["offset"].as_str().and_then(|text| text.parse().ok()).ok_or_else(missing)?;
        let size = entry["size"].as_u64().ok_or_else(missing)?;
        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(self.data_start + offset))?;
        let mut bytes = vec![0u8; size as usize];
        file.read_exact(&mut bytes)?;
        Ok(bytes)
    }
}

fn u32_at(bytes: &[u8], start: usize) -> u32 {
    u32::from_le_bytes(bytes[start..start + 4].try_into().expect("four bytes"))
}

/// "Thermal Energy · Prohibited-da63eeb3.png" is the poster for "thermalenergyprohibited".
fn is_poster_for(file_name: &str, wanted: &str) -> bool {
    let Some((stem, extension)) = file_name.rsplit_once('.') else { return false };
    let name = stem.rsplit_once('-').map_or(stem, |(name, _hash)| name);
    POSTER_EXTENSIONS.contains(&extension.to_lowercase().as_str()) && loose_name(name) == wanted
}

fn loose_name(name: &str) -> String {
    name.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn archive_with(files: &[(&str, &[u8])]) -> PathBuf {
        let mut entries = serde_json::Map::new();
        let mut data = Vec::new();
        for (name, bytes) in files {
            entries.insert(name.to_string(), json!({ "size": bytes.len(), "offset": data.len().to_string() }));
            data.extend_from_slice(bytes);
        }
        let index = json!({ "files": { "out": { "files": { "renderer": { "files": { "assets": { "files": entries } } } } } } }).to_string();
        let mut archive = Vec::new();
        for value in [4, index.len() as u32 + 8, index.len() as u32 + 4, index.len() as u32] {
            archive.extend_from_slice(&value.to_le_bytes());
        }
        archive.extend_from_slice(index.as_bytes());
        archive.extend_from_slice(&data);
        let path = std::env::temp_dir().join(format!("panorama-test-{}.asar", std::process::id()));
        std::fs::write(&path, archive).unwrap();
        path
    }

    #[test]
    fn finds_posters_by_loose_name() {
        let path = archive_with(&[("Cyber Bunker-5ec4689a.mp4", b"video"), ("CoolingDelivery-0d5484ec.png", b"cool"), ("Thermal Energy · Prohibited-da63eeb3.png", b"hot")]);
        let archive = KanaliArchive::open(&path).unwrap();
        assert_eq!(archive.preset_poster("Cooling delivery").as_deref(), Some(&b"cool"[..]));
        assert_eq!(archive.preset_poster("Thermal Energy Prohibited").as_deref(), Some(&b"hot"[..]));
        assert_eq!(archive.preset_poster("Cyber Bunker"), None);
    }
}
