use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{atomic::AtomicBool, mpsc};

use crate::core::bulk::BulkProcessor;
use crate::core::formats;
use crate::core::metadata::{MetadataEngine, MetadataError};
use crate::core::presets::builtin_presets;
use crate::models::{
    MetadataTag, OperationResult, OperationSummary, OutputMode, PhotoEntry, PresetId,
    ProgressEvent, StripPreset, TagCategory, TagValue,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewMode {
    Grid,
    Table,
}

impl ViewMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Grid => "Grid",
            Self::Table => "Table",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Panel {
    Import,
    Photos,
    Bulk,
    Presets,
    Settings,
}

impl Panel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Import => "Import",
            Self::Photos => "Photos",
            Self::Bulk => "Bulk",
            Self::Presets => "Presets",
            Self::Settings => "Settings",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetadataTab {
    Exif,
    Iptc,
    Xmp,
    All,
}

impl MetadataTab {
    pub fn label(self) -> &'static str {
        match self {
            Self::Exif => "EXIF",
            Self::Iptc => "IPTC",
            Self::Xmp => "XMP",
            Self::All => "All",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableColumn {
    Filename,
    DateTaken,
    Camera,
    Gps,
    TagCount,
    FileSize,
}

impl TableColumn {
    pub fn label(self) -> &'static str {
        match self {
            Self::Filename => "Filename",
            Self::DateTaken => "Date Taken",
            Self::Camera => "Camera",
            Self::Gps => "GPS",
            Self::TagCount => "Tags",
            Self::FileSize => "File Size",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TableSort {
    pub column: TableColumn,
    pub descending: bool,
}

impl Default for TableSort {
    fn default() -> Self {
        Self {
            column: TableColumn::Filename,
            descending: false,
        }
    }
}

#[derive(Debug)]
pub enum AppError {
    InvalidPhotoIndex(usize),
    PresetNotFound(PresetId),
    NoSelection,
    Metadata(MetadataError),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPhotoIndex(index) => write!(f, "invalid photo index: {index}"),
            Self::PresetNotFound(preset_id) => write!(f, "preset not found: {preset_id}"),
            Self::NoSelection => write!(f, "no photos selected"),
            Self::Metadata(err) => write!(f, "metadata error: {err}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<MetadataError> for AppError {
    fn from(value: MetadataError) -> Self {
        Self::Metadata(value)
    }
}

#[derive(Clone, Debug)]
struct UndoEntry {
    index: usize,
    metadata: crate::models::PhotoMetadata,
    persisted_metadata: crate::models::PhotoMetadata,
    dirty: bool,
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub photos: Vec<PhotoEntry>,
    pub selected_indices: HashSet<usize>,
    pub active_photo: Option<usize>,
    pub view_mode: ViewMode,
    pub active_panel: Panel,
    pub presets: Vec<StripPreset>,
    pub search_query: String,
    pub tag_filter: Option<TagCategory>,
    pub metadata_search_query: String,
    pub metadata_tab: MetadataTab,
    pub table_sort: TableSort,
    pub bulk_output_mode: OutputMode,
    pub active_preset: Option<PresetId>,
    pub is_processing: bool,
    pub progress: Option<ProgressEvent>,
    pub operation_results: Vec<OperationResult>,
    pub last_summary: Option<OperationSummary>,
    undo_stack: Vec<UndoEntry>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            photos: Vec::new(),
            selected_indices: HashSet::new(),
            active_photo: None,
            view_mode: ViewMode::Grid,
            active_panel: Panel::Import,
            presets: builtin_presets(),
            search_query: String::new(),
            tag_filter: None,
            metadata_search_query: String::new(),
            metadata_tab: MetadataTab::All,
            table_sort: TableSort::default(),
            bulk_output_mode: OutputMode::Overwrite,
            active_preset: None,
            is_processing: false,
            progress: None,
            operation_results: Vec::new(),
            last_summary: None,
            undo_stack: Vec::new(),
        }
    }
}

impl AppState {
    pub fn import_paths<I, P>(&mut self, paths: I) -> Vec<PathBuf>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut skipped = Vec::new();
        let mut next_id = self
            .photos
            .iter()
            .map(|photo| photo.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);

        for candidate in paths {
            let path = candidate.as_ref();

            if !path.is_file() {
                skipped.push(path.to_path_buf());
                continue;
            }

            if self.photos.iter().any(|photo| photo.path == path) {
                continue;
            }

            let format = formats::detect_format(path);
            if format.is_unknown() {
                skipped.push(path.to_path_buf());
                continue;
            }

            let mut entry = PhotoEntry::from_path(next_id, path.to_path_buf(), format);
            if let Ok(metadata) = MetadataEngine::read(path) {
                entry.set_loaded_metadata(metadata);
            }

            self.photos.push(entry);
            next_id += 1;
        }

        if self.active_photo.is_none() && !self.photos.is_empty() {
            self.active_photo = Some(0);
            self.active_panel = Panel::Photos;
        }

        skipped
    }

    pub fn set_view_mode(&mut self, view_mode: ViewMode) {
        self.view_mode = view_mode;
    }

    pub fn set_search_query(&mut self, query: impl Into<String>) {
        self.search_query = query.into();
    }

    pub fn set_metadata_search_query(&mut self, query: impl Into<String>) {
        self.metadata_search_query = query.into();
    }

    pub fn set_tag_filter(&mut self, category: Option<TagCategory>) {
        self.tag_filter = category;
    }

    pub fn set_metadata_tab(&mut self, tab: MetadataTab) {
        self.metadata_tab = tab;
    }

    pub fn set_table_sort(&mut self, sort: TableSort) {
        self.table_sort = sort;
    }

    pub fn select_photo(&mut self, index: usize, additive: bool) {
        if index >= self.photos.len() {
            return;
        }

        if !additive {
            self.selected_indices.clear();
        }

        self.selected_indices.insert(index);
        self.active_photo = Some(index);
        self.active_panel = Panel::Photos;

        self.sync_selection_flags();
    }

    pub fn toggle_photo_selection(&mut self, index: usize) {
        if index >= self.photos.len() {
            return;
        }

        if self.selected_indices.contains(&index) {
            self.selected_indices.remove(&index);
        } else {
            self.selected_indices.insert(index);
        }

        self.active_photo = Some(index);
        self.sync_selection_flags();
    }

    pub fn select_range(&mut self, start: usize, end: usize) {
        if self.photos.is_empty() {
            return;
        }

        let low = start.min(end);
        let high = start.max(end).min(self.photos.len().saturating_sub(1));

        for index in low..=high {
            self.selected_indices.insert(index);
        }

        self.active_photo = Some(high);
        self.sync_selection_flags();
    }

    pub fn select_all_visible(&mut self) {
        for index in self.sorted_visible_indices() {
            self.selected_indices.insert(index);
        }
        self.sync_selection_flags();
    }

    pub fn clear_selection(&mut self) {
        self.selected_indices.clear();
        self.sync_selection_flags();
    }

    pub fn selected_photos(&self) -> Vec<&PhotoEntry> {
        self.selected_indices_sorted()
            .into_iter()
            .filter_map(|index| self.photos.get(index))
            .collect()
    }

    pub fn visible_photos(&self) -> Vec<&PhotoEntry> {
        self.sorted_visible_indices()
            .into_iter()
            .filter_map(|index| self.photos.get(index))
            .collect()
    }

    pub fn visible_photo_indices(&self) -> Vec<usize> {
        let query = self.search_query.trim().to_ascii_lowercase();

        self.photos
            .iter()
            .enumerate()
            .filter(|(_, photo)| {
                if !query.is_empty() && !photo_matches_query(photo, &query) {
                    return false;
                }

                if let Some(filter) = self.tag_filter {
                    return photo.metadata.all_tags().any(|tag| tag.category == filter);
                }

                true
            })
            .map(|(index, _)| index)
            .collect()
    }

    pub fn sorted_visible_indices(&self) -> Vec<usize> {
        let mut indices = self.visible_photo_indices();
        indices.sort_by(|left, right| {
            let left_photo = &self.photos[*left];
            let right_photo = &self.photos[*right];

            let order = match self.table_sort.column {
                TableColumn::Filename => left_photo
                    .filename
                    .to_ascii_lowercase()
                    .cmp(&right_photo.filename.to_ascii_lowercase()),
                TableColumn::DateTaken => left_photo
                    .metadata
                    .date_taken
                    .as_deref()
                    .unwrap_or("")
                    .cmp(right_photo.metadata.date_taken.as_deref().unwrap_or("")),
                TableColumn::Camera => camera_label(left_photo)
                    .to_ascii_lowercase()
                    .cmp(&camera_label(right_photo).to_ascii_lowercase()),
                TableColumn::Gps => left_photo
                    .metadata
                    .has_gps
                    .cmp(&right_photo.metadata.has_gps),
                TableColumn::TagCount => left_photo
                    .metadata
                    .total_tag_count()
                    .cmp(&right_photo.metadata.total_tag_count()),
                TableColumn::FileSize => left_photo.file_size.cmp(&right_photo.file_size),
            };

            let adjusted = if self.table_sort.descending {
                order.reverse()
            } else {
                order
            };

            if adjusted == Ordering::Equal {
                left_photo
                    .filename
                    .to_ascii_lowercase()
                    .cmp(&right_photo.filename.to_ascii_lowercase())
            } else {
                adjusted
            }
        });

        indices
    }

    pub fn inspector_tags(&self, photo_index: usize) -> Vec<MetadataTag> {
        let Some(photo) = self.photos.get(photo_index) else {
            return Vec::new();
        };

        let mut tags = Vec::new();

        match self.metadata_tab {
            MetadataTab::Exif => tags.extend(photo.metadata.exif_tags.iter().cloned()),
            MetadataTab::Iptc => tags.extend(photo.metadata.iptc_tags.iter().cloned()),
            MetadataTab::Xmp => tags.extend(photo.metadata.xmp_tags.iter().cloned()),
            MetadataTab::All => tags.extend(photo.metadata.all_tags().cloned()),
        }

        let query = self.metadata_search_query.trim().to_ascii_lowercase();
        let filter = self.tag_filter;

        tags.retain(|tag| {
            if !query.is_empty() {
                let key = tag.key.to_ascii_lowercase();
                let name = tag.display_name.to_ascii_lowercase();
                let value = tag.value.to_string().to_ascii_lowercase();
                if !key.contains(&query) && !name.contains(&query) && !value.contains(&query) {
                    return false;
                }
            }

            if let Some(category) = filter {
                return tag.category == category;
            }

            true
        });

        tags.sort_by(|left, right| {
            left.display_name
                .to_ascii_lowercase()
                .cmp(&right.display_name.to_ascii_lowercase())
                .then_with(|| {
                    left.key
                        .to_ascii_lowercase()
                        .cmp(&right.key.to_ascii_lowercase())
                })
        });

        tags
    }

    pub fn edit_tag(
        &mut self,
        photo_index: usize,
        tag_key: &str,
        value: TagValue,
    ) -> Result<(), AppError> {
        self.push_undo_snapshot(photo_index)?;

        let photo = self
            .photos
            .get_mut(photo_index)
            .ok_or(AppError::InvalidPhotoIndex(photo_index))?;

        MetadataEngine::set_tag_in_metadata(&mut photo.metadata, tag_key, value);
        photo.recompute_dirty();
        Ok(())
    }

    pub fn clear_tag(&mut self, photo_index: usize, tag_key: &str) -> Result<bool, AppError> {
        self.push_undo_snapshot(photo_index)?;

        let photo = self
            .photos
            .get_mut(photo_index)
            .ok_or(AppError::InvalidPhotoIndex(photo_index))?;

        let removed =
            MetadataEngine::remove_tags_by_key(&mut photo.metadata, &[String::from(tag_key)]);
        photo.recompute_dirty();
        Ok(removed > 0)
    }

    pub fn mark_tag_for_removal(
        &mut self,
        photo_index: usize,
        tag_key: &str,
        marked: bool,
    ) -> Result<bool, AppError> {
        self.push_undo_snapshot(photo_index)?;

        let photo = self
            .photos
            .get_mut(photo_index)
            .ok_or(AppError::InvalidPhotoIndex(photo_index))?;

        let Some(tag) = photo.metadata.find_tag_mut(tag_key) else {
            return Ok(false);
        };

        tag.marked_for_removal = marked;
        photo.recompute_dirty();
        Ok(true)
    }

    pub fn strip_marked_tags(&mut self, photo_index: usize) -> Result<usize, AppError> {
        self.push_undo_snapshot(photo_index)?;

        let photo = self
            .photos
            .get_mut(photo_index)
            .ok_or(AppError::InvalidPhotoIndex(photo_index))?;

        let removed = MetadataEngine::remove_marked_tags(&mut photo.metadata);
        photo.recompute_dirty();
        Ok(removed)
    }

    pub fn apply_preset_to_photo(
        &mut self,
        photo_index: usize,
        preset_id: PresetId,
    ) -> Result<(), AppError> {
        let preset = self
            .preset_by_id(preset_id)
            .cloned()
            .ok_or(AppError::PresetNotFound(preset_id))?;

        self.push_undo_snapshot(photo_index)?;

        let photo = self
            .photos
            .get_mut(photo_index)
            .ok_or(AppError::InvalidPhotoIndex(photo_index))?;

        MetadataEngine::apply_preset_to_metadata(&mut photo.metadata, &preset);
        photo.recompute_dirty();
        self.active_preset = Some(preset_id);
        Ok(())
    }

    pub fn save_photo_changes(&mut self, photo_index: usize) -> Result<(), AppError> {
        let photo = self
            .photos
            .get_mut(photo_index)
            .ok_or(AppError::InvalidPhotoIndex(photo_index))?;

        MetadataEngine::write(&photo.path, &photo.metadata)?;
        photo.persisted_metadata = photo.metadata.clone();
        photo.dirty = false;
        Ok(())
    }

    pub fn save_all_dirty(&mut self) -> Result<usize, AppError> {
        let dirty_indices = self
            .photos
            .iter()
            .enumerate()
            .filter_map(|(index, photo)| if photo.dirty { Some(index) } else { None })
            .collect::<Vec<_>>();

        for index in &dirty_indices {
            self.save_photo_changes(*index)?;
        }

        Ok(dirty_indices.len())
    }

    pub fn clear_all_metadata(&mut self) -> usize {
        let count = self.photos.len();
        for photo in &mut self.photos {
            photo.metadata.exif_tags.clear();
            photo.metadata.iptc_tags.clear();
            photo.metadata.xmp_tags.clear();
            photo.metadata.update_summary_fields();
            photo.recompute_dirty();
        }
        count
    }

    pub fn revert_photo(&mut self, photo_index: usize) -> Result<(), AppError> {
        self.push_undo_snapshot(photo_index)?;

        let photo = self
            .photos
            .get_mut(photo_index)
            .ok_or(AppError::InvalidPhotoIndex(photo_index))?;

        photo.metadata = photo.persisted_metadata.clone();
        photo.dirty = false;
        Ok(())
    }

    pub fn undo_last_change(&mut self) -> bool {
        let Some(entry) = self.undo_stack.pop() else {
            return false;
        };

        let Some(photo) = self.photos.get_mut(entry.index) else {
            return false;
        };

        photo.metadata = entry.metadata;
        photo.persisted_metadata = entry.persisted_metadata;
        photo.dirty = entry.dirty;
        true
    }

    pub fn reload_photo_from_disk(&mut self, photo_index: usize) -> Result<(), AppError> {
        let photo = self
            .photos
            .get_mut(photo_index)
            .ok_or(AppError::InvalidPhotoIndex(photo_index))?;

        let metadata = MetadataEngine::read(&photo.path)?;
        photo.set_loaded_metadata(metadata);
        Ok(())
    }

    pub fn run_bulk_selected(
        &mut self,
        preset_id: PresetId,
        output_mode: OutputMode,
        cancel_flag: Option<&AtomicBool>,
    ) -> Result<OperationSummary, AppError> {
        let preset = self
            .preset_by_id(preset_id)
            .cloned()
            .ok_or(AppError::PresetNotFound(preset_id))?;

        let selected_indices = self.selected_indices_sorted();
        if selected_indices.is_empty() {
            return Err(AppError::NoSelection);
        }

        let selected_photos = selected_indices
            .iter()
            .filter_map(|&index| self.photos.get(index).cloned())
            .collect::<Vec<_>>();

        let (progress_tx, progress_rx) = mpsc::channel();

        self.is_processing = true;
        let results = BulkProcessor::process_with_cancel(
            &selected_photos,
            &preset,
            &output_mode,
            progress_tx,
            cancel_flag,
        );

        for event in progress_rx.try_iter() {
            self.progress = Some(event);
        }

        self.is_processing = false;
        self.operation_results = results.clone();
        let summary = OperationSummary::from_results(selected_photos.len(), &results);
        self.last_summary = Some(summary.clone());
        self.bulk_output_mode = output_mode.clone();
        self.active_preset = Some(preset_id);

        match output_mode {
            OutputMode::Overwrite => {
                for index in selected_indices {
                    let _ = self.reload_photo_from_disk(index);
                }
            }
            OutputMode::ExportTo(_) | OutputMode::Suffix(_) => {
                let output_paths = results
                    .iter()
                    .filter(|result| result.success)
                    .map(|result| result.output_path.clone())
                    .collect::<Vec<_>>();
                self.import_paths(output_paths);
            }
        }

        Ok(summary)
    }

    fn push_undo_snapshot(&mut self, photo_index: usize) -> Result<(), AppError> {
        let photo = self
            .photos
            .get(photo_index)
            .ok_or(AppError::InvalidPhotoIndex(photo_index))?;

        self.undo_stack.push(UndoEntry {
            index: photo_index,
            metadata: photo.metadata.clone(),
            persisted_metadata: photo.persisted_metadata.clone(),
            dirty: photo.dirty,
        });

        Ok(())
    }

    fn selected_indices_sorted(&self) -> Vec<usize> {
        let mut indices = self.selected_indices.iter().copied().collect::<Vec<_>>();
        indices.sort_unstable();
        indices
    }

    fn preset_by_id(&self, preset_id: PresetId) -> Option<&StripPreset> {
        self.presets.iter().find(|preset| preset.id == preset_id)
    }

    fn sync_selection_flags(&mut self) {
        for (index, photo) in self.photos.iter_mut().enumerate() {
            photo.selected = self.selected_indices.contains(&index);
        }
    }
}

#[derive(Default)]
pub struct AppView {
    pub state: AppState,
}

impl AppView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render(&self) -> String {
        format!(
            "panel={} view={} photos={} selected={} dirty={} processing={}",
            self.state.active_panel.label(),
            self.state.view_mode.label(),
            self.state.photos.len(),
            self.state.selected_indices.len(),
            self.state.photos.iter().filter(|photo| photo.dirty).count(),
            self.state.is_processing,
        )
    }
}

fn photo_matches_query(photo: &PhotoEntry, query: &str) -> bool {
    if photo.filename.to_ascii_lowercase().contains(query) {
        return true;
    }

    if let Some(make) = &photo.metadata.camera_make {
        if make.to_ascii_lowercase().contains(query) {
            return true;
        }
    }

    if let Some(model) = &photo.metadata.camera_model {
        if model.to_ascii_lowercase().contains(query) {
            return true;
        }
    }

    photo.metadata.all_tags().any(|tag| {
        tag.key.to_ascii_lowercase().contains(query)
            || tag.display_name.to_ascii_lowercase().contains(query)
            || tag.value.to_string().to_ascii_lowercase().contains(query)
    })
}

fn camera_label(photo: &PhotoEntry) -> String {
    match (&photo.metadata.camera_make, &photo.metadata.camera_model) {
        (Some(make), Some(model)) => format!("{make} {model}"),
        (Some(make), None) => make.clone(),
        (None, Some(model)) => model.clone(),
        (None, None) => String::new(),
    }
}
