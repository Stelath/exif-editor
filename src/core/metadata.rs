use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use little_exif::exif_tag::ExifTag;
use little_exif::metadata::Metadata as ExifMetadata;

use crate::models::{MetadataTag, PhotoMetadata, PresetRule, StripPreset, TagCategory, TagValue};

#[derive(Debug)]
pub enum MetadataError {
    FileNotFound(PathBuf),
    InvalidTagKey(String),
    Io(std::io::Error),
    Serialization(serde_json::Error),
}

impl fmt::Display for MetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileNotFound(path) => write!(f, "file not found: {}", path.display()),
            Self::InvalidTagKey(key) => write!(f, "invalid metadata tag key: {key}"),
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Serialization(err) => write!(f, "metadata serialization error: {err}"),
        }
    }
}

impl std::error::Error for MetadataError {}

impl From<std::io::Error> for MetadataError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for MetadataError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialization(value)
    }
}

pub type Result<T> = std::result::Result<T, MetadataError>;

pub struct MetadataEngine;

impl MetadataEngine {
    pub fn read(path: &Path) -> Result<PhotoMetadata> {
        if !path.exists() {
            return Err(MetadataError::FileNotFound(path.to_path_buf()));
        }

        let sidecar = Self::sidecar_path(path);
        if sidecar.exists() {
            let contents = fs::read_to_string(&sidecar)?;
            let mut metadata: PhotoMetadata = serde_json::from_str(&contents)?;
            metadata.update_summary_fields();
            return Ok(metadata);
        }

        match Self::read_exif_from_file(path) {
            Some(metadata) if !metadata.exif_tags.is_empty() => Ok(metadata),
            _ => Self::default_metadata_for_path(path),
        }
    }

    pub fn write(path: &Path, metadata: &PhotoMetadata) -> Result<()> {
        if !path.exists() {
            return Err(MetadataError::FileNotFound(path.to_path_buf()));
        }

        Self::write_exif_to_file(path, metadata);

        let sidecar = Self::sidecar_path(path);
        if let Some(parent) = sidecar.parent() {
            fs::create_dir_all(parent)?;
        }

        let encoded = serde_json::to_string_pretty(metadata)?;
        fs::write(sidecar, encoded)?;
        Ok(())
    }

    pub fn apply_preset(path: &Path, preset: &StripPreset, output: &Path) -> Result<PhotoMetadata> {
        if !path.exists() {
            return Err(MetadataError::FileNotFound(path.to_path_buf()));
        }

        if path != output {
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, output)?;
        }

