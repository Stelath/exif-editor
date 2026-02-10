use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc::Sender,
};

use rayon::prelude::*;

use crate::core::metadata::MetadataEngine;
use crate::models::{OperationResult, OutputMode, PhotoEntry, ProgressEvent, StripPreset};

pub struct BulkProcessor;

impl BulkProcessor {
    pub fn process(
        photos: &[PhotoEntry],
        preset: &StripPreset,
        output_mode: &OutputMode,
        progress_tx: Sender<ProgressEvent>,
    ) -> Vec<OperationResult> {
        Self::process_with_cancel(photos, preset, output_mode, progress_tx, None)
    }

    pub fn process_with_cancel(
        photos: &[PhotoEntry],
        preset: &StripPreset,
        output_mode: &OutputMode,
        progress_tx: Sender<ProgressEvent>,
        cancel_flag: Option<&AtomicBool>,
    ) -> Vec<OperationResult> {
        let total = photos.len();
        let progress_counter = AtomicUsize::new(0);

        let mut indexed: Vec<(usize, OperationResult)> = photos
            .par_iter()
            .enumerate()
            .filter_map(|(index, photo)| {
                if let Some(flag) = cancel_flag {
                    if flag.load(Ordering::Relaxed) {
                        return None;
                    }
                }

                let output_path = Self::output_path(photo, output_mode);
                let operation = MetadataEngine::apply_preset(&photo.path, preset, &output_path);

                let result = match operation {
                    Ok(_) => OperationResult::success(photo.id, output_path),
                    Err(err) => OperationResult::failure(photo.id, output_path, err.to_string()),
                };

                let current = progress_counter.fetch_add(1, Ordering::Relaxed) + 1;
                let _ = progress_tx.send(ProgressEvent {
                    current,
                    total,
                    filename: photo.filename.clone(),
                    success: result.success,
                });

                Some((index, result))
            })
            .collect();

        indexed.sort_by_key(|(index, _)| *index);
        indexed.into_iter().map(|(_, result)| result).collect()
    }

    pub fn output_path(photo: &PhotoEntry, output_mode: &OutputMode) -> PathBuf {
        match output_mode {
            OutputMode::Overwrite => photo.path.clone(),
            OutputMode::ExportTo(dir) => dir.join(&photo.filename),
            OutputMode::Suffix(suffix) => Self::add_suffix(&photo.path, suffix),
        }
    }

    pub fn add_suffix(path: &Path, suffix: &str) -> PathBuf {
        let stem = path
            .file_stem()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| String::from("output"));

        let ext = path
            .extension()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default();

        let file_name = if ext.is_empty() {
            format!("{stem}{suffix}")
        } else {
            format!("{stem}{suffix}.{ext}")
        };

        path.with_file_name(file_name)
    }
}
