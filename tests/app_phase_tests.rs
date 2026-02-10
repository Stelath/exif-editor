use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{SystemTime, UNIX_EPOCH};

use metastrip::app::{AppState, TableColumn, TableSort};
use metastrip::core::metadata::MetadataEngine;
use metastrip::models::{OutputMode, TagValue};

fn unique_path(name: &str, ext: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let mut path = std::env::temp_dir();
    path.push(format!("metastrip_{name}_{stamp}.{ext}"));
    path
}

fn write_file(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).expect("should create file");
}

fn cleanup_file(path: &Path) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(MetadataEngine::sidecar_path(path));
}

#[test]
fn phase2_sort_filter_and_inspector_query() {
    let file_a = unique_path("phase2_a", "jpg");
    let file_b = unique_path("phase2_b", "jpg");
    write_file(&file_a, b"small");
    write_file(&file_b, b"this-file-is-larger-than-a");

    let mut state = AppState::default();
    let skipped = state.import_paths([file_a.clone(), file_b.clone()]);
    assert!(skipped.is_empty());
    assert_eq!(state.photos.len(), 2);

    state
        .edit_tag(0, "Exif.Image.Make", TagValue::Text(String::from("Canon")))
        .expect("edit should succeed");
    state
        .edit_tag(1, "Exif.Image.Make", TagValue::Text(String::from("Sony")))
        .expect("edit should succeed");

    state.set_search_query("sony");
    let visible = state.visible_photos();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].filename, state.photos[1].filename);

    state.set_search_query("");
    state.set_table_sort(TableSort {
        column: TableColumn::FileSize,
        descending: true,
    });
    let sorted = state.sorted_visible_indices();
    assert_eq!(sorted[0], 1);

    state.active_photo = Some(1);
    state.set_metadata_search_query("make");
    let tags = state.inspector_tags(1);
    assert!(tags.iter().any(|tag| tag.key == "Exif.Image.Make"));

    cleanup_file(&file_a);
    cleanup_file(&file_b);
}

#[test]
fn phase3_edit_save_revert_strip_and_undo() {
    let file = unique_path("phase3", "png");
    write_file(&file, b"phase3-input");

    let mut state = AppState::default();
    state.import_paths([file.clone()]);
    assert_eq!(state.photos.len(), 1);

    state
        .edit_tag(0, "Exif.Image.Make", TagValue::Text(String::from("Nikon")))
        .expect("edit should succeed");
    assert!(state.photos[0].dirty);

    assert!(state.undo_last_change());
    assert!(!state.photos[0].dirty);

    state
        .edit_tag(0, "Exif.Image.Make", TagValue::Text(String::from("Nikon")))
        .expect("edit should succeed");
    state.save_photo_changes(0).expect("save should succeed");
    assert!(!state.photos[0].dirty);

    let reloaded = MetadataEngine::read(&file).expect("metadata reload should succeed");
    assert!(reloaded
        .all_tags()
        .any(|tag| tag.key == "Exif.Image.Make"
            && tag.value == TagValue::Text(String::from("Nikon"))));

    state
        .apply_preset_to_photo(0, 1)
        .expect("strip-all preset should exist");
    assert!(state.photos[0].dirty);
    assert_eq!(state.photos[0].metadata.total_tag_count(), 0);

    state.revert_photo(0).expect("revert should succeed");
    assert!(!state.photos[0].dirty);
    assert!(state.photos[0]
        .metadata
        .all_tags()
        .any(|tag| tag.key == "Exif.Image.Make"));

    state
        .mark_tag_for_removal(0, "Exif.Image.Make", true)
        .expect("mark should succeed");
    let removed = state.strip_marked_tags(0).expect("strip should succeed");
    assert_eq!(removed, 1);

    state.save_photo_changes(0).expect("save should succeed");
    state
        .reload_photo_from_disk(0)
        .expect("reload should succeed");
    assert!(!state.photos[0]
        .metadata
        .all_tags()
        .any(|tag| tag.key == "Exif.Image.Make"));

    cleanup_file(&file);
}

#[test]
fn phase4_bulk_selected_with_summary_and_cancel() {
    let file_a = unique_path("phase4_a", "jpg");
    let file_b = unique_path("phase4_b", "jpg");
    let file_c = unique_path("phase4_c", "jpg");
    write_file(&file_a, b"a");
    write_file(&file_b, b"bbbb");
    write_file(&file_c, b"cccccccc");

    let mut state = AppState::default();
    state.import_paths([file_a.clone(), file_b.clone(), file_c.clone()]);
    state.select_range(0, 2);

    let summary = state
        .run_bulk_selected(1, OutputMode::Suffix(String::from("_clean")), None)
        .expect("bulk should succeed");
    assert_eq!(summary.total, 3);
    assert_eq!(summary.succeeded, 3);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.cancelled, 0);
    assert_eq!(state.operation_results.len(), 3);

    let output_paths = state
        .operation_results
        .iter()
        .map(|result| result.output_path.clone())
        .collect::<Vec<_>>();

    for path in &output_paths {
        assert!(path.exists());
    }

    assert!(state.photos.len() >= 6);

    state.clear_selection();
    state.select_range(0, 1);

    let cancel = AtomicBool::new(true);
    let cancelled = state
        .run_bulk_selected(1, OutputMode::Overwrite, Some(&cancel))
        .expect("bulk should return a cancelled summary");

    assert_eq!(cancelled.total, 2);
    assert_eq!(cancelled.succeeded, 0);
    assert_eq!(cancelled.failed, 0);
    assert_eq!(cancelled.cancelled, 2);

    cleanup_file(&file_a);
    cleanup_file(&file_b);
    cleanup_file(&file_c);

    for output in output_paths {
        cleanup_file(&output);
    }
}
