//! Gives canalis.exe the app icon, so Explorer, the Start menu and pinned taskbar buttons show it.

#[allow(dead_code)]
#[path = "src/app_icon.rs"]
mod app_icon;

use image::ExtendedColorType;
use image::codecs::ico::{IcoEncoder, IcoFrame};
use std::path::{Path, PathBuf};

const ICON_SIZES: [u32; 6] = [16, 24, 32, 48, 64, app_icon::LARGEST_SIZE];

fn main() {
    println!("cargo:rerun-if-changed=src/app_icon.rs");
    println!("cargo:rerun-if-changed=assets/icons/logo.svg");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let icon_file = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR")).join("canalis.ico");
    write_icon(&icon_file).expect("cannot write canalis.ico");
    winresource::WindowsResource::new()
        .set_icon(&icon_file.to_string_lossy())
        .set("ProductName", "Canalis")
        .set("FileDescription", "Canalis")
        .compile()
        .expect("cannot embed the icon in canalis.exe (needs the Windows SDK's rc.exe)");
}

fn write_icon(path: &Path) -> image::ImageResult<()> {
    let frames = ICON_SIZES
        .iter()
        .map(|&size| IcoFrame::as_png(&app_icon::rgba(size), size, size, ExtendedColorType::Rgba8))
        .collect::<Result<Vec<_>, _>>()?;
    IcoEncoder::new(std::fs::File::create(path)?).encode_images(&frames)
}
