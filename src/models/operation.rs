use std::path::PathBuf;

use crate::models::{PhotoId, PresetId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutputMode {
    Overwrite,
    ExportTo(PathBuf),
    Suffix(String),
}

impl OutputMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Overwrite => "Overwrite",
            Self::ExportTo(_) => "ExportTo",
            Self::Suffix(_) => "Suffix",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Operation {
    pub photo_id: PhotoId,
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub preset_id: PresetId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationResult {
    pub photo_id: PhotoId,
    pub output_path: PathBuf,
    pub success: bool,
    pub error: Option<String>,
}

impl OperationResult {
    pub fn success(photo_id: PhotoId, output_path: PathBuf) -> Self {
        Self {
            photo_id,
            output_path,
            success: true,
            error: None,
        }
    }

    pub fn failure(photo_id: PhotoId, output_path: PathBuf, error: impl Into<String>) -> Self {
        Self {
            photo_id,
            output_path,
            success: false,
            error: Some(error.into()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressEvent {
    pub current: usize,
    pub total: usize,
    pub filename: String,
    pub success: bool,
}

#[derive(Clone, Debug)]
pub struct BatchJob {
    pub id: u64,
    pub photo_ids: Vec<PhotoId>,
    pub preset_id: PresetId,
    pub output_mode: OutputMode,
    pub completed: usize,
    pub failed: usize,
}

impl BatchJob {
    pub fn new(
        id: u64,
        photo_ids: Vec<PhotoId>,
        preset_id: PresetId,
        output_mode: OutputMode,
    ) -> Self {
        Self {
            id,
            photo_ids,
            preset_id,
            output_mode,
            completed: 0,
            failed: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationSummary {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub cancelled: usize,
}

impl OperationSummary {
    pub fn from_results(expected_total: usize, results: &[OperationResult]) -> Self {
        let succeeded = results.iter().filter(|result| result.success).count();
        let failed = results.len().saturating_sub(succeeded);
        let cancelled = expected_total.saturating_sub(results.len());

        Self {
            total: expected_total,
            succeeded,
            failed,
            cancelled,
        }
    }
}
