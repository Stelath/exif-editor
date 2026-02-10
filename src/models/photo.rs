use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::models::{MetadataTag, TagValue};

pub type PhotoId = u64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Tiff,
    WebP,
    Heif,
    Avif,
    Jxl,
    Unknown,
}

impl ImageFormat {
    pub fn is_unknown(self) -> bool {
        self == Self::Unknown
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Jpeg => "JPEG",
            Self::Png => "PNG",
            Self::Tiff => "TIFF",
            Self::WebP => "WebP",
            Self::Heif => "HEIF",
            Self::Avif => "AVIF",
            Self::Jxl => "JXL",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ThumbnailData {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct PhotoEntry {
    pub id: PhotoId,
    pub path: PathBuf,
    pub filename: String,
    pub file_size: u64,
    pub format: ImageFormat,
    pub dimensions: Option<Dimensions>,
    pub thumbnail: Option<ThumbnailData>,
    pub metadata: PhotoMetadata,
    pub persisted_metadata: PhotoMetadata,
    pub selected: bool,
    pub dirty: bool,
}

impl PhotoEntry {
    pub fn from_path(id: PhotoId, path: PathBuf, format: ImageFormat) -> Self {
        let filename = path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| String::from("unknown"));

        let file_size = fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);

        Self {
            id,
            path,
            filename,
            file_size,
            format,
            dimensions: None,
            thumbnail: None,
            metadata: PhotoMetadata::default(),
            persisted_metadata: PhotoMetadata::default(),
            selected: false,
            dirty: false,
        }
    }

    pub fn set_loaded_metadata(&mut self, metadata: PhotoMetadata) {
        self.persisted_metadata = metadata.clone();
        self.metadata = metadata;
        self.dirty = false;
    }

    pub fn recompute_dirty(&mut self) {
        self.dirty = self.metadata != self.persisted_metadata;
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PhotoMetadata {
    pub exif_tags: Vec<MetadataTag>,
    pub iptc_tags: Vec<MetadataTag>,
    pub xmp_tags: Vec<MetadataTag>,
    pub has_gps: bool,
    pub date_taken: Option<String>,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
}

impl PhotoMetadata {
    pub fn all_tags(&self) -> impl Iterator<Item = &MetadataTag> {
        self.exif_tags
            .iter()
            .chain(self.iptc_tags.iter())
            .chain(self.xmp_tags.iter())
    }

    pub fn total_tag_count(&self) -> usize {
        self.exif_tags.len() + self.iptc_tags.len() + self.xmp_tags.len()
    }

    pub fn find_tag_mut(&mut self, key: &str) -> Option<&mut MetadataTag> {
        if let Some(tag) = self
            .exif_tags
            .iter_mut()
            .find(|tag| tag.key.eq_ignore_ascii_case(key))
        {
            return Some(tag);
        }

        if let Some(tag) = self
            .iptc_tags
            .iter_mut()
            .find(|tag| tag.key.eq_ignore_ascii_case(key))
        {
            return Some(tag);
        }

        self.xmp_tags
            .iter_mut()
            .find(|tag| tag.key.eq_ignore_ascii_case(key))
    }

    pub fn update_summary_fields(&mut self) {
        let mut has_gps = false;
        let mut date_taken = None;
        let mut camera_make = None;
        let mut camera_model = None;

        for tag in self.all_tags() {
            let key = tag.key.to_ascii_lowercase();

            if !has_gps && (matches!(tag.value, TagValue::Gps(_, _, _)) || key.contains("gps")) {
                has_gps = true;
            }

            if date_taken.is_none() && key.contains("datetimeoriginal") {
                date_taken = Some(tag.value.to_string());
            }

            if camera_make.is_none() && key.ends_with("make") {
                camera_make = Some(tag.value.to_string());
            }

            if camera_model.is_none() && key.ends_with("model") {
                camera_model = Some(tag.value.to_string());
            }
        }

        self.has_gps = has_gps;
        self.date_taken = date_taken;
        self.camera_make = camera_make;
        self.camera_model = camera_model;
    }
}
