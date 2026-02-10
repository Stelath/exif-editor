use serde::{Deserialize, Serialize};

use crate::models::TagCategory;

pub type PresetId = u64;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StripPreset {
    pub id: PresetId,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub rules: Vec<PresetRule>,
    pub is_builtin: bool,
}

impl StripPreset {
    pub fn new(
        id: PresetId,
        name: impl Into<String>,
        description: impl Into<String>,
        icon: impl Into<String>,
        rules: Vec<PresetRule>,
        is_builtin: bool,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            description: description.into(),
            icon: icon.into(),
            rules,
            is_builtin,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PresetRule {
    RemoveCategory(TagCategory),
    RemoveTag(String),
    RemoveAllExcept(Vec<String>),
    RemoveAll,
    RemoveGps,
    RemoveThumbnail,
    SetTag(String, String),
}