        let mut metadata = Self::read(path)?;
        Self::apply_preset_to_metadata(&mut metadata, preset);
        Self::write(output, &metadata)?;
        Ok(metadata)
    }

    pub fn set_tag(path: &Path, tag_key: &str, value: &TagValue) -> Result<PhotoMetadata> {
        let key = tag_key.trim();
        if key.is_empty() {
            return Err(MetadataError::InvalidTagKey(String::from(tag_key)));
        }

        let mut metadata = Self::read(path)?;
        Self::set_tag_in_metadata(&mut metadata, key, value.clone());
        Self::write(path, &metadata)?;
        Ok(metadata)
    }

    pub fn sidecar_path(path: &Path) -> PathBuf {
        let base_name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| String::from("photo"));

        path.with_file_name(format!("{base_name}.metastrip.json"))
    }

    pub fn apply_preset_to_metadata(metadata: &mut PhotoMetadata, preset: &StripPreset) {
        for rule in &preset.rules {
            Self::apply_rule(metadata, rule);
        }
        metadata.update_summary_fields();
    }

    pub fn set_tag_in_metadata(metadata: &mut PhotoMetadata, tag_key: &str, value: TagValue) {
        if Self::update_existing_tag(&mut metadata.exif_tags, tag_key, &value)
            || Self::update_existing_tag(&mut metadata.iptc_tags, tag_key, &value)
            || Self::update_existing_tag(&mut metadata.xmp_tags, tag_key, &value)
        {
            metadata.update_summary_fields();
            return;
        }

        let tag = MetadataTag {
            key: tag_key.to_string(),
            display_name: display_name_from_key(tag_key),
            value,
            category: infer_category_from_key(tag_key),
            editable: true,
            marked_for_removal: false,
        };

        metadata.exif_tags.push(tag);
        metadata.update_summary_fields();
    }

    pub fn remove_marked_tags(metadata: &mut PhotoMetadata) -> usize {
        let before = metadata.total_tag_count();
        Self::retain_all(metadata, |tag| !tag.marked_for_removal);
        metadata.update_summary_fields();
        before.saturating_sub(metadata.total_tag_count())
    }

    pub fn remove_tags_by_key(metadata: &mut PhotoMetadata, tag_keys: &[String]) -> usize {
        if tag_keys.is_empty() {
            return 0;
        }

        let normalized: HashSet<String> = tag_keys
            .iter()
            .map(|key| key.trim().to_ascii_lowercase())
            .filter(|key| !key.is_empty())
            .collect();

        if normalized.is_empty() {
            return 0;
        }

        let before = metadata.total_tag_count();
        Self::retain_all(metadata, |tag| {
            !normalized.contains(&tag.key.to_ascii_lowercase())
        });
        metadata.update_summary_fields();
        before.saturating_sub(metadata.total_tag_count())
    }

    fn read_exif_from_file(path: &Path) -> Option<PhotoMetadata> {
        let exif = ExifMetadata::new_from_path(path).ok()?;

        // Collect all tags from the metadata iterator
        let tags: Vec<&ExifTag> = (&exif).into_iter().collect();

        if tags.is_empty() {
            return None;
        }

        let mut exif_tags = Vec::new();
        let mut gps_lat_ref: Option<String> = None;
        let mut gps_lat_dms: Option<(f64, f64, f64)> = None;
        let mut gps_lon_ref: Option<String> = None;
        let mut gps_lon_dms: Option<(f64, f64, f64)> = None;
        let mut gps_alt_ref: Option<u8> = None;
        let mut gps_alt: Option<f64> = None;

        for tag in tags {
            let hex = tag.as_u16();

            // Collect GPS sub-IFD fields from native GPS variants
            match tag {
                ExifTag::GPSLatitudeRef(s) => {
                    gps_lat_ref = Some(s.trim_end_matches('\0').to_string());
                    continue;
                }
                ExifTag::GPSLatitude(rats) if rats.len() >= 3 => {
                    let d: f64 = rats[0].clone().into();
                    let m: f64 = rats[1].clone().into();
                    let s: f64 = rats[2].clone().into();
                    gps_lat_dms = Some((d, m, s));
                    continue;
                }
                ExifTag::GPSLongitudeRef(s) => {
                    gps_lon_ref = Some(s.trim_end_matches('\0').to_string());
                    continue;
                }
                ExifTag::GPSLongitude(rats) if rats.len() >= 3 => {
                    let d: f64 = rats[0].clone().into();
                    let m: f64 = rats[1].clone().into();
                    let s: f64 = rats[2].clone().into();
                    gps_lon_dms = Some((d, m, s));
                    continue;
                }
                ExifTag::GPSAltitudeRef(bytes) if !bytes.is_empty() => {
                    gps_alt_ref = Some(bytes[0]);
                    continue;
                }
                ExifTag::GPSAltitude(rats) if !rats.is_empty() => {
                    gps_alt = Some(rats[0].clone().into());
                    continue;
                }
                _ => {}
            }

            // Skip internal IFD offset pointers and thumbnail-related tags
            if matches!(
                tag,
                ExifTag::ExifOffset(_)
                    | ExifTag::GPSInfo(_)
                    | ExifTag::InteropOffset(_)
                    | ExifTag::ThumbnailOffset(..)
                    | ExifTag::ThumbnailLength(_)
                    | ExifTag::StripOffsets(..)
                    | ExifTag::StripByteCounts(_)
            ) {
                continue;
            }

            if let Some(converted) = convert_exif_tag(tag, hex) {
                exif_tags.push(converted);
            }
        }

        // Assemble GPS coordinate if we found lat/lon
        if let (Some(lat_dms), Some(lon_dms)) = (gps_lat_dms, gps_lon_dms) {
            let mut lat = dms_to_decimal(lat_dms.0, lat_dms.1, lat_dms.2);
            let mut lon = dms_to_decimal(lon_dms.0, lon_dms.1, lon_dms.2);

            if gps_lat_ref.as_deref() == Some("S") {
                lat = -lat;
            }
            if gps_lon_ref.as_deref() == Some("W") {
                lon = -lon;
            }

            let altitude = gps_alt.map(|a| if gps_alt_ref == Some(1) { -a } else { a });

            exif_tags.push(MetadataTag::new(
                "Exif.GPSInfo.GPSCoordinates",
                "GPS Coordinates",
                TagValue::Gps(lat, lon, altitude),
                TagCategory::Location,
            ));
        }

        let mut metadata = PhotoMetadata {
            exif_tags,
            iptc_tags: Vec::new(),
            xmp_tags: Vec::new(),
            has_gps: false,
            date_taken: None,
            camera_make: None,
            camera_model: None,
        };

        metadata.update_summary_fields();
        Some(metadata)
    }

    fn write_exif_to_file(path: &Path, metadata: &PhotoMetadata) {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();

        if !matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp" | "heic" | "heif") {
            return;
        }

        let mut exif = match ExifMetadata::new_from_path(path) {
            Ok(e) => e,
            Err(_) => ExifMetadata::new(),
        };

        for tag in metadata.all_tags() {
            if let Some(exif_tag) = metadata_tag_to_exif(tag) {
                exif.set_tag(exif_tag);
            }

            // Write GPS as individual EXIF fields
            if let TagValue::Gps(lat, lon, alt) = &tag.value {
                write_gps_tags(&mut exif, *lat, *lon, alt);
            }
        }

        let _ = exif.write_to_file(path);
    }

    fn default_metadata_for_path(path: &Path) -> Result<PhotoMetadata> {
        let file_meta = fs::metadata(path)?;
        let mut exif_tags = Vec::new();

        exif_tags.push(MetadataTag::new(
            "MetaStrip.FileName",
            "File Name",
            TagValue::Text(
                path.file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| String::from("unknown")),
            ),
            TagCategory::Image,
        ));

        exif_tags.push(MetadataTag::new(
            "MetaStrip.FileSize",
            "File Size",
            TagValue::Integer(file_meta.len() as i64),
            TagCategory::Image,
        ));

        if let Ok(modified) = file_meta.modified() {
            if let Ok(elapsed) = modified.duration_since(UNIX_EPOCH) {
                exif_tags.push(MetadataTag::new(
                    "Exif.Photo.DateTimeOriginal",
                    "Date Taken",
                    TagValue::DateTime(elapsed.as_secs().to_string()),
                    TagCategory::DateTime,
                ));
            }
        }

        let mut metadata = PhotoMetadata {
            exif_tags,
            iptc_tags: Vec::new(),
            xmp_tags: Vec::new(),
            has_gps: false,
            date_taken: None,
            camera_make: None,
            camera_model: None,
        };

        metadata.update_summary_fields();
        Ok(metadata)
    }

    fn apply_rule(metadata: &mut PhotoMetadata, rule: &PresetRule) {
        match rule {
            PresetRule::RemoveCategory(category) => Self::remove_category(metadata, *category),
            PresetRule::RemoveTag(tag_key) => Self::remove_tag(metadata, tag_key),
            PresetRule::RemoveAllExcept(allowed_keys) => {
                Self::remove_all_except(metadata, allowed_keys)
            }
            PresetRule::RemoveAll => Self::clear_all(metadata),
            PresetRule::RemoveGps => Self::remove_gps(metadata),
            PresetRule::RemoveThumbnail => Self::remove_thumbnail(metadata),
            PresetRule::SetTag(key, value) => {
                Self::set_tag_in_metadata(metadata, key, TagValue::Text(value.clone()))
            }
        }
    }

    fn remove_category(metadata: &mut PhotoMetadata, category: TagCategory) {
        Self::retain_all(metadata, |tag| tag.category != category);
    }

    fn remove_tag(metadata: &mut PhotoMetadata, tag_key: &str) {
        let normalized = tag_key.to_ascii_lowercase();
        Self::retain_all(metadata, |tag| tag.key.to_ascii_lowercase() != normalized);
    }

    fn remove_all_except(metadata: &mut PhotoMetadata, allowed_keys: &[String]) {
        let mut allowed_set = HashSet::new();
        let mut allowed_categories = HashSet::new();

        for entry in allowed_keys {
            let normalized = entry.trim().to_ascii_lowercase();
            if let Some(category) = category_from_token(&normalized) {
                allowed_categories.insert(category);
            } else if !normalized.is_empty() {
                allowed_set.insert(normalized);
            }
        }

        Self::retain_all(metadata, |tag| {
            allowed_set.contains(&tag.key.to_ascii_lowercase())
                || allowed_categories.contains(&tag.category)
        });
    }

    fn clear_all(metadata: &mut PhotoMetadata) {
        metadata.exif_tags.clear();
        metadata.iptc_tags.clear();
        metadata.xmp_tags.clear();
        metadata.has_gps = false;
        metadata.date_taken = None;
        metadata.camera_make = None;
        metadata.camera_model = None;
    }

    fn remove_gps(metadata: &mut PhotoMetadata) {
        Self::retain_all(metadata, |tag| !is_gps_tag(tag));
        metadata.has_gps = false;
    }

    fn remove_thumbnail(metadata: &mut PhotoMetadata) {
        Self::retain_all(metadata, |tag| {
            !tag.key.to_ascii_lowercase().contains("thumbnail")
        });
    }

    fn retain_all<F>(metadata: &mut PhotoMetadata, mut predicate: F)
    where
        F: FnMut(&MetadataTag) -> bool,
    {
        metadata.exif_tags.retain(|tag| predicate(tag));
        metadata.iptc_tags.retain(|tag| predicate(tag));
        metadata.xmp_tags.retain(|tag| predicate(tag));
    }

    fn update_existing_tag(tags: &mut [MetadataTag], tag_key: &str, value: &TagValue) -> bool {
        if let Some(existing) = tags
            .iter_mut()
            .find(|tag| tag.key.eq_ignore_ascii_case(tag_key))
        {
            existing.value = value.clone();
            existing.marked_for_removal = false;
            true
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// EXIF tag conversion helpers
// ---------------------------------------------------------------------------

fn convert_exif_tag(tag: &ExifTag, _hex: u16) -> Option<MetadataTag> {
    let (key, display, value) = match tag {
        // -- String tags --
        ExifTag::Make(s) => ("Exif.Image.Make", "Make", TagValue::Text(clean_string(s))),
        ExifTag::Model(s) => ("Exif.Image.Model", "Model", TagValue::Text(clean_string(s))),
        ExifTag::Software(s) => (
            "Exif.Image.Software",
            "Software",
            TagValue::Text(clean_string(s)),
        ),
        ExifTag::Artist(s) => (
            "Exif.Image.Artist",
            "Artist",
            TagValue::Text(clean_string(s)),
        ),
        ExifTag::Copyright(s) => (
            "Exif.Image.Copyright",
            "Copyright",
            TagValue::Text(clean_string(s)),
        ),
        ExifTag::ImageDescription(s) => (
            "Exif.Image.ImageDescription",
            "Image Description",
            TagValue::Text(clean_string(s)),
        ),
        ExifTag::LensMake(s) => (
            "Exif.Photo.LensMake",
            "Lens Make",
            TagValue::Text(clean_string(s)),
        ),
        ExifTag::LensModel(s) => (
            "Exif.Photo.LensModel",
            "Lens Model",
            TagValue::Text(clean_string(s)),
        ),
        ExifTag::LensSerialNumber(s) => (
            "Exif.Photo.LensSerialNumber",
            "Lens Serial Number",
            TagValue::Text(clean_string(s)),
        ),
        ExifTag::OwnerName(s) => (
            "Exif.Photo.OwnerName",
            "Owner Name",
            TagValue::Text(clean_string(s)),
        ),
        ExifTag::SerialNumber(s) => (
            "Exif.Photo.SerialNumber",
            "Serial Number",
            TagValue::Text(clean_string(s)),
        ),

        // -- DateTime tags --
        ExifTag::DateTimeOriginal(s) => (
            "Exif.Photo.DateTimeOriginal",
            "Date Taken",
            TagValue::DateTime(clean_string(s)),
        ),
        ExifTag::CreateDate(s) => (
            "Exif.Photo.CreateDate",
            "Create Date",
            TagValue::DateTime(clean_string(s)),
        ),
        ExifTag::ModifyDate(s) => (
            "Exif.Image.ModifyDate",
            "Modify Date",
            TagValue::DateTime(clean_string(s)),
        ),
        ExifTag::OffsetTime(s) => (
            "Exif.Photo.OffsetTime",
            "Offset Time",
            TagValue::Text(clean_string(s)),
        ),
        ExifTag::OffsetTimeOriginal(s) => (
            "Exif.Photo.OffsetTimeOriginal",
            "Offset Time Original",
            TagValue::Text(clean_string(s)),
        ),
        ExifTag::OffsetTimeDigitized(s) => (
            "Exif.Photo.OffsetTimeDigitized",
            "Offset Time Digitized",
            TagValue::Text(clean_string(s)),
        ),
        ExifTag::SubSecTime(s) => (
            "Exif.Photo.SubSecTime",
            "Sub Sec Time",
            TagValue::Text(clean_string(s)),
        ),
        ExifTag::SubSecTimeOriginal(s) => (
            "Exif.Photo.SubSecTimeOriginal",
            "Sub Sec Time Original",
            TagValue::Text(clean_string(s)),
        ),
        ExifTag::SubSecTimeDigitized(s) => (
            "Exif.Photo.SubSecTimeDigitized",
            "Sub Sec Time Digitized",
            TagValue::Text(clean_string(s)),
        ),

        // -- Integer tags (u16 vecs) --
        ExifTag::Orientation(v) => (
            "Exif.Image.Orientation",
            "Orientation",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),
        ExifTag::ISO(v) => (
            "Exif.Photo.ISO",
            "ISO",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),
        ExifTag::ExposureProgram(v) => (
            "Exif.Photo.ExposureProgram",
            "Exposure Program",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),
        ExifTag::MeteringMode(v) => (
            "Exif.Photo.MeteringMode",
            "Metering Mode",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),
        ExifTag::Flash(v) => (
            "Exif.Photo.Flash",
            "Flash",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),
        ExifTag::ColorSpace(v) => (
            "Exif.Photo.ColorSpace",
            "Color Space",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),
        ExifTag::ExposureMode(v) => (
            "Exif.Photo.ExposureMode",
            "Exposure Mode",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),
        ExifTag::WhiteBalance(v) => (
            "Exif.Photo.WhiteBalance",
            "White Balance",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),
        ExifTag::SceneCaptureType(v) => (
            "Exif.Photo.SceneCaptureType",
            "Scene Capture Type",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),
        ExifTag::Contrast(v) => (
            "Exif.Photo.Contrast",
            "Contrast",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),
        ExifTag::Saturation(v) => (
            "Exif.Photo.Saturation",
            "Saturation",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),
        ExifTag::Sharpness(v) => (
            "Exif.Photo.Sharpness",
            "Sharpness",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),
        ExifTag::LightSource(v) => (
            "Exif.Photo.LightSource",
            "Light Source",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),
        ExifTag::FocalLengthIn35mmFormat(v) => (
            "Exif.Photo.FocalLengthIn35mmFormat",
            "Focal Length (35mm)",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),
        ExifTag::Compression(v) => (
            "Exif.Image.Compression",
            "Compression",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),
        ExifTag::ResolutionUnit(v) => (
            "Exif.Image.ResolutionUnit",
            "Resolution Unit",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),
        ExifTag::SensingMethod(v) => (
            "Exif.Photo.SensingMethod",
            "Sensing Method",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),
        ExifTag::CustomRendered(v) => (
            "Exif.Photo.CustomRendered",
            "Custom Rendered",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),
        ExifTag::GainControl(v) => (
            "Exif.Photo.GainControl",
            "Gain Control",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),
        ExifTag::SubjectDistanceRange(v) => (
            "Exif.Photo.SubjectDistanceRange",
            "Subject Distance Range",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),

        // -- Integer tags (u32 vecs) --
        ExifTag::ImageWidth(v) => (
            "Exif.Image.ImageWidth",
            "Image Width",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),
        ExifTag::ImageHeight(v) => (
            "Exif.Image.ImageHeight",
            "Image Height",
            TagValue::Integer(v.first().copied().unwrap_or(0) as i64),
        ),

        // -- Unsigned rational tags --
        ExifTag::ExposureTime(v) => {
            let r = v.first()?;
            (
                "Exif.Photo.ExposureTime",
                "Exposure Time",
                TagValue::Rational(r.nominator, r.denominator),
            )
        }
        ExifTag::FNumber(v) => {
            let r = v.first()?;
            (
                "Exif.Photo.FNumber",
                "F-Number",
                TagValue::Rational(r.nominator, r.denominator),
            )
        }
        ExifTag::FocalLength(v) => {
            let r = v.first()?;
            (
                "Exif.Photo.FocalLength",
                "Focal Length",
                TagValue::Rational(r.nominator, r.denominator),
            )
        }
        ExifTag::ApertureValue(v) => {
            let r = v.first()?;
            (
                "Exif.Photo.ApertureValue",
                "Aperture Value",
                TagValue::Rational(r.nominator, r.denominator),
            )
        }
        ExifTag::MaxApertureValue(v) => {
            let r = v.first()?;
            (
                "Exif.Photo.MaxApertureValue",
                "Max Aperture Value",
                TagValue::Rational(r.nominator, r.denominator),
            )
        }
        ExifTag::XResolution(v) => {
            let r = v.first()?;
            (
                "Exif.Image.XResolution",
                "X Resolution",
                TagValue::Rational(r.nominator, r.denominator),
            )
        }
        ExifTag::YResolution(v) => {
            let r = v.first()?;
            (
                "Exif.Image.YResolution",
                "Y Resolution",
                TagValue::Rational(r.nominator, r.denominator),
            )
        }
        ExifTag::SubjectDistance(v) => {
            let r = v.first()?;
            let val: f64 = if r.denominator != 0 {
                r.nominator as f64 / r.denominator as f64
            } else {
                0.0
            };
            (
                "Exif.Photo.SubjectDistance",
                "Subject Distance",
                TagValue::Float(val),
            )
        }
        ExifTag::DigitalZoomRatio(v) => {
            let r = v.first()?;
            (
                "Exif.Photo.DigitalZoomRatio",
                "Digital Zoom Ratio",
                TagValue::Rational(r.nominator, r.denominator),
            )
        }
        ExifTag::CompressedBitsPerPixel(v) => {
            let r = v.first()?;
            (
                "Exif.Photo.CompressedBitsPerPixel",
                "Compressed Bits Per Pixel",
                TagValue::Rational(r.nominator, r.denominator),
            )
        }

        // -- Signed rational tags (convert to float) --
        ExifTag::ShutterSpeedValue(v) => {
            let r = v.first()?;
            let val: f64 = if r.denominator != 0 {
                r.nominator as f64 / r.denominator as f64
            } else {
                0.0
            };
            (
                "Exif.Photo.ShutterSpeedValue",
                "Shutter Speed Value",
                TagValue::Float(val),
            )
        }
        ExifTag::BrightnessValue(v) => {
            let r = v.first()?;
            let val: f64 = if r.denominator != 0 {
                r.nominator as f64 / r.denominator as f64
            } else {
                0.0
            };
            (
                "Exif.Photo.BrightnessValue",
                "Brightness Value",
                TagValue::Float(val),
            )
        }
        ExifTag::ExposureCompensation(v) => {
            let r = v.first()?;
            let val: f64 = if r.denominator != 0 {
                r.nominator as f64 / r.denominator as f64
            } else {
                0.0
            };
            (
                "Exif.Photo.ExposureCompensation",
                "Exposure Compensation",
                TagValue::Float(val),
            )
        }

        // -- UNDEF / binary tags --
        ExifTag::MakerNote(v) => (
            "Exif.Photo.MakerNote",
            "Maker Note",
            TagValue::Binary(v.clone()),
        ),
        ExifTag::ExifVersion(v) => (
            "Exif.Photo.ExifVersion",
            "EXIF Version",
            TagValue::Text(String::from_utf8_lossy(v).to_string()),
        ),
        ExifTag::FlashpixVersion(v) => (
            "Exif.Photo.FlashpixVersion",
            "Flashpix Version",
            TagValue::Text(String::from_utf8_lossy(v).to_string()),
        ),
        ExifTag::ComponentsConfiguration(v) => (
            "Exif.Photo.ComponentsConfiguration",
            "Components Configuration",
            TagValue::Binary(v.clone()),
        ),

        // -- Lens info (multi-value rational) --
        ExifTag::LensInfo(v) if !v.is_empty() => {
            let display: String = v
                .iter()
                .map(|r| {
                    if r.denominator == 0 {
                        String::from("0")
                    } else {
                        format!("{}", r.nominator as f64 / r.denominator as f64)
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            ("Exif.Photo.LensInfo", "Lens Info", TagValue::Text(display))
        }

        // -- Unknown variants: surface as text/binary --
        ExifTag::UnknownSTRING(s, hex, _) => {
            let key_str = format!("Exif.Unknown.0x{hex:04X}");
            let display = format!("Tag 0x{hex:04X}");
            return Some(MetadataTag {
                key: key_str,
                display_name: display,
                value: TagValue::Text(clean_string(s)),
                category: TagCategory::Other,
                editable: true,
                marked_for_removal: false,
            });
        }
        ExifTag::UnknownINT16U(v, hex, _) => {
            let key_str = format!("Exif.Unknown.0x{hex:04X}");
            let display = format!("Tag 0x{hex:04X}");
            let text = v
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return Some(MetadataTag {
                key: key_str,
                display_name: display,
                value: TagValue::Text(text),
                category: TagCategory::Other,
                editable: false,
                marked_for_removal: false,
            });
        }
        ExifTag::UnknownINT32U(v, hex, _) => {
            let key_str = format!("Exif.Unknown.0x{hex:04X}");
            let display = format!("Tag 0x{hex:04X}");
            let text = v
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return Some(MetadataTag {
                key: key_str,
                display_name: display,
                value: TagValue::Text(text),
                category: TagCategory::Other,
                editable: false,
                marked_for_removal: false,
            });
        }
        ExifTag::UnknownRATIONAL64U(v, hex, _) => {
            let key_str = format!("Exif.Unknown.0x{hex:04X}");
            let display = format!("Tag 0x{hex:04X}");
            let text = v
                .iter()
                .map(|r| format!("{}/{}", r.nominator, r.denominator))
                .collect::<Vec<_>>()
                .join(", ");
            return Some(MetadataTag {
                key: key_str,
                display_name: display,
                value: TagValue::Text(text),
                category: TagCategory::Other,
                editable: false,
                marked_for_removal: false,
            });
        }
        ExifTag::UnknownRATIONAL64S(v, hex, _) => {
            let key_str = format!("Exif.Unknown.0x{hex:04X}");
            let display = format!("Tag 0x{hex:04X}");
            let text = v
                .iter()
                .map(|r| format!("{}/{}", r.nominator, r.denominator))
                .collect::<Vec<_>>()
                .join(", ");
            return Some(MetadataTag {
                key: key_str,
                display_name: display,
                value: TagValue::Text(text),
                category: TagCategory::Other,
                editable: false,
                marked_for_removal: false,
            });
        }
        ExifTag::UnknownUNDEF(v, hex, _) => {
            let key_str = format!("Exif.Unknown.0x{hex:04X}");
            let display = format!("Tag 0x{hex:04X}");
            return Some(MetadataTag {
                key: key_str,
                display_name: display,
                value: TagValue::Binary(v.clone()),
                category: TagCategory::Other,
                editable: false,
                marked_for_removal: false,
            });
        }
        ExifTag::UnknownINT8U(v, hex, _) => {
            let key_str = format!("Exif.Unknown.0x{hex:04X}");
            let display = format!("Tag 0x{hex:04X}");
            return Some(MetadataTag {
                key: key_str,
                display_name: display,
                value: TagValue::Binary(v.clone()),
                category: TagCategory::Other,
                editable: false,
                marked_for_removal: false,
            });
        }

        // Catch-all for remaining tags
        _ => return None,
    };

    Some(MetadataTag {
        key: key.to_string(),
        display_name: display.to_string(),
        value,
        category: infer_category_from_key(key),
        editable: true,
        marked_for_removal: false,
    })
}

fn metadata_tag_to_exif(tag: &MetadataTag) -> Option<ExifTag> {
    let key = tag.key.as_str();
    match (&tag.value, key) {
        // String tags
        (TagValue::Text(s), "Exif.Image.Make") => Some(ExifTag::Make(s.clone())),
        (TagValue::Text(s), "Exif.Image.Model") => Some(ExifTag::Model(s.clone())),
        (TagValue::Text(s), "Exif.Image.Software") => Some(ExifTag::Software(s.clone())),
        (TagValue::Text(s), "Exif.Image.Artist") => Some(ExifTag::Artist(s.clone())),
        (TagValue::Text(s), "Exif.Image.Copyright") => Some(ExifTag::Copyright(s.clone())),
        (TagValue::Text(s), "Exif.Image.ImageDescription") => {
            Some(ExifTag::ImageDescription(s.clone()))
        }
        (TagValue::Text(s), "Exif.Photo.LensMake") => Some(ExifTag::LensMake(s.clone())),
        (TagValue::Text(s), "Exif.Photo.LensModel") => Some(ExifTag::LensModel(s.clone())),
        (TagValue::Text(s), "Exif.Photo.LensSerialNumber") => {
            Some(ExifTag::LensSerialNumber(s.clone()))
        }
        (TagValue::Text(s), "Exif.Photo.OwnerName") => Some(ExifTag::OwnerName(s.clone())),
        (TagValue::Text(s), "Exif.Photo.SerialNumber") => Some(ExifTag::SerialNumber(s.clone())),

        // DateTime tags
        (TagValue::DateTime(s), "Exif.Photo.DateTimeOriginal") => {
            Some(ExifTag::DateTimeOriginal(s.clone()))
        }
        (TagValue::DateTime(s), "Exif.Photo.CreateDate") => Some(ExifTag::CreateDate(s.clone())),
        (TagValue::DateTime(s), "Exif.Image.ModifyDate") => Some(ExifTag::ModifyDate(s.clone())),

        // Integer tags
        (TagValue::Integer(v), "Exif.Image.Orientation") => {
            Some(ExifTag::Orientation(vec![*v as u16]))
        }
        (TagValue::Integer(v), "Exif.Photo.ISO") => Some(ExifTag::ISO(vec![*v as u16])),
        (TagValue::Integer(v), "Exif.Photo.Flash") => Some(ExifTag::Flash(vec![*v as u16])),
        (TagValue::Integer(v), "Exif.Photo.ColorSpace") => {
            Some(ExifTag::ColorSpace(vec![*v as u16]))
        }
        (TagValue::Integer(v), "Exif.Photo.ExposureProgram") => {
            Some(ExifTag::ExposureProgram(vec![*v as u16]))
        }
        (TagValue::Integer(v), "Exif.Photo.MeteringMode") => {
            Some(ExifTag::MeteringMode(vec![*v as u16]))
        }
        (TagValue::Integer(v), "Exif.Photo.WhiteBalance") => {
            Some(ExifTag::WhiteBalance(vec![*v as u16]))
        }
        (TagValue::Integer(v), "Exif.Photo.ExposureMode") => {
            Some(ExifTag::ExposureMode(vec![*v as u16]))
        }
        (TagValue::Integer(v), "Exif.Image.ImageWidth") => {
            Some(ExifTag::ImageWidth(vec![*v as u32]))
        }
        (TagValue::Integer(v), "Exif.Image.ImageHeight") => {
            Some(ExifTag::ImageHeight(vec![*v as u32]))
        }

        // Rational tags
        (TagValue::Rational(n, d), "Exif.Photo.ExposureTime") => {
            Some(ExifTag::ExposureTime(vec![ur64(*n, *d)]))
        }
        (TagValue::Rational(n, d), "Exif.Photo.FNumber") => {
            Some(ExifTag::FNumber(vec![ur64(*n, *d)]))
        }
        (TagValue::Rational(n, d), "Exif.Photo.FocalLength") => {
            Some(ExifTag::FocalLength(vec![ur64(*n, *d)]))
        }
        (TagValue::Rational(n, d), "Exif.Photo.ApertureValue") => {
            Some(ExifTag::ApertureValue(vec![ur64(*n, *d)]))
        }
        (TagValue::Rational(n, d), "Exif.Image.XResolution") => {
            Some(ExifTag::XResolution(vec![ur64(*n, *d)]))
        }
        (TagValue::Rational(n, d), "Exif.Image.YResolution") => {
            Some(ExifTag::YResolution(vec![ur64(*n, *d)]))
        }

        _ => None,
    }
}

fn write_gps_tags(exif: &mut ExifMetadata, lat: f64, lon: f64, alt: &Option<f64>) {
    let lat_ref = if lat >= 0.0 { "N" } else { "S" };
    let lon_ref = if lon >= 0.0 { "E" } else { "W" };

    let (lat_d, lat_m, lat_sn, lat_sd) = decimal_to_dms(lat.abs());
    let (lon_d, lon_m, lon_sn, lon_sd) = decimal_to_dms(lon.abs());

    exif.set_tag(ExifTag::GPSLatitudeRef(lat_ref.to_string()));
    exif.set_tag(ExifTag::GPSLatitude(vec![
        ur64(lat_d, 1),
        ur64(lat_m, 1),
        ur64(lat_sn, lat_sd),
    ]));
    exif.set_tag(ExifTag::GPSLongitudeRef(lon_ref.to_string()));
    exif.set_tag(ExifTag::GPSLongitude(vec![
        ur64(lon_d, 1),
        ur64(lon_m, 1),
        ur64(lon_sn, lon_sd),
    ]));

    if let Some(altitude) = alt {
        let alt_ref: u8 = if *altitude < 0.0 { 1 } else { 0 };
        let alt_abs = altitude.abs();
        let alt_num = (alt_abs * 100.0).round() as u32;
        exif.set_tag(ExifTag::GPSAltitudeRef(vec![alt_ref]));
        exif.set_tag(ExifTag::GPSAltitude(vec![ur64(alt_num, 100)]));
    }
}

fn ur64(nominator: u32, denominator: u32) -> little_exif::rational::uR64 {
    little_exif::rational::uR64 {
        nominator,
        denominator,
    }
}

// ---------------------------------------------------------------------------
// GPS conversion helpers
// ---------------------------------------------------------------------------

/// Convert decimal degrees to DMS (degrees, minutes, seconds_numerator, seconds_denominator).
/// Seconds are expressed as a rational with denominator 10000 for sub-second precision.
pub fn decimal_to_dms(decimal: f64) -> (u32, u32, u32, u32) {
    let d = decimal.abs();
    let degrees = d as u32;
    let minutes_full = (d - degrees as f64) * 60.0;
    let minutes = minutes_full as u32;
    let seconds = (minutes_full - minutes as f64) * 60.0;
    let seconds_num = (seconds * 10000.0).round() as u32;
    (degrees, minutes, seconds_num, 10000)
}

/// Convert DMS components to decimal degrees.
pub fn dms_to_decimal(degrees: f64, minutes: f64, seconds: f64) -> f64 {
    degrees + minutes / 60.0 + seconds / 3600.0
}

// ---------------------------------------------------------------------------
// Existing helpers
// ---------------------------------------------------------------------------

fn clean_string(s: &str) -> String {
    s.trim_end_matches('\0').trim().to_string()
}

fn is_gps_tag(tag: &MetadataTag) -> bool {
    tag.category == TagCategory::Location || tag.key.to_ascii_lowercase().contains("gps")
}

fn infer_category_from_key(tag_key: &str) -> TagCategory {
    let key = tag_key.to_ascii_lowercase();

    if key.contains("gps") || key.contains("latitude") || key.contains("longitude") {
        TagCategory::Location
    } else if key.contains("datetime") || key.contains("timestamp") || key.contains("digitized") {
        TagCategory::DateTime
    } else if key.contains("make")
        || key.contains("model")
        || key.contains("lens")
        || key.contains("serial")
    {
        TagCategory::Camera
    } else if key.contains("iso")
        || key.contains("aperture")
        || key.contains("shutter")
        || key.contains("exposure")
        || key.contains("flash")
    {
        TagCategory::Capture
    } else if key.contains("pixel")
        || key.contains("resolution")
        || key.contains("orientation")
        || key.contains("colorspace")
        || key.contains("width")
        || key.contains("height")
    {
        TagCategory::Image
    } else if key.contains("title")
        || key.contains("description")
        || key.contains("caption")
        || key.contains("keyword")
        || key.contains("copyright")
        || key.contains("artist")
    {
        TagCategory::Description
    } else if key.contains("software") || key.contains("editor") || key.contains("processing") {
        TagCategory::Software
    } else {
        TagCategory::Other
    }
}

fn category_from_token(token: &str) -> Option<TagCategory> {
    match token {
        "camera" => Some(TagCategory::Camera),
        "capture" => Some(TagCategory::Capture),
        "location" => Some(TagCategory::Location),
        "datetime" | "date" | "date/time" | "time" => Some(TagCategory::DateTime),
        "image" => Some(TagCategory::Image),
        "description" => Some(TagCategory::Description),
        "software" => Some(TagCategory::Software),
        "other" => Some(TagCategory::Other),
        _ => None,
    }
}

fn display_name_from_key(tag_key: &str) -> String {
    let raw = tag_key.rsplit('.').next().unwrap_or(tag_key);
    let mut words = Vec::new();
    let mut current = String::new();

    for (index, ch) in raw.chars().enumerate() {
        if ch == '_' || ch == '-' {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            continue;
        }

        if ch.is_ascii_uppercase() && index > 0 && !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }

        current.push(ch);
    }

    if !current.is_empty() {
        words.push(current);
    }

    if words.is_empty() {
        return raw.to_string();
    }

    words
        .into_iter()
        .map(|word| {
            let mut chars = word.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };

            let mut title = String::new();
            title.push(first.to_ascii_uppercase());
            title.push_str(chars.as_str());
            title
        })
        .collect::<Vec<_>>()
        .join(" ")
}
