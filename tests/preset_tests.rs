use exif_editor::core::presets::builtin_presets;
use exif_editor::models::{PresetRule, TagCategory};

#[test]
fn builtin_presets_include_expected_names() {
    let presets = builtin_presets();
    let names = presets
        .iter()
        .map(|preset| preset.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(presets.len(), 6);
    assert!(names.contains(&"Strip All"));
    assert!(names.contains(&"Privacy Clean"));
    assert!(names.contains(&"Social Media"));
    assert!(names.contains(&"GPS Only"));
    assert!(names.contains(&"Keep Basics"));
    assert!(names.contains(&"Copyright Stamp"));
}

#[test]
fn privacy_clean_has_gps_and_software_rules() {
    let presets = builtin_presets();
    let privacy = presets
        .iter()
        .find(|preset| preset.name == "Privacy Clean")
        .expect("privacy clean preset should exist");

    assert!(privacy.rules.contains(&PresetRule::RemoveGps));
    assert!(privacy
        .rules
        .contains(&PresetRule::RemoveCategory(TagCategory::Software)));
    assert!(privacy.rules.contains(&PresetRule::RemoveTag(String::from(
        "Exif.Photo.BodySerialNumber"
    ))));
}
