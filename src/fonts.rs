use font_kit::family_name::FamilyName;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;

/// CJK font families to search for, in preference order.
const CJK_FAMILIES: &[&str] = &[
    "Noto Sans CJK KR",
    "Noto Sans CJK",
    "Noto Sans KR",
    "Noto Sans CJK SC",
    "Noto Sans CJK JP",
    "Apple SD Gothic Neo",
    "PingFang SC",
    "Malgun Gothic",
    "Microsoft YaHei",
    "Meiryo",
    "Gulim",
    "WenQuanYi Zen Hei",
    "Arial Unicode MS",
];

/// Query the OS font database for a CJK font and return its raw bytes.
fn find_cjk_font() -> Option<Vec<u8>> {
    let source = SystemSource::new();

    // Try each known CJK family name
    for family in CJK_FAMILIES {
        if let Ok(handle) = source.select_best_match(
            &[FamilyName::Title(family.to_string())],
            &Properties::new(),
        ) {
            if let Ok(font) = handle.load() {
                if let Some(data) = font.copy_font_data() {
                    return Some((*data).clone());
                }
            }
        }
    }

    None
}

/// Configure egui fonts with CJK support if a system font is available.
/// Call this in the eframe creation callback.
pub fn configure_fonts(ctx: &egui::Context) {
    let Some(cjk_data) = find_cjk_font() else {
        return;
    };

    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("system_cjk".to_owned(), egui::FontData::from_owned(cjk_data));

    // Append CJK font as fallback after the default proportional font
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        family.push("system_cjk".to_owned());
    }
    // Also for monospace (text preview uses code_editor)
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
        family.push("system_cjk".to_owned());
    }

    ctx.set_fonts(fonts);
}
