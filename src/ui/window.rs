use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::app::AppState;
use crate::core::metadata::MetadataEngine;
use crate::models::{MetadataTag, OutputMode, TagCategory, TagValue};
use gpui::{
    div, img, px, rgb, size, AnyElement, App, AppContext as _, Bounds, Context, ElementId,
    ExternalPaths, FocusHandle, Focusable, InteractiveElement as _, IntoElement, KeyDownEvent,
    ObjectFit, ParentElement as _, Render, SharedString, StatefulInteractiveElement as _,
    Styled as _, StyledImage as _, Window, WindowBounds, WindowOptions,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::divider::Divider;
use gpui_component::form::{Field, Form};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::scroll::ScrollableElement as _;
use gpui_component::{
    h_flex, v_flex, Disableable as _, Icon, IconName, Root, Sizable as _, WindowExt as _,
};

const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "tif", "tiff", "webp", "heif", "heic", "avif", "jxl",
];

// ---------------------------------------------------------------------------
// Addable tag definitions
// ---------------------------------------------------------------------------

struct AddableTagDef {
    key: &'static str,
    display_name: &'static str,
    category: TagCategory,
    default_value: TagValue,
}

const ADDABLE_TAGS: &[AddableTagDef] = &[
    // Description / text tags
    AddableTagDef { key: "Exif.Image.ImageDescription", display_name: "Image Description", category: TagCategory::Description, default_value: TagValue::Text(String::new()) },
    AddableTagDef { key: "Exif.Image.Artist", display_name: "Artist", category: TagCategory::Description, default_value: TagValue::Text(String::new()) },
    AddableTagDef { key: "Exif.Image.Copyright", display_name: "Copyright", category: TagCategory::Description, default_value: TagValue::Text(String::new()) },
    // Camera tags
    AddableTagDef { key: "Exif.Image.Make", display_name: "Make", category: TagCategory::Camera, default_value: TagValue::Text(String::new()) },
    AddableTagDef { key: "Exif.Image.Model", display_name: "Model", category: TagCategory::Camera, default_value: TagValue::Text(String::new()) },
    AddableTagDef { key: "Exif.Photo.LensMake", display_name: "Lens Make", category: TagCategory::Camera, default_value: TagValue::Text(String::new()) },
    AddableTagDef { key: "Exif.Photo.LensModel", display_name: "Lens Model", category: TagCategory::Camera, default_value: TagValue::Text(String::new()) },
    AddableTagDef { key: "Exif.Photo.LensSerialNumber", display_name: "Lens Serial Number", category: TagCategory::Camera, default_value: TagValue::Text(String::new()) },
    AddableTagDef { key: "Exif.Photo.OwnerName", display_name: "Owner Name", category: TagCategory::Camera, default_value: TagValue::Text(String::new()) },
    AddableTagDef { key: "Exif.Photo.SerialNumber", display_name: "Serial Number", category: TagCategory::Camera, default_value: TagValue::Text(String::new()) },
    // DateTime tags
    AddableTagDef { key: "Exif.Photo.DateTimeOriginal", display_name: "Date Taken", category: TagCategory::DateTime, default_value: TagValue::DateTime(String::new()) },
    AddableTagDef { key: "Exif.Photo.CreateDate", display_name: "Create Date", category: TagCategory::DateTime, default_value: TagValue::DateTime(String::new()) },
    AddableTagDef { key: "Exif.Image.ModifyDate", display_name: "Modify Date", category: TagCategory::DateTime, default_value: TagValue::DateTime(String::new()) },
    // Software
    AddableTagDef { key: "Exif.Image.Software", display_name: "Software", category: TagCategory::Software, default_value: TagValue::Text(String::new()) },
    // Image properties
    AddableTagDef { key: "Exif.Image.Orientation", display_name: "Orientation", category: TagCategory::Image, default_value: TagValue::Integer(1) },
    AddableTagDef { key: "Exif.Image.XResolution", display_name: "X Resolution", category: TagCategory::Image, default_value: TagValue::Rational(72, 1) },
    AddableTagDef { key: "Exif.Image.YResolution", display_name: "Y Resolution", category: TagCategory::Image, default_value: TagValue::Rational(72, 1) },
    AddableTagDef { key: "Exif.Image.ImageWidth", display_name: "Image Width", category: TagCategory::Image, default_value: TagValue::Integer(0) },
    AddableTagDef { key: "Exif.Image.ImageHeight", display_name: "Image Height", category: TagCategory::Image, default_value: TagValue::Integer(0) },
    // Capture settings
    AddableTagDef { key: "Exif.Photo.ISO", display_name: "ISO", category: TagCategory::Capture, default_value: TagValue::Integer(100) },
    AddableTagDef { key: "Exif.Photo.ExposureTime", display_name: "Exposure Time", category: TagCategory::Capture, default_value: TagValue::Rational(1, 60) },
    AddableTagDef { key: "Exif.Photo.FNumber", display_name: "F-Number", category: TagCategory::Capture, default_value: TagValue::Rational(28, 10) },
    AddableTagDef { key: "Exif.Photo.FocalLength", display_name: "Focal Length", category: TagCategory::Capture, default_value: TagValue::Rational(50, 1) },
    AddableTagDef { key: "Exif.Photo.ApertureValue", display_name: "Aperture Value", category: TagCategory::Capture, default_value: TagValue::Rational(30, 10) },
    AddableTagDef { key: "Exif.Photo.ExposureProgram", display_name: "Exposure Program", category: TagCategory::Capture, default_value: TagValue::Integer(0) },
    AddableTagDef { key: "Exif.Photo.MeteringMode", display_name: "Metering Mode", category: TagCategory::Capture, default_value: TagValue::Integer(0) },
    AddableTagDef { key: "Exif.Photo.Flash", display_name: "Flash", category: TagCategory::Capture, default_value: TagValue::Integer(0) },
    AddableTagDef { key: "Exif.Photo.WhiteBalance", display_name: "White Balance", category: TagCategory::Capture, default_value: TagValue::Integer(0) },
    AddableTagDef { key: "Exif.Photo.ExposureMode", display_name: "Exposure Mode", category: TagCategory::Capture, default_value: TagValue::Integer(0) },
    AddableTagDef { key: "Exif.Photo.ColorSpace", display_name: "Color Space", category: TagCategory::Capture, default_value: TagValue::Integer(1) },
    // Location
    AddableTagDef { key: "Exif.GPSInfo.GPSCoordinates", display_name: "GPS Coordinates", category: TagCategory::Location, default_value: TagValue::Gps(0.0, 0.0, None) },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScalarKind {
    Text,
    Integer,
    Float,
    DateTime,
    Unknown,
}

#[derive(Debug)]
enum TagEditorKind {
    Scalar {
        scalar_kind: ScalarKind,
        input: gpui::Entity<InputState>,
        _subscription: gpui::Subscription,
    },
    Rational {
        numerator: gpui::Entity<InputState>,
        denominator: gpui::Entity<InputState>,
        _num_subscription: gpui::Subscription,
        _den_subscription: gpui::Subscription,
    },
    Gps {
        latitude: gpui::Entity<InputState>,
        longitude: gpui::Entity<InputState>,
        altitude: gpui::Entity<InputState>,
        _lat_subscription: gpui::Subscription,
        _lon_subscription: gpui::Subscription,
        _alt_subscription: gpui::Subscription,
    },
    Binary {
        bytes: usize,
    },
}

#[derive(Debug)]
struct TagEditorRow {
    row_id: String,
    tag_key: String,
    display_name: String,
    parse_error: Option<String>,
    kind: TagEditorKind,
}

#[derive(Clone, Debug)]
struct MapPopupState {
    row_id: String,
    tag_key: String,
    latitude: f64,
    longitude: f64,
    altitude: Option<f64>,
}

impl MapPopupState {
    fn osm_url(&self) -> String {
        format!(
            "https://www.openstreetmap.org/?mlat={:.6}&mlon={:.6}#map=14/{:.6}/{:.6}",
            self.latitude, self.longitude, self.latitude, self.longitude
        )
    }

    fn static_map_url(&self) -> String {
        format!(
            "https://staticmap.openstreetmap.de/staticmap.php?center={:.6},{:.6}&zoom=14&size=600x320&markers={:.6},{:.6},red-pushpin",
            self.latitude, self.longitude, self.latitude, self.longitude
        )
    }
}

#[derive(Debug)]
struct DateTimePopupState {
    row_id: String,
    tag_key: String,
    year: gpui::Entity<InputState>,
    month: gpui::Entity<InputState>,
    day: gpui::Entity<InputState>,
    hour: gpui::Entity<InputState>,
    minute: gpui::Entity<InputState>,
    second: gpui::Entity<InputState>,
}

struct MetaStripWindow {
    state: AppState,
    status: String,
    focus_handle: FocusHandle,
    tag_rows: Vec<TagEditorRow>,
    tag_rows_photo_index: Option<usize>,
    refresh_tag_rows: bool,
    map_popup: Option<MapPopupState>,
    add_tag_popup_open: bool,
    add_tag_search: String,
    add_tag_search_input: Option<gpui::Entity<InputState>>,
    add_tag_search_subscription: Option<gpui::Subscription>,
    datetime_popup: Option<DateTimePopupState>,
}

impl MetaStripWindow {
    fn new(focus_handle: FocusHandle) -> Self {
        let mut state = AppState::default();
        state.active_preset = Some(2);

        Self {
            state,
            status: String::from("Drop photos or click Browse Files to start."),
            focus_handle,
            tag_rows: Vec::new(),
            tag_rows_photo_index: None,
            refresh_tag_rows: true,
            map_popup: None,
            add_tag_popup_open: false,
            add_tag_search: String::new(),
            add_tag_search_input: None,
            add_tag_search_subscription: None,
            datetime_popup: None,
        }
    }

    fn effective_batch_preset_id(&self) -> u64 {
        self.state.active_preset.unwrap_or(2)
    }

    fn on_root_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_focused_input(cx) {
            cx.propagate();
            return;
        }

        match event.keystroke.key.as_str() {
            "left" => {
                self.move_carousel(-1, cx);
                cx.stop_propagation();
            }
            "right" => {
                self.move_carousel(1, cx);
                cx.stop_propagation();
            }
            _ => cx.propagate(),
        }
    }

    fn move_carousel(&mut self, delta: isize, cx: &mut Context<Self>) {
        if self.state.photos.is_empty() {
            return;
        }

        let len = self.state.photos.len() as isize;
        let current = self.state.active_photo.unwrap_or(0) as isize;
        let mut next = current + delta;

        if next < 0 {
            next = len - 1;
        }
        if next >= len {
            next = 0;
        }

        self.state.select_photo(next as usize, false);
        self.refresh_tag_rows = true;
        self.status = format!(
            "Selected {} ({}/{})",
            self.state.photos[next as usize].filename,
            next + 1,
            len
        );
        cx.notify();
    }

    fn browse_files(&mut self, cx: &mut Context<Self>) {
        let maybe_paths = rfd::FileDialog::new()
            .add_filter("Images", IMAGE_EXTENSIONS)
            .pick_files();

        if let Some(paths) = maybe_paths {
            self.import_paths(paths, cx);
        }
    }

    fn import_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        let before_count = self.state.photos.len();
        let expanded_paths = expand_paths(paths);
        let skipped = self.state.import_paths(expanded_paths.clone());
        let imported = self.state.photos.len().saturating_sub(before_count);

        if imported > 0 {
            if self.state.active_photo.is_none() {
                self.state.select_photo(0, false);
            }
            self.refresh_tag_rows = true;
            self.status = format!(
                "Imported {imported} photo(s). Skipped {} unsupported path(s).",
                skipped.len()
            );
        } else {
            self.status = String::from("No new supported photos were imported.");
        }

        cx.notify();
    }

    fn ensure_tag_rows(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let active = self.state.active_photo;

        if !self.refresh_tag_rows && self.tag_rows_photo_index == active {
            return;
        }

        self.tag_rows.clear();
        self.tag_rows_photo_index = active;
        self.map_popup = None;

        let Some(photo_index) = active else {
            self.refresh_tag_rows = false;
            return;
        };

        let tags = self.state.inspector_tags(photo_index);

        for (row_ix, tag) in tags.into_iter().enumerate() {
            let row_id = format!("{}::{row_ix}", tag.key);
            let row = self.build_tag_row(photo_index, row_id, tag, window, cx);
            self.tag_rows.push(row);
        }

        self.refresh_tag_rows = false;
    }

    fn build_tag_row(
        &mut self,
        photo_index: usize,
        row_id: String,
        tag: MetadataTag,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> TagEditorRow {
        let display_name = tag.display_name.clone();
        let tag_key = tag.key.clone();
        let _editable = tag.editable || !matches!(tag.value, TagValue::Binary(_));

        let kind = match tag.value {
            TagValue::Text(value) => {
                let input = cx.new(|cx| InputState::new(window, cx).default_value(value));
                let sub_row_id = row_id.clone();
                let sub_tag_key = tag_key.clone();
                let subscription =
                    cx.subscribe(&input, move |this, input_state, event: &InputEvent, cx| {
                        if matches!(event, InputEvent::Change) {
                            this.commit_scalar_from_input(
                                photo_index,
                                &sub_row_id,
                                &sub_tag_key,
                                ScalarKind::Text,
                                &input_state,
                                cx,
                            );
                        }
                    });

                TagEditorKind::Scalar {
                    scalar_kind: ScalarKind::Text,
                    input,
                    _subscription: subscription,
                }
            }
            TagValue::Integer(value) => {
                let input =
                    cx.new(|cx| InputState::new(window, cx).default_value(value.to_string()));
                let sub_row_id = row_id.clone();
                let sub_tag_key = tag_key.clone();
                let subscription =
                    cx.subscribe(&input, move |this, input_state, event: &InputEvent, cx| {
                        if matches!(event, InputEvent::Change) {
                            this.commit_scalar_from_input(
                                photo_index,
                                &sub_row_id,
                                &sub_tag_key,
                                ScalarKind::Integer,
                                &input_state,
                                cx,
                            );
                        }
                    });

                TagEditorKind::Scalar {
                    scalar_kind: ScalarKind::Integer,
                    input,
                    _subscription: subscription,
                }
            }
            TagValue::Float(value) => {
                let input =
                    cx.new(|cx| InputState::new(window, cx).default_value(format!("{value:.6}")));
                let sub_row_id = row_id.clone();
                let sub_tag_key = tag_key.clone();
                let subscription =
                    cx.subscribe(&input, move |this, input_state, event: &InputEvent, cx| {
                        if matches!(event, InputEvent::Change) {
                            this.commit_scalar_from_input(
                                photo_index,
                                &sub_row_id,
                                &sub_tag_key,
                                ScalarKind::Float,
                                &input_state,
                                cx,
                            );
                        }
                    });

                TagEditorKind::Scalar {
                    scalar_kind: ScalarKind::Float,
                    input,
                    _subscription: subscription,
                }
            }
            TagValue::DateTime(value) => {
                let input = cx.new(|cx| InputState::new(window, cx).default_value(value));
                let sub_row_id = row_id.clone();
                let sub_tag_key = tag_key.clone();
                let subscription =
                    cx.subscribe(&input, move |this, input_state, event: &InputEvent, cx| {
                        if matches!(event, InputEvent::Change) {
                            this.commit_scalar_from_input(
                                photo_index,
                                &sub_row_id,
                                &sub_tag_key,
                                ScalarKind::DateTime,
                                &input_state,
                                cx,
                            );
                        }
                    });

                TagEditorKind::Scalar {
                    scalar_kind: ScalarKind::DateTime,
                    input,
                    _subscription: subscription,
                }
            }
            TagValue::Unknown(value) => {
                let input = cx.new(|cx| InputState::new(window, cx).default_value(value));
                let sub_row_id = row_id.clone();
                let sub_tag_key = tag_key.clone();
                let subscription =
                    cx.subscribe(&input, move |this, input_state, event: &InputEvent, cx| {
                        if matches!(event, InputEvent::Change) {
                            this.commit_scalar_from_input(
                                photo_index,
                                &sub_row_id,
                                &sub_tag_key,
                                ScalarKind::Unknown,
                                &input_state,
                                cx,
                            );
                        }
                    });

                TagEditorKind::Scalar {
                    scalar_kind: ScalarKind::Unknown,
                    input,
                    _subscription: subscription,
                }
            }
            TagValue::Rational(numerator, denominator) => {
                let numerator_input =
                    cx.new(|cx| InputState::new(window, cx).default_value(numerator.to_string()));
                let denominator_input =
                    cx.new(|cx| InputState::new(window, cx).default_value(denominator.to_string()));

                let sub_row_id_num = row_id.clone();
                let sub_tag_key_num = tag_key.clone();
                let num_subscription =
                    cx.subscribe(&numerator_input, move |this, _, event: &InputEvent, cx| {
                        if matches!(event, InputEvent::Change) {
                            this.commit_rational_from_inputs(
                                photo_index,
                                &sub_row_id_num,
                                &sub_tag_key_num,
                                cx,
                            );
                        }
                    });

                let sub_row_id_den = row_id.clone();
                let sub_tag_key_den = tag_key.clone();
                let den_subscription = cx.subscribe(
                    &denominator_input,
                    move |this, _, event: &InputEvent, cx| {
                        if matches!(event, InputEvent::Change) {
                            this.commit_rational_from_inputs(
                                photo_index,
                                &sub_row_id_den,
                                &sub_tag_key_den,
                                cx,
                            );
                        }
                    },
                );

                TagEditorKind::Rational {
                    numerator: numerator_input,
                    denominator: denominator_input,
                    _num_subscription: num_subscription,
                    _den_subscription: den_subscription,
                }
            }
            TagValue::Gps(latitude, longitude, altitude) => {
                let latitude_input = cx
                    .new(|cx| InputState::new(window, cx).default_value(format!("{latitude:.6}")));
                let longitude_input = cx
                    .new(|cx| InputState::new(window, cx).default_value(format!("{longitude:.6}")));
                let altitude_input = cx.new(|cx| {
                    InputState::new(window, cx).default_value(
                        altitude
                            .map(|value| format!("{value:.2}"))
                            .unwrap_or_default(),
                    )
                });

                let sub_row_id_lat = row_id.clone();
                let sub_tag_key_lat = tag_key.clone();
                let lat_subscription =
                    cx.subscribe(&latitude_input, move |this, _, event: &InputEvent, cx| {
                        if matches!(event, InputEvent::Change) {
                            this.commit_gps_from_inputs(
                                photo_index,
                                &sub_row_id_lat,
                                &sub_tag_key_lat,
                                cx,
                            );
                        }
                    });

                let sub_row_id_lon = row_id.clone();
                let sub_tag_key_lon = tag_key.clone();
                let lon_subscription =
                    cx.subscribe(&longitude_input, move |this, _, event: &InputEvent, cx| {
                        if matches!(event, InputEvent::Change) {
                            this.commit_gps_from_inputs(
                                photo_index,
                                &sub_row_id_lon,
                                &sub_tag_key_lon,
                                cx,
                            );
                        }
                    });

                let sub_row_id_alt = row_id.clone();
                let sub_tag_key_alt = tag_key.clone();
                let alt_subscription =
                    cx.subscribe(&altitude_input, move |this, _, event: &InputEvent, cx| {
                        if matches!(event, InputEvent::Change) {
                            this.commit_gps_from_inputs(
                                photo_index,
                                &sub_row_id_alt,
                                &sub_tag_key_alt,
                                cx,
                            );
                        }
                    });

                TagEditorKind::Gps {
                    latitude: latitude_input,
                    longitude: longitude_input,
                    altitude: altitude_input,
                    _lat_subscription: lat_subscription,
                    _lon_subscription: lon_subscription,
                    _alt_subscription: alt_subscription,
                }
            }
            TagValue::Binary(bytes) => TagEditorKind::Binary { bytes: bytes.len() },
        };

        TagEditorRow {
            row_id,
            tag_key,
            display_name,
            parse_error: None,
            kind,
        }
    }

    fn commit_scalar_from_input(
        &mut self,
        photo_index: usize,
        row_id: &str,
        tag_key: &str,
        scalar_kind: ScalarKind,
        input: &gpui::Entity<InputState>,
        cx: &mut Context<Self>,
    ) {
        let raw = input.read(cx).value().to_string();
        let parsed = match scalar_kind {
            ScalarKind::Text => Ok(TagValue::Text(raw)),
            ScalarKind::DateTime => Ok(TagValue::DateTime(raw)),
            ScalarKind::Unknown => Ok(TagValue::Unknown(raw)),
            ScalarKind::Integer => raw
                .trim()
                .parse::<i64>()
                .map(TagValue::Integer)
                .map_err(|_| String::from("Expected an integer value")),
            ScalarKind::Float => raw
                .trim()
                .parse::<f64>()
                .map(TagValue::Float)
                .map_err(|_| String::from("Expected a float value")),
        };

        match parsed {
            Ok(value) => {
                self.set_row_error(row_id, None);
                if let Err(err) = self.state.edit_tag(photo_index, tag_key, value) {
                    self.set_row_error(row_id, Some(format!("Failed to edit tag: {err}")));
                }
            }
            Err(message) => {
                self.set_row_error(row_id, Some(message));
            }
        }

        cx.notify();
    }

    fn commit_rational_from_inputs(
        &mut self,
        photo_index: usize,
        row_id: &str,
        tag_key: &str,
        cx: &mut Context<Self>,
    ) {
        let Some((numerator, denominator)) = self.read_rational_inputs(row_id, cx) else {
            return;
        };

        let numerator = match numerator.trim().parse::<u32>() {
            Ok(value) => value,
            Err(_) => {
                self.set_row_error(
                    row_id,
                    Some(String::from("Numerator must be a positive integer")),
                );
                cx.notify();
                return;
            }
        };

        let denominator = match denominator.trim().parse::<u32>() {
            Ok(value) if value > 0 => value,
            _ => {
                self.set_row_error(
                    row_id,
                    Some(String::from("Denominator must be greater than zero")),
                );
                cx.notify();
                return;
            }
        };

        self.set_row_error(row_id, None);
        if let Err(err) = self.state.edit_tag(
            photo_index,
            tag_key,
            TagValue::Rational(numerator, denominator),
        ) {
            self.set_row_error(row_id, Some(format!("Failed to edit rational tag: {err}")));
        }

        cx.notify();
    }

    fn commit_gps_from_inputs(
        &mut self,
        photo_index: usize,
        row_id: &str,
        tag_key: &str,
        cx: &mut Context<Self>,
    ) {
        let Some((latitude, longitude, altitude)) = self.read_gps_inputs(row_id, cx) else {
            return;
        };

        let latitude = match latitude.trim().parse::<f64>() {
            Ok(value) if (-90.0..=90.0).contains(&value) => value,
            _ => {
                self.set_row_error(
                    row_id,
                    Some(String::from("Latitude must be a number between -90 and 90")),
                );
                cx.notify();
                return;
            }
        };

        let longitude = match longitude.trim().parse::<f64>() {
            Ok(value) if (-180.0..=180.0).contains(&value) => value,
            _ => {
                self.set_row_error(
                    row_id,
                    Some(String::from(
                        "Longitude must be a number between -180 and 180",
                    )),
                );
                cx.notify();
                return;
            }
        };

        let altitude = if altitude.trim().is_empty() {
            None
        } else {
            match altitude.trim().parse::<f64>() {
                Ok(value) => Some(value),
                Err(_) => {
                    self.set_row_error(
                        row_id,
                        Some(String::from("Altitude must be empty or a numeric value")),
                    );
                    cx.notify();
                    return;
                }
            }
        };

        self.set_row_error(row_id, None);
        if let Err(err) = self.state.edit_tag(
            photo_index,
            tag_key,
            TagValue::Gps(latitude, longitude, altitude),
        ) {
            self.set_row_error(row_id, Some(format!("Failed to edit GPS tag: {err}")));
        }

        if let Some(popup) = self.map_popup.as_mut() {
            if popup.row_id == row_id {
                popup.latitude = latitude;
                popup.longitude = longitude;
                popup.altitude = altitude;
            }
        }

        cx.notify();
    }

    fn read_rational_inputs(&self, row_id: &str, cx: &Context<Self>) -> Option<(String, String)> {
        self.tag_rows.iter().find_map(|row| {
            if row.row_id != row_id {
                return None;
            }

            match &row.kind {
                TagEditorKind::Rational {
                    numerator,
                    denominator,
                    ..
                } => Some((
                    numerator.read(cx).value().to_string(),
                    denominator.read(cx).value().to_string(),
                )),
                _ => None,
            }
        })
    }

    fn read_gps_inputs(
        &self,
        row_id: &str,
        cx: &Context<Self>,
    ) -> Option<(String, String, String)> {
        self.tag_rows.iter().find_map(|row| {
            if row.row_id != row_id {
                return None;
            }

            match &row.kind {
                TagEditorKind::Gps {
                    latitude,
                    longitude,
                    altitude,
                    ..
                } => Some((
                    latitude.read(cx).value().to_string(),
                    longitude.read(cx).value().to_string(),
                    altitude.read(cx).value().to_string(),
                )),
                _ => None,
            }
        })
    }

    fn set_row_error(&mut self, row_id: &str, error: Option<String>) {
        if let Some(row) = self.tag_rows.iter_mut().find(|row| row.row_id == row_id) {
            row.parse_error = error;
        }
    }

    fn clear_row(&mut self, tag_key: &str, cx: &mut Context<Self>) {
        let Some(photo_index) = self.state.active_photo else {
            self.status = String::from("No active photo selected");
            cx.notify();
            return;
        };

        match self.state.clear_tag(photo_index, tag_key) {
            Ok(true) => {
                self.status = format!("Cleared {tag_key}");
                self.refresh_tag_rows = true;
            }
            Ok(false) => {
                self.status = format!("Tag not found: {tag_key}");
            }
            Err(err) => {
                self.status = format!("Failed to clear tag {tag_key}: {err}");
            }
        }

        cx.notify();
    }

    fn open_map_popup_for_row(&mut self, row_id: &str, tag_key: &str, cx: &mut Context<Self>) {
        let Some((latitude_raw, longitude_raw, altitude_raw)) = self.read_gps_inputs(row_id, cx)
        else {
            self.status = String::from("Unable to read current GPS values");
            cx.notify();
            return;
        };

        let parsed_latitude = latitude_raw.trim().parse::<f64>().ok();
        let parsed_longitude = longitude_raw.trim().parse::<f64>().ok();
        let parsed_altitude = if altitude_raw.trim().is_empty() {
            Some(None)
        } else {
            altitude_raw.trim().parse::<f64>().ok().map(Some)
        };

        let fallback_gps = self
            .state
            .active_photo
            .and_then(|photo_index| self.state.photos.get(photo_index))
            .and_then(|photo| {
                photo
                    .metadata
                    .all_tags()
                    .find(|tag| tag.key.eq_ignore_ascii_case(tag_key))
            })
            .and_then(|tag| match &tag.value {
                TagValue::Gps(lat, lon, alt) => Some((*lat, *lon, *alt)),
                _ => None,
            });

        let (latitude, longitude, altitude) = match (parsed_latitude, parsed_longitude) {
            (Some(lat), Some(lon)) => (lat, lon, parsed_altitude.flatten()),
            _ => {
                if let Some((lat, lon, alt)) = fallback_gps {
                    self.status = String::from(
                        "Using last saved GPS values because current input is not a valid coordinate",
                    );
                    (lat, lon, alt)
                } else {
                    self.status = String::from(
                        "Latitude/Longitude must be valid numbers before opening map",
                    );
                    cx.notify();
                    return;
                }
            }
        };

        self.map_popup = Some(MapPopupState {
            row_id: String::from(row_id),
            tag_key: String::from(tag_key),
            latitude,
            longitude,
            altitude,
        });

        cx.notify();
    }

    fn close_map_popup(&mut self, cx: &mut Context<Self>) {
        self.map_popup = None;
        cx.notify();
    }

    fn open_map_in_browser(&mut self, cx: &mut Context<Self>) {
        let Some(popup) = &self.map_popup else {
            return;
        };

        let url = popup.osm_url();
        match open_url(&url) {
            Ok(()) => {
                self.status = format!("Opened map: {url}");
            }
            Err(err) => {
                self.status = format!("Failed to open browser: {err}");
            }
        }

        cx.notify();
    }

    // -----------------------------------------------------------------------
    // Add-tag popup
    // -----------------------------------------------------------------------

    fn open_add_tag_popup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.add_tag_popup_open = true;
        self.add_tag_search.clear();
        self.add_tag_search_subscription = None;

        let search_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Type to search tags...")
                .default_value("")
        });

        let subscription =
            cx.subscribe(&search_input, |this, input_state, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.add_tag_search = input_state.read(cx).value().to_string();
                    cx.notify();
                }
            });

        search_input.update(cx, |state, cx| state.focus(window, cx));
        self.add_tag_search_input = Some(search_input);
        self.add_tag_search_subscription = Some(subscription);
        cx.notify();
    }

    fn close_add_tag_popup(&mut self, cx: &mut Context<Self>) {
        self.add_tag_popup_open = false;
        self.add_tag_search.clear();
        self.add_tag_search_input = None;
        self.add_tag_search_subscription = None;
        cx.notify();
    }

    fn add_tag_from_popup(&mut self, key: &str, cx: &mut Context<Self>) {
        let Some(photo_index) = self.state.active_photo else {
            self.status = String::from("No active photo selected");
            cx.notify();
            return;
        };

        let def = ADDABLE_TAGS.iter().find(|d| d.key == key);
        let value = def.map(|d| d.default_value.clone()).unwrap_or(TagValue::Text(String::new()));

        match self.state.edit_tag(photo_index, key, value) {
            Ok(()) => {
                self.status = format!("Added {key}");
                self.refresh_tag_rows = true;
            }
            Err(err) => {
                self.status = format!("Failed to add tag: {err}");
            }
        }

        self.add_tag_popup_open = false;
        self.add_tag_search.clear();
        self.add_tag_search_input = None;
        self.add_tag_search_subscription = None;
        cx.notify();
    }

    fn available_addable_tags(&self) -> Vec<&'static AddableTagDef> {
        let Some(photo_index) = self.state.active_photo else {
            return Vec::new();
        };
        let Some(photo) = self.state.photos.get(photo_index) else {
            return Vec::new();
        };

        let existing_keys: std::collections::HashSet<String> = photo
            .metadata
            .all_tags()
            .map(|t| t.key.to_ascii_lowercase())
            .collect();

        let query = self.add_tag_search.trim().to_ascii_lowercase();

        ADDABLE_TAGS
            .iter()
            .filter(|def| !existing_keys.contains(&def.key.to_ascii_lowercase()))
            .filter(|def| {
                query.is_empty()
                    || def.display_name.to_ascii_lowercase().contains(&query)
                    || def.key.to_ascii_lowercase().contains(&query)
                    || def.category.as_str().to_ascii_lowercase().contains(&query)
            })
            .collect()
    }

    fn render_add_tag_popup(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.add_tag_popup_open {
            return None;
        }

        let available = self.available_addable_tags();

        let tag_list: Vec<AnyElement> = available
            .iter()
            .map(|def| {
                let key = def.key;
                h_flex()
                    .id(SharedString::from(format!("add-tag-{key}")))
                    .w_full()
                    .px_3()
                    .py_1()
                    .gap_3()
                    .items_center()
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(0x1d2632)))
                    .rounded_sm()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.add_tag_from_popup(key, cx);
                    }))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(rgb(0xe7edf4))
                            .child(def.display_name),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x6b7a8d))
                            .child(def.category.as_str()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(rgb(0x4a5568))
                            .overflow_hidden()
                            .child(def.key),
                    )
                    .into_any_element()
            })
            .collect();

        Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bottom_0()
                .bg(rgb(0x060b10))
                .opacity(0.96)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    v_flex()
                        .w(px(560.0))
                        .max_h(px(520.0))
                        .overflow_hidden()
                        .p_4()
                        .gap_3()
                        .bg(rgb(0x111820))
                        .border_1()
                        .border_color(rgb(0x2a3545))
                        .rounded_md()
                        .child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .text_lg()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(rgb(0xe7edf4))
                                        .child("Add Metadata Field"),
                                )
                                .child(
                                    Button::new("close-add-tag")
                                        .ghost()
                                        .small()
                                        .icon(IconName::Close)
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.close_add_tag_popup(cx)),
                                        ),
                                ),
                        )
                        .child(
                            self.add_tag_search_input.as_ref().map_or_else(
                                || {
                                    div()
                                        .w_full()
                                        .px_2()
                                        .py_1()
                                        .bg(rgb(0x0a1018))
                                        .border_1()
                                        .border_color(rgb(0x222a33))
                                        .rounded_sm()
                                        .text_sm()
                                        .text_color(rgb(0xa8b5c2))
                                        .child("Type to search tags...")
                                        .into_any_element()
                                },
                                |search_input| {
                                    Input::new(search_input)
                                        .w_full()
                                        .small()
                                        .into_any_element()
                                },
                            ),
                        )
                        .child(
                            div()
                                .id(SharedString::from("add-tag-list"))
                                .flex_1()
                                .w_full()
                                .overflow_y_scrollbar()
                                .child(
                                    v_flex().w_full().gap_1().children(
                                        if tag_list.is_empty() {
                                            vec![div()
                                                .py_4()
                                                .text_sm()
                                                .text_color(rgb(0x6b7a8d))
                                                .child("All supported tags are already present on this photo.")
                                                .into_any_element()]
                                        } else {
                                            tag_list
                                        },
                                    ),
                                ),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x4a5568))
                                .child(format!(
                                    "{} tag(s) available",
                                    available.len()
                                )),
                        ),
                )
                .into_any_element(),
        )
    }

    // -----------------------------------------------------------------------
    // DateTime picker popup
    // -----------------------------------------------------------------------

    fn open_datetime_popup(
        &mut self,
        row_id: &str,
        tag_key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Try to parse existing value "YYYY:MM:DD HH:MM:SS"
        let (yr, mo, dy, hr, mi, se) = self
            .tag_rows
            .iter()
            .find(|r| r.row_id == row_id)
            .and_then(|r| match &r.kind {
                TagEditorKind::Scalar { input, .. } => {
                    let raw = input.read(cx).value().to_string();
                    parse_datetime_parts(&raw)
                }
                _ => None,
            })
            .unwrap_or(("2025".into(), "01".into(), "01".into(), "12".into(), "00".into(), "00".into()));

        let year = cx.new(|cx| InputState::new(window, cx).default_value(yr));
        let month = cx.new(|cx| InputState::new(window, cx).default_value(mo));
        let day = cx.new(|cx| InputState::new(window, cx).default_value(dy));
        let hour = cx.new(|cx| InputState::new(window, cx).default_value(hr));
        let minute = cx.new(|cx| InputState::new(window, cx).default_value(mi));
        let second = cx.new(|cx| InputState::new(window, cx).default_value(se));

        self.datetime_popup = Some(DateTimePopupState {
            row_id: row_id.to_string(),
            tag_key: tag_key.to_string(),
            year,
            month,
            day,
            hour,
            minute,
            second,
        });

        cx.notify();
    }

    fn close_datetime_popup(&mut self, cx: &mut Context<Self>) {
        self.datetime_popup = None;
        cx.notify();
    }

    fn commit_datetime_popup(&mut self, cx: &mut Context<Self>) {
        let Some(photo_index) = self.state.active_photo else {
            self.datetime_popup = None;
            cx.notify();
            return;
        };

        let Some(popup) = &self.datetime_popup else {
            return;
        };

        let yr = popup.year.read(cx).value().to_string();
        let mo = popup.month.read(cx).value().to_string();
        let dy = popup.day.read(cx).value().to_string();
        let hr = popup.hour.read(cx).value().to_string();
        let mi = popup.minute.read(cx).value().to_string();
        let se = popup.second.read(cx).value().to_string();

        let formatted = format!(
            "{:0>4}:{:0>2}:{:0>2} {:0>2}:{:0>2}:{:0>2}",
            yr.trim(),
            mo.trim(),
            dy.trim(),
            hr.trim(),
            mi.trim(),
            se.trim()
        );

        let tag_key = popup.tag_key.clone();

        match self.state.edit_tag(photo_index, &tag_key, TagValue::DateTime(formatted.clone())) {
            Ok(()) => {
                self.status = format!("Set {tag_key} = {formatted}");
                self.refresh_tag_rows = true;
            }
            Err(err) => {
                self.status = format!("Failed to set date/time: {err}");
            }
        }

        self.datetime_popup = None;
        cx.notify();
    }

    fn render_datetime_popup(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let popup = self.datetime_popup.as_ref()?;

        Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bottom_0()
                .bg(rgb(0x060b10))
                .opacity(0.96)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    v_flex()
                        .w(px(440.0))
                        .p_4()
                        .gap_3()
                        .bg(rgb(0x111820))
                        .border_1()
                        .border_color(rgb(0x2a3545))
                        .rounded_md()
                        .child(
                            div()
                                .text_lg()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(rgb(0xe7edf4))
                                .child("Set Date & Time"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x8d9cac))
                                .child(format!("Tag: {}", popup.tag_key)),
                        )
                        .child(
                            h_flex()
                                .w_full()
                                .gap_2()
                                .items_end()
                                .child(
                                    v_flex()
                                        .gap_1()
                                        .child(div().text_xs().text_color(rgb(0x8d9cac)).child("Year"))
                                        .child(Input::new(&popup.year).w(px(72.0))),
                                )
                                .child(
                                    v_flex()
                                        .gap_1()
                                        .child(div().text_xs().text_color(rgb(0x8d9cac)).child("Month"))
                                        .child(Input::new(&popup.month).w(px(52.0))),
                                )
                                .child(
                                    v_flex()
                                        .gap_1()
                                        .child(div().text_xs().text_color(rgb(0x8d9cac)).child("Day"))
                                        .child(Input::new(&popup.day).w(px(52.0))),
                                ),
                        )
                        .child(
                            h_flex()
                                .w_full()
                                .gap_2()
                                .items_end()
                                .child(
                                    v_flex()
                                        .gap_1()
                                        .child(div().text_xs().text_color(rgb(0x8d9cac)).child("Hour"))
                                        .child(Input::new(&popup.hour).w(px(52.0))),
                                )
                                .child(
                                    v_flex()
                                        .gap_1()
                                        .child(div().text_xs().text_color(rgb(0x8d9cac)).child("Min"))
                                        .child(Input::new(&popup.minute).w(px(52.0))),
                                )
                                .child(
                                    v_flex()
                                        .gap_1()
                                        .child(div().text_xs().text_color(rgb(0x8d9cac)).child("Sec"))
                                        .child(Input::new(&popup.second).w(px(52.0))),
                                ),
                        )
                        .child(
                            h_flex()
                                .pt_2()
                                .gap_2()
                                .justify_end()
                                .child(
                                    Button::new("dt-apply")
                                        .small()
                                        .primary()
                                        .icon(IconName::Check)
                                        .label("Apply")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.commit_datetime_popup(cx);
                                        })),
                                )
                                .child(
                                    Button::new("dt-cancel")
                                        .small()
                                        .ghost()
                                        .label("Cancel")
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.close_datetime_popup(cx)),
                                        ),
                                ),
                        ),
                )
                .into_any_element(),
        )
    }

    fn save_active(&mut self, cx: &mut Context<Self>) {
        let Some(photo_index) = self.state.active_photo else {
            self.status = String::from("No active photo selected");
            cx.notify();
            return;
        };

        match self.state.save_photo_changes(photo_index) {
            Ok(()) => {
                self.status = format!("Saved {}", self.state.photos[photo_index].filename);
            }
            Err(err) => {
                self.status = format!("Save failed: {err}");
            }
        }

        cx.notify();
    }

    fn export_active(&mut self, cx: &mut Context<Self>) {
        let Some(photo_index) = self.state.active_photo else {
            self.status = String::from("No active photo selected");
            cx.notify();
            return;
        };

        let Some(export_dir) = rfd::FileDialog::new()
            .set_title("Choose export folder")
            .pick_folder()
        else {
            self.status = String::from("Export cancelled");
            cx.notify();
            return;
        };

        let photo = &self.state.photos[photo_index];
        let output_path = unique_export_path(&export_dir, &photo.filename, "_export");

        if let Err(err) = fs::copy(&photo.path, &output_path) {
            self.status = format!("Failed to copy file to export path: {err}");
            cx.notify();
            return;
        }

        if let Err(err) = MetadataEngine::write(&output_path, &photo.metadata) {
            self.status = format!("File copied, but metadata export failed: {err}");
            cx.notify();
            return;
        }

        self.status = format!("Exported {}", output_path.display());
        cx.notify();
    }

    fn batch_clear(&mut self, cx: &mut Context<Self>) {
        if self.state.photos.is_empty() {
            self.status = String::from("No photos loaded");
            cx.notify();
            return;
        }

        if self.state.selected_indices.is_empty() {
            self.state.select_all_visible();
        }

        let preset_id = self.effective_batch_preset_id();
        match self
            .state
            .run_bulk_selected(preset_id, OutputMode::Overwrite, None)
        {
            Ok(summary) => {
                self.status = format!(
                    "Batch clear complete: {} ok, {} failed, {} cancelled",
                    summary.succeeded, summary.failed, summary.cancelled
                );
                self.refresh_tag_rows = true;
            }
            Err(err) => {
                self.status = format!("Batch clear failed: {err}");
            }
        }

        cx.notify();
    }

    fn render_upload_box(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id(SharedString::from("upload-drop-zone"))
            .w_full()
            .h_full()
            .min_h(px(560.0))
            .p_4()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgb(0x11161d))
            .border_1()
            .border_color(rgb(0x222a33))
            .can_drop(|value, _, _| value.is::<ExternalPaths>())
            .drag_over::<ExternalPaths>(|style, _, _, _| {
                style.bg(rgb(0xede7db)).border_color(rgb(0xb8a98c))
            })
            .on_click(cx.listener(|this, _, _, cx| this.browse_files(cx)))
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                this.import_paths(paths.paths().to_vec(), cx);
            }))
            .cursor_pointer()
            .child(
                v_flex()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .text_color(rgb(0xe7edf4))
                    .child(Icon::new(IconName::FolderOpen).large())
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Upload Photos"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x8d9cac))
                            .child("Drag photos here or click to browse"),
                    ),
            )
            .into_any_element()
    }

    fn render_carousel(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.state.photos.is_empty() {
            return self.render_upload_box(cx);
        }

        let active_index = self.state.active_photo.unwrap_or(0);
        let photo = &self.state.photos[active_index];
        let disable_nav = self.state.photos.len() <= 1;

        v_flex()
            .flex_1()
            .w_full()
            .h_full()
            .gap_2()
            .p_2()
            .can_drop(|value, _, _| value.is::<ExternalPaths>())
            .drag_over::<ExternalPaths>(|style, _, _, _| style.bg(rgb(0x18202a)))
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                this.import_paths(paths.paths().to_vec(), cx);
            }))
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .child(
                        Button::new("carousel-prev")
                            .ghost()
                            .small()
                            .icon(IconName::ChevronLeft)
                            .disabled(disable_nav)
                            .on_click(cx.listener(|this, _, _, cx| this.move_carousel(-1, cx))),
                    )
                    .child(div().text_sm().text_color(rgb(0xa8b5c2)).child(format!(
                        "{}/{}",
                        active_index + 1,
                        self.state.photos.len()
                    )))
                    .child(
                        Button::new("carousel-next")
                            .ghost()
                            .small()
                            .icon(IconName::ChevronRight)
                            .disabled(disable_nav)
                            .on_click(cx.listener(|this, _, _, cx| this.move_carousel(1, cx))),
                    ),
            )
            .child(
                div().w_full().flex_1().min_h(px(460.0)).child(
                    div()
                        .w_full()
                        .h_full()
                        .bg(rgb(0x0a0f14))
                        .border_1()
                        .border_color(rgb(0x222a33))
                        .overflow_hidden()
                        .child(
                            img(photo.path.clone())
                                .w_full()
                                .h_full()
                                .object_fit(ObjectFit::Cover)
                                .with_fallback(|| image_fallback("No preview available")),
                        ),
                ),
            )
            .child(
                h_flex().w_full().justify_start().child(
                    div()
                        .w_full()
                        .text_sm()
                        .text_color(rgb(0xa8b5c2))
                        .child(photo.filename.clone()),
                ),
            )
            .child(self.render_thumbnail_strip(cx))
            .into_any_element()
    }

    fn render_thumbnail_strip(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id(SharedString::from("carousel-thumbnails"))
            .h(px(112.0))
            .w_full()
            .overflow_x_scrollbar()
            .child(h_flex().h_full().items_start().gap_2().pr_3().children(
                self.state.photos.iter().enumerate().map(|(index, photo)| {
                    let is_active = self.state.active_photo == Some(index);
                    let filename = photo.filename.clone();

                    div()
                        .id(SharedString::from(format!("thumb-{index}")))
                        .w(px(96.0))
                        .h(px(96.0))
                        .flex_none()
                        .overflow_hidden()
                        .bg(rgb(0x0a0f14))
                        .border_1()
                        .border_color(if is_active {
                            rgb(0x3e77b6)
                        } else {
                            rgb(0x25303c)
                        })
                        .cursor_pointer()
                        .hover(|style| style.bg(rgb(0x18202a)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.state.select_photo(index, false);
                            this.refresh_tag_rows = true;
                            this.status = format!("Selected {filename}");
                            cx.notify();
                        }))
                        .child(
                            img(photo.path.clone())
                                .w_full()
                                .h_full()
                                .object_fit(ObjectFit::Cover)
                                .with_fallback(|| image_fallback("No preview")),
                        )
                }),
            ))
            .into_any_element()
    }

    fn render_action_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let has_photo = self.state.active_photo.is_some();
        let has_photos = !self.state.photos.is_empty();

        h_flex()
            .h(px(44.0))
            .w_full()
            .items_center()
            .gap_2()
            .child(
                Button::new("save-active")
                    .small()
                    .primary()
                    .icon(IconName::Check)
                    .label("Save")
                    .disabled(!has_photo)
                    .on_click(cx.listener(|this, _, _, cx| this.save_active(cx))),
            )
            .child(
                Button::new("export-active")
                    .small()
                    .icon(IconName::ExternalLink)
                    .label("Export")
                    .disabled(!has_photo)
                    .on_click(cx.listener(|this, _, _, cx| this.export_active(cx))),
            )
            .child(
                Button::new("batch-clear")
                    .small()
                    .danger()
                    .icon(IconName::Delete)
                    .label("Batch Clear")
                    .disabled(!has_photos)
                    .on_click(cx.listener(|this, _, _, cx| this.batch_clear(cx))),
            )
            .into_any_element()
    }

    fn render_left_pane(&self, cx: &mut Context<Self>) -> AnyElement {
        let media = if self.state.photos.is_empty() {
            self.render_upload_box(cx)
        } else {
            self.render_carousel(cx)
        };

        div()
            .id(SharedString::from("left-pane"))
            .w_2_3()
            .max_w(px(980.0))
            .h_full()
            .bg(rgb(0x0e141b))
            .border_1()
            .border_color(rgb(0x222a33))
            .child(
                v_flex()
                    .h_full()
                    .w_full()
                    .gap_2()
                    .child(div().flex_1().child(media))
                    .child(div().px_2().child(self.render_action_row(cx))),
            )
            .into_any_element()
    }

    fn render_tag_field(&self, row: &TagEditorRow, cx: &mut Context<Self>) -> Field {
        let label = row.display_name.clone();

        let editor = match &row.kind {
            TagEditorKind::Scalar {
                scalar_kind, input, ..
            } => {
                let tag_key = row.tag_key.clone();
                let tag_key_for_clear = row.tag_key.clone();
                let scalar_kind = *scalar_kind;
                let input_for_check = input.clone();
                let input_widget = Input::new(input).w_full().suffix(
                    Button::new((ElementId::from("clear-inline"), row.row_id.clone()))
                        .ghost()
                        .xsmall()
                        .icon(IconName::CircleX)
                        .tab_stop(false)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let current = input_for_check.read(cx).value().to_string();
                            if current.trim().is_empty() {
                                // Already empty — remove the tag entirely
                                this.clear_row(&tag_key_for_clear, cx);
                            } else {
                                // Clear the value to empty
                                let empty_value = match scalar_kind {
                                    ScalarKind::DateTime => TagValue::DateTime(String::new()),
                                    ScalarKind::Integer => TagValue::Integer(0),
                                    ScalarKind::Float => TagValue::Float(0.0),
                                    _ => TagValue::Text(String::new()),
                                };
                                if let Some(photo_index) = this.state.active_photo {
                                    let _ = this.state.edit_tag(photo_index, &tag_key, empty_value);
                                }
                                this.refresh_tag_rows = true;
                                cx.notify();
                            }
                        })),
                );

                if matches!(scalar_kind, ScalarKind::DateTime) {
                    let dt_row_id = row.row_id.clone();
                    let dt_tag_key = row.tag_key.clone();
                    h_flex()
                        .w_full()
                        .gap_1()
                        .items_center()
                        .child(input_widget)
                        .child(
                            Button::new((ElementId::from("dt-pick"), row.row_id.clone()))
                                .ghost()
                                .small()
                                .icon(IconName::Calendar)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.open_datetime_popup(&dt_row_id, &dt_tag_key, window, cx);
                                })),
                        )
                        .into_any_element()
                } else {
                    input_widget.into_any_element()
                }
            }
            TagEditorKind::Rational {
                numerator,
                denominator,
                ..
            } => {
                let tag_key = row.tag_key.clone();

                h_flex()
                    .w_full()
                    .gap_2()
                    .items_center()
                    .child(Input::new(numerator).w(px(96.0)))
                    .child(div().text_sm().text_color(rgb(0x7a746a)).child("/"))
                    .child(Input::new(denominator).w(px(96.0)))
                    .child(
                        Button::new((ElementId::from("clear-rational"), row.row_id.clone()))
                            .ghost()
                            .xsmall()
                            .icon(IconName::CircleX)
                            .tab_stop(false)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.clear_row(&tag_key, cx);
                            })),
                    )
                    .into_any_element()
            }
            TagEditorKind::Gps {
                latitude,
                longitude,
                altitude,
                ..
            } => {
                let row_id = row.row_id.clone();
                let tag_key_for_map = row.tag_key.clone();
                let tag_key_for_clear = row.tag_key.clone();

                h_flex()
                    .w_full()
                    .gap_2()
                    .items_center()
                    .child(Input::new(latitude).w(px(110.0)))
                    .child(Input::new(longitude).w(px(110.0)))
                    .child(Input::new(altitude).w(px(110.0)))
                    .child(
                        Button::new((ElementId::from("map"), row.row_id.clone()))
                            .small()
                            .ghost()
                            .icon(IconName::Map)
                            .label("Map")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_map_popup_for_row(&row_id, &tag_key_for_map, cx);
                            })),
                    )
                    .child(
                        Button::new((ElementId::from("clear-gps"), row.row_id.clone()))
                            .ghost()
                            .xsmall()
                            .icon(IconName::CircleX)
                            .tab_stop(false)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.clear_row(&tag_key_for_clear, cx);
                            })),
                    )
                    .into_any_element()
            }
            TagEditorKind::Binary { bytes } => {
                let tag_key = row.tag_key.clone();
                h_flex()
                    .w_full()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .text_color(rgb(0x7a746a))
                            .child(format!("<{bytes} bytes>")),
                    )
                    .child(
                        Button::new((ElementId::from("clear-binary"), row.row_id.clone()))
                            .ghost()
                            .xsmall()
                            .icon(IconName::CircleX)
                            .tab_stop(false)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.clear_row(&tag_key, cx);
                            })),
                    )
                    .into_any_element()
            }
        };

        let mut field = Field::new().label(label).items_start().child(editor);
        if let Some(error) = row.parse_error.as_ref() {
            let error_text = error.clone();
            field = field.description_fn(move |_, _| {
                div().text_color(rgb(0xb33928)).child(error_text.clone())
            });
        }

        field
    }

    fn render_metadata_editor(&self, cx: &mut Context<Self>) -> AnyElement {
        let fields: Vec<Field> = self
            .tag_rows
            .iter()
            .map(|row| self.render_tag_field(row, cx))
            .collect();

        let has_photo = self.state.active_photo.is_some();

        div()
            .id(SharedString::from("metadata-pane"))
            .w_1_3()
            .h_full()
            .bg(rgb(0x0e141b))
            .border_1()
            .border_color(rgb(0x222a33))
            .child(
                div()
                    .id(SharedString::from("metadata-scroll"))
                    .flex_1()
                    .w_full()
                    .h_full()
                    .p_2()
                    .overflow_y_scrollbar()
                    .child(
                        v_flex()
                            .w_full()
                            .gap_2()
                            .child(
                                Form::vertical()
                                    .label_width(px(170.0))
                                    .children(fields)
                                    .w_full(),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .pt_2()
                                    .child(
                                    Button::new("add-metadata")
                                            .small()
                                            .icon(IconName::Plus)
                                            .label("Add Metadata")
                                            .disabled(!has_photo)
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.open_add_tag_popup(window, cx)
                                            })),
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_map_popup(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let popup = self.map_popup.as_ref()?;

        Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bottom_0()
                .bg(rgb(0x202830))
                .opacity(0.96)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(620.0))
                        .p_4()
                        .gap_2()
                        .flex()
                        .flex_col()
                        .bg(rgb(0xf7f5f0))
                        .border_1()
                        .border_color(rgb(0xb8b0a0))
                        .rounded_md()
                        .child(
                            div()
                                .text_lg()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child("Location Map"),
                        )
                        .child(format!(
                            "Tag: {} | lat={:.6} lon={:.6}{}",
                            popup.tag_key,
                            popup.latitude,
                            popup.longitude,
                            popup
                                .altitude
                                .map(|value| format!(" alt={value:.2}m"))
                                .unwrap_or_default()
                        ))
                        .child("Map preview URL (OpenStreetMap):")
                        .child(
                            div()
                                .w_full()
                                .h(px(320.0))
                                .bg(rgb(0xece7dd))
                                .border_1()
                                .border_color(rgb(0xd6d0c4))
                                .rounded_sm()
                                .overflow_hidden()
                                .child(
                                    img(popup.static_map_url())
                                        .w_full()
                                        .h_full()
                                        .object_fit(ObjectFit::Cover)
                                        .with_fallback(|| {
                                            div()
                                                .size_full()
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .text_sm()
                                                .text_color(rgb(0x6b6560))
                                                .child("Map preview unavailable")
                                                .into_any_element()
                                        }),
                                ),
                        )
                        .child(
                            div()
                                .p_2()
                                .bg(rgb(0xeee9e0))
                                .border_1()
                                .border_color(rgb(0xd6d0c4))
                                .rounded_sm()
                                .child(popup.osm_url()),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x6b6560))
                                .child(
                                    "Use the GPS inputs in the row to adjust coordinates, then open in browser to inspect location.",
                                ),
                        )
                        .child(
                            h_flex()
                                .pt_2()
                                .gap_2()
                                .justify_end()
                                .child(
                                    Button::new("map-open-browser")
                                        .small()
                                        .primary()
                                        .icon(IconName::ExternalLink)
                                        .label("Open in Browser")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.open_map_in_browser(cx)
                                        })),
                                )
                                .child(
                                    Button::new("map-clear")
                                        .small()
                                        .danger()
                                        .icon(IconName::Delete)
                                        .label("Clear Location")
                                        .on_click(cx.listener({
                                            let tag_key = popup.tag_key.clone();
                                            move |this, _, _, cx| {
                                                this.clear_row(&tag_key, cx);
                                                this.close_map_popup(cx);
                                            }
                                        })),
                                )
                                .child(
                                    Button::new("map-close")
                                        .small()
                                        .ghost()
                                        .label("Close")
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.close_map_popup(cx)),
                                        ),
                                ),
                        ),
                )
                .into_any_element(),
        )
    }
}

