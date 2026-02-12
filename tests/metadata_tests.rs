use std::path::Path;
use exif_editor::core::metadata::MetadataEngine;
use exif_editor::models::{
    MetadataTag, PhotoMetadata, PresetRule, StripPreset, TagCategory, TagValue,
};

#[test]
fn remove_gps_rule_drops_location_tags() {
    let mut metadata = PhotoMetadata {
        exif_tags: vec![
            MetadataTag::new(
                "Exif.GPSInfo.GPSLatitude",
                "GPS Latitude",
                TagValue::Gps(40.0, -74.0, None),
                TagCategory::Location,
            ),
            MetadataTag::new(
                "Exif.Image.Make",
                "Camera Make",
                TagValue::Text(String::from("Canon")),
                TagCategory::Camera,
            ),
        ],
        iptc_tags: Vec::new(),
        xmp_tags: Vec::new(),
        has_gps: true,
        date_taken: None,
        camera_make: None,
        camera_model: None,
    };

    let preset = StripPreset::new(
        1,
        "GPS",
        "remove location",
        "pin-off",
        vec![PresetRule::RemoveGps],
        false,
    );

    MetadataEngine::apply_preset_to_metadata(&mut metadata, &preset);

    assert!(!metadata.has_gps);
    assert_eq!(metadata.total_tag_count(), 1);
    assert_eq!(metadata.exif_tags[0].key, "Exif.Image.Make");
}

#[test]
fn remove_all_except_keeps_only_allowed_keys() {
    let mut metadata = PhotoMetadata {
        exif_tags: vec![
            MetadataTag::new(
                "Exif.Image.Orientation",
                "Orientation",
                TagValue::Integer(1),
                TagCategory::Image,
            ),
            MetadataTag::new(
                "Exif.Image.Make",
                "Camera Make",
                TagValue::Text(String::from("Canon")),
                TagCategory::Camera,
            ),
        ],
        iptc_tags: vec![MetadataTag::new(
            "Iptc.Application2.Caption",
            "Caption",
            TagValue::Text(String::from("Sample")),
            TagCategory::Description,
        )],
        xmp_tags: Vec::new(),
        has_gps: false,
        date_taken: None,
        camera_make: None,
        camera_model: None,
    };

    let preset = StripPreset::new(
        2,
        "Social",
        "keep only orientation",
        "share",
        vec![PresetRule::RemoveAllExcept(vec![String::from(
            "Exif.Image.Orientation",
        )])],
        false,
    );

    MetadataEngine::apply_preset_to_metadata(&mut metadata, &preset);

    assert_eq!(metadata.total_tag_count(), 1);
    assert_eq!(metadata.exif_tags[0].key, "Exif.Image.Orientation");
}

#[test]
fn set_tag_updates_existing_and_inserts_new() {
    let mut metadata = PhotoMetadata {
        exif_tags: vec![MetadataTag::new(
            "Exif.Image.Make",
            "Camera Make",
            TagValue::Text(String::from("Canon")),
            TagCategory::Camera,
        )],
        iptc_tags: Vec::new(),
        xmp_tags: Vec::new(),
        has_gps: false,
        date_taken: None,
        camera_make: None,
        camera_model: None,
    };

    MetadataEngine::set_tag_in_metadata(
        &mut metadata,
        "Exif.Image.Make",
        TagValue::Text(String::from("Sony")),
    );
    MetadataEngine::set_tag_in_metadata(
        &mut metadata,
        "Exif.Image.Copyright",
        TagValue::Text(String::from("(c) Exif Editor")),
    );

    assert_eq!(metadata.total_tag_count(), 2);
    assert!(metadata.all_tags().any(
        |tag| tag.key == "Exif.Image.Make" && tag.value == TagValue::Text(String::from("Sony"))
    ));
    assert!(metadata
        .all_tags()
        .any(|tag| tag.key == "Exif.Image.Copyright"));
}

#[test]
fn read_heic_extracts_full_exif() {
    let path = Path::new("demo_images/IMG_0205.HEIC");
    if !path.exists() {
        eprintln!("Skipping HEIC test: demo image not found");
        return;
    }

    let metadata = MetadataEngine::read(path).expect("should read HEIC metadata");

    // Camera make & model
    assert!(
        metadata
            .all_tags()
            .any(|tag| tag.key == "Exif.Image.Make"),
        "Expected camera Make tag"
    );
    assert!(
        metadata
            .all_tags()
            .any(|tag| tag.key == "Exif.Image.Model"),
        "Expected camera Model tag"
    );

    // Focal length
    assert!(
        metadata
            .all_tags()
            .any(|tag| tag.key == "Exif.Photo.FocalLength"),
        "Expected FocalLength tag"
    );

    // Exposure time
    assert!(
        metadata
            .all_tags()
            .any(|tag| tag.key == "Exif.Photo.ExposureTime"),
        "Expected ExposureTime tag"
    );

    // GPS coordinates
    assert!(
        metadata
            .all_tags()
            .any(|tag| tag.key == "Exif.GPSInfo.GPSCoordinates"),
        "Expected GPS coordinates"
    );
    assert!(metadata.has_gps, "has_gps should be true for HEIC with GPS");

    // Date taken
    assert!(
        metadata.date_taken.is_some(),
        "Expected date_taken to be populated"
    );

    println!(
        "HEIC EXIF: {} total tags, make={:?}, model={:?}, date={:?}, has_gps={}",
        metadata.total_tag_count(),
        metadata.camera_make,
        metadata.camera_model,
        metadata.date_taken,
        metadata.has_gps
    );
}
