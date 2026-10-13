//! Gives panorama.exe the app icon, so Explorer, the Start menu and pinned taskbar buttons show it.

#[allow(dead_code)]
#[path = "src/app_icon.rs"]
mod app_icon;

use image::ExtendedColorType;
use image::codecs::ico::{IcoEncoder, IcoFrame};
use std::path::{Path, PathBuf};

const ICON_SIZES: [u32; 6] = [16, 24, 32, 48, 64, app_icon::LARGEST_SIZE];

fn main() {
    println!("cargo:rerun-if-changed=src/app_icon.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let icon_file = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR")).join("panorama.ico");
    write_icon(&icon_file).expect("cannot write panorama.ico");
    winresource::WindowsResource::new()
        .set_icon(&icon_file.to_string_lossy())
        .compile()
        .expect("cannot embed the icon in panorama.exe (needs the Windows SDK's rc.exe)");
}

fn write_icon(path: &Path) -> image::ImageResult<()> {
    let frames = ICON_SIZES
        .iter()
        .map(|&size| IcoFrame::as_png(&app_icon::rgba(size), size, size, ExtendedColorType::Rgba8))
        .collect::<Result<Vec<_>, _>>()?;
    IcoEncoder::new(std::fs::File::create(path)?).encode_images(&frames)
}