impl Focusable for MetaStripWindow {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MetaStripWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_tag_rows(window, cx);

        div()
            .id(SharedString::from("metastrip-root"))
            .track_focus(&self.focus_handle(cx))
            .on_key_down(cx.listener(Self::on_root_key_down))
            .size_full()
            .relative()
            .gap_0()
            .flex()
            .bg(rgb(0x060b10))
            .text_color(rgb(0xe7edf4))
            .child(self.render_left_pane(cx))
            .child(Divider::vertical().color(rgb(0x222a33)))
            .child(self.render_metadata_editor(cx))
            .children(self.render_map_popup(cx))
            .children(self.render_add_tag_popup(cx))
            .children(self.render_datetime_popup(cx))
    }
}

fn image_fallback(message: &str) -> AnyElement {
    div()
        .size_full()
        .bg(rgb(0x0a0f14))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_1()
        .text_color(rgb(0x8d9cac))
        .child(Icon::new(IconName::File).size(px(16.0)))
        .child(div().text_xs().child(message.to_string()))
        .into_any_element()
}

fn looks_like_image(path: &Path) -> bool {
    let Some(extension) = path.extension() else {
        return false;
    };

    let extension = extension.to_string_lossy().to_ascii_lowercase();
    IMAGE_EXTENSIONS.iter().any(|known| *known == extension)
}

