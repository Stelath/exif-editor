use std::fs;
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

use exif_editor::core::bulk::BulkProcessor;
use exif_editor::models::{ImageFormat, OutputMode, PhotoEntry, PresetRule, StripPreset};

fn unique_path(name: &str, ext: &str) -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let mut path = std::env::temp_dir();
    path.push(format!("exif_editor_{name}_{stamp}.{ext}"));
    path
}

#[test]
fn process_with_suffix_creates_output_file_and_progress_event() {
    let input = unique_path("input", "jpg");
    fs::write(&input, b"sample-bytes").expect("should create input file");

    let photo = PhotoEntry::from_path(1, input.clone(), ImageFormat::Jpeg);
    let preset = StripPreset::new(
        1,
        "Strip All",
        "remove all",
        "trash",
        vec![PresetRule::RemoveAll],
        false,
    );

    let (tx, rx) = mpsc::channel();
    let results = BulkProcessor::process(
        &[photo],
        &preset,
        &OutputMode::Suffix(String::from("_clean")),
        tx,
    );

    assert_eq!(results.len(), 1);
    assert!(results[0].success);
    assert!(results[0].output_path.exists());

    let event = rx.recv().expect("should receive progress event");
    assert_eq!(event.current, 1);
    assert_eq!(event.total, 1);
    assert!(event.success);

    let _ = fs::remove_file(&input);
    let _ = fs::remove_file(&results[0].output_path);
}

#[test]
fn process_with_export_dir_writes_to_export_location() {
    let input = unique_path("input_export", "png");
    fs::write(&input, b"sample-bytes").expect("should create input file");

    let export_dir = unique_path("export_dir", "tmp");
    fs::create_dir_all(&export_dir).expect("should create export directory");

    let photo = PhotoEntry::from_path(2, input.clone(), ImageFormat::Png);
    let preset = StripPreset::new(
        2,
        "GPS",
        "remove gps",
        "pin-off",
        vec![PresetRule::RemoveGps],
        false,
    );

    let (tx, _rx) = mpsc::channel();
    let results = BulkProcessor::process(
        &[photo],
        &preset,
        &OutputMode::ExportTo(export_dir.clone()),
        tx,
    );

    assert_eq!(results.len(), 1);
    assert!(results[0].success);
    assert!(results[0].output_path.starts_with(&export_dir));
    assert!(results[0].output_path.exists());

    let _ = fs::remove_file(&input);
    let _ = fs::remove_file(&results[0].output_path);
    let _ = fs::remove_dir_all(&export_dir);
}
