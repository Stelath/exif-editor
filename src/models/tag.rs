use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MetadataTag {
    pub key: String,
    pub display_name: String,
    pub value: TagValue,
    pub category: TagCategory,
    pub editable: bool,
    pub marked_for_removal: bool,
}

impl MetadataTag {
    pub fn new<K: Into<String>, D: Into<String>>(
        key: K,
        display_name: D,
        value: TagValue,
        category: TagCategory,
    ) -> Self {
        Self {
            key: key.into(),
            display_name: display_name.into(),
            value,
            category,
            editable: true,
            marked_for_removal: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TagValue {
    Text(String),
    Integer(i64),
    Float(f64),
    Rational(u32, u32),
    DateTime(String),
    Gps(f64, f64, Option<f64>),
    Binary(Vec<u8>),
    Unknown(String),
}

impl Eq for TagValue {}

impl fmt::Display for TagValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(v) => write!(f, "{v}"),
            Self::Integer(v) => write!(f, "{v}"),
            Self::Float(v) => write!(f, "{v}"),
            Self::Rational(n, d) => write!(f, "{n}/{d}"),
            Self::DateTime(v) => write!(f, "{v}"),
            Self::Gps(lat, lon, alt) => {
                if let Some(altitude) = alt {
                    write!(f, "{lat:.6}, {lon:.6} @ {altitude:.2}m")
                } else {
                    write!(f, "{lat:.6}, {lon:.6}")
                }
            }
            Self::Binary(v) => write!(f, "<{} bytes>", v.len()),
            Self::Unknown(v) => write!(f, "{v}"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum TagCategory {
    Camera,
    Capture,
    Location,
    DateTime,
    Image,
    Description,
    Software,
    Other,
}

impl TagCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Camera => "Camera",
            Self::Capture => "Capture",
            Self::Location => "Location",
            Self::DateTime => "Date/Time",
            Self::Image => "Image",
            Self::Description => "Description",
            Self::Software => "Software",
            Self::Other => "Other",
        }
    }
}