fn expand_paths(input_paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for path in input_paths {
        collect_path(&path, &mut files);
    }

    files
}

fn collect_path(path: &Path, files: &mut Vec<PathBuf>) {
    if path.is_file() {
        if looks_like_image(path) {
            files.push(path.to_path_buf());
        }
        return;
    }

    if !path.is_dir() {
        return;
    }

    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        collect_path(&entry.path(), files);
    }
}

fn unique_export_path(export_dir: &Path, filename: &str, suffix: &str) -> PathBuf {
    let input_path = Path::new(filename);
    let stem = input_path
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("photo"));
    let extension = input_path
        .extension()
        .map(|value| value.to_string_lossy().to_string());

    for attempt in 0..1000_usize {
        let candidate_name = if attempt == 0 {
            match &extension {
                Some(ext) => format!("{stem}{suffix}.{ext}"),
                None => format!("{stem}{suffix}"),
            }
        } else {
            match &extension {
                Some(ext) => format!("{stem}{suffix}_{attempt}.{ext}"),
                None => format!("{stem}{suffix}_{attempt}"),
            }
        };

        let candidate = export_dir.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
    }

    export_dir.join(format!("{stem}{suffix}_overflow"))
}

fn open_url(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open").arg(url).status()?;
        if status.success() {
            return Ok(());
        }
        return Err(std::io::Error::other(format!(
            "open command failed with status {status}"
        )));
    }

    #[cfg(target_os = "windows")]
    {
        let status = Command::new("cmd").args(["/C", "start", "", url]).status()?;
        if status.success() {
            return Ok(());
        }
        return Err(std::io::Error::other(format!(
            "start command failed with status {status}"
        )));
    }

    #[cfg(target_os = "linux")]
    {
        let status = Command::new("xdg-open").arg(url).status()?;
        if status.success() {
            return Ok(());
        }
        return Err(std::io::Error::other(format!(
            "xdg-open command failed with status {status}"
        )));
    }

    #[allow(unreachable_code)]
    #[allow(unused_variables)]
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "opening URLs is not supported on this platform",
    ))
}

