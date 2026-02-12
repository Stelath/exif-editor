use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{Datelike, NaiveDate};

use crate::app::AppState;
use crate::core::metadata::MetadataEngine;
use crate::models::{MetadataTag, TagCategory, TagValue};
use gpui::{
    div, img, px, size, AnyElement, App, AppContext as _, Bounds, Context, ElementId,
    ExternalPaths, FocusHandle, Focusable, InteractiveElement as _, IntoElement, KeyDownEvent,
    ObjectFit, ParentElement as _, Render, SharedString, StatefulInteractiveElement as _,
    Styled as _, StyledImage as _, Window, WindowBounds, WindowOptions,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::calendar::Date;
use gpui_component::date_picker::{DatePicker, DatePickerState};
use gpui_component::divider::Divider;
use gpui_component::form::{Field, Form};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::scroll::ScrollableElement as _;
use gpui_component::theme::{ActiveTheme, Theme, ThemeMode};
use gpui_component::{
    h_flex, v_flex, Disableable as _, Icon, IconName, Root, Sizable as _, WindowExt as _,
};

mod actions;
mod popups;
mod render_media;
mod render_metadata;
mod render_shell;
mod state;
mod tag_rows;
mod utils;

use self::utils::{
    expand_paths, image_fallback, open_url, parse_datetime_parts, unique_export_path,
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

#[derive(Debug)]
struct DateTimePopupState {
    tag_key: String,
    date_picker: gpui::Entity<DatePickerState>,
    hour: gpui::Entity<InputState>,
    minute: gpui::Entity<InputState>,
    second: gpui::Entity<InputState>,
}

struct ExifEditorWindow {
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
    metadata_filter: String,
    metadata_filter_input: Option<gpui::Entity<InputState>>,
    metadata_filter_subscription: Option<gpui::Subscription>,
}

impl Focusable for ExifEditorWindow {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

pub fn open_exif_editor_window(cx: &mut App) {
    let bounds = Bounds::centered(None, size(px(1100.0), px(750.0)), cx);

    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(gpui::TitlebarOptions {
                title: Some("Exif Editor".into()),
                appears_transparent: false,
                traffic_light_position: None,
            }),
            ..Default::default()
        },
        |window, cx| {
            let view = cx.new(|cx| ExifEditorWindow::new(cx.focus_handle()));
            cx.new(|cx| Root::new(view, window, cx))
        },
    )
    .expect("failed to open Exif Editor window");
}
