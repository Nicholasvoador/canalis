//! Uses an installed sans-serif font (regular + bold) when one is available;
//! falls back to egui's built-in font otherwise.

use super::theme::BOLD;
use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

/// (regular, bold) pairs, tried in order.
const CANDIDATES: [(&str, &str); 7] = [
    ("/usr/share/fonts/inter/InterVariable.ttf", "/usr/share/fonts/inter/InterVariable.ttf"),
    ("/usr/share/fonts/noto/NotoSans-Regular.ttf", "/usr/share/fonts/noto/NotoSans-Bold.ttf"),
    ("/usr/share/fonts/TTF/DejaVuSans.ttf", "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf"),
    ("/usr/share/fonts/liberation/LiberationSans-Regular.ttf", "/usr/share/fonts/liberation/LiberationSans-Bold.ttf"),
    ("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"),
    ("C:\\Windows\\Fonts\\segoeui.ttf", "C:\\Windows\\Fonts\\segoeuib.ttf"),
    ("C:\\Windows\\Fonts\\arial.ttf", "C:\\Windows\\Fonts\\arialbd.ttf"),
];

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let bold_fallbacks = fonts.families[&FontFamily::Proportional].clone();
    fonts.families.insert(FontFamily::Name(BOLD.into()), bold_fallbacks);
    if let Some((regular, bold)) = CANDIDATES.iter().find_map(|(regular, bold)| Some((std::fs::read(regular).ok()?, std::fs::read(bold).ok()?))) {
        add_primary(&mut fonts, FontFamily::Proportional, "system-regular", regular);
        add_primary(&mut fonts, FontFamily::Name(BOLD.into()), "system-bold", bold);
    }
    ctx.set_fonts(fonts);
}

fn add_primary(fonts: &mut FontDefinitions, family: FontFamily, name: &str, bytes: Vec<u8>) {
    fonts.font_data.insert(name.to_string(), Arc::new(FontData::from_owned(bytes)));
    fonts.families.entry(family).or_default().insert(0, name.to_string());
}