/// Parse an EXIF date/time string "YYYY:MM:DD HH:MM:SS" into 6 components.
fn parse_datetime_parts(raw: &str) -> Option<(String, String, String, String, String, String)> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    // Expected format: "YYYY:MM:DD HH:MM:SS"
    let parts: Vec<&str> = raw.splitn(2, ' ').collect();
    let date_str = parts.first()?;
    let time_str = parts.get(1).unwrap_or(&"00:00:00");

    let date_parts: Vec<&str> = date_str.split(':').collect();
    let time_parts: Vec<&str> = time_str.split(':').collect();

    if date_parts.len() < 3 {
        return None;
    }

    Some((
        date_parts[0].to_string(),
        date_parts[1].to_string(),
        date_parts[2].to_string(),
        time_parts.first().unwrap_or(&"00").to_string(),
        time_parts.get(1).unwrap_or(&"00").to_string(),
        time_parts.get(2).unwrap_or(&"00").to_string(),
    ))
}

pub fn open_metastrip_window(cx: &mut App) {
    let bounds = Bounds::centered(None, size(px(1380.0), px(900.0)), cx);

    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(gpui::TitlebarOptions {
                title: Some("MetaStrip".into()),
                appears_transparent: false,
                traffic_light_position: None,
            }),
            ..Default::default()
        },
        |window, cx| {
            let view = cx.new(|cx| MetaStripWindow::new(cx.focus_handle()));
            cx.new(|cx| Root::new(view, window, cx))
        },
    )
    .expect("failed to open MetaStrip window");
}
