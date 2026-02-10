use crate::models::{PresetRule, StripPreset, TagCategory};

pub fn builtin_presets() -> Vec<StripPreset> {
    vec![
        StripPreset::new(
            1,
            "Strip All",
            "Remove every metadata tag",
            "trash",
            vec![PresetRule::RemoveAll],
            true,
        ),
        StripPreset::new(
            2,
            "Privacy Clean",
            "Remove GPS, serial numbers, and software tags",
            "shield",
            vec![
                PresetRule::RemoveGps,
                PresetRule::RemoveCategory(TagCategory::Location),
                PresetRule::RemoveTag(String::from("Exif.Photo.BodySerialNumber")),
                PresetRule::RemoveTag(String::from("Exif.Image.CameraSerialNumber")),
                PresetRule::RemoveCategory(TagCategory::Software),
            ],
            true,
        ),
        StripPreset::new(
            3,
            "Social Media",
            "Keep orientation and display dimensions while stripping identifying data",
            "share",
            vec![PresetRule::RemoveAllExcept(vec![
                String::from("Exif.Image.Orientation"),
                String::from("Exif.Photo.PixelXDimension"),
                String::from("Exif.Photo.PixelYDimension"),
                String::from("Exif.Photo.ColorSpace"),
            ])],
            true,
        ),
        StripPreset::new(
            4,
            "GPS Only",
            "Remove only location metadata",
            "map-pin-off",
            vec![
                PresetRule::RemoveGps,
                PresetRule::RemoveCategory(TagCategory::Location),
            ],
            true,
        ),
        StripPreset::new(
            5,
            "Keep Basics",
            "Keep camera, capture, datetime, and image tags",
            "bookmark",
            vec![
                PresetRule::RemoveCategory(TagCategory::Location),
                PresetRule::RemoveCategory(TagCategory::Description),
                PresetRule::RemoveCategory(TagCategory::Software),
                PresetRule::RemoveCategory(TagCategory::Other),
            ],
            true,
        ),
        StripPreset::new(
            6,
            "Copyright Stamp",
            "Strip all tags then set the copyright field",
            "copyright",
            vec![
                PresetRule::RemoveAll,
                PresetRule::SetTag(
                    String::from("Exif.Image.Copyright"),
                    String::from("{user_value}"),
                ),
            ],
            true,
        ),
    ]
}

pub fn preset_by_name<'a>(presets: &'a [StripPreset], name: &str) -> Option<&'a StripPreset> {
    presets
        .iter()
        .find(|preset| preset.name.eq_ignore_ascii_case(name))
}
