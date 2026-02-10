use std::path::Path;

use crate::models::ImageFormat;

pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "tif", "tiff", "webp", "heic", "heif", "avif", "jxl",
];

pub fn detect_format(path: &Path) -> ImageFormat {
    let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
        return ImageFormat::Unknown;
    };

    match ext.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => ImageFormat::Jpeg,
        "png" => ImageFormat::Png,
        "tif" | "tiff" => ImageFormat::Tiff,
        "webp" => ImageFormat::WebP,
        "heic" | "heif" => ImageFormat::Heif,
        "avif" => ImageFormat::Avif,
        "jxl" => ImageFormat::Jxl,
        _ => ImageFormat::Unknown,
    }
}

pub fn is_supported(path: &Path) -> bool {
    !detect_format(path).is_unknown()
}

pub fn supported_extensions() -> &'static [&'static str] {
    SUPPORTED_EXTENSIONS
}
