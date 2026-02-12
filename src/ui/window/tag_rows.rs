use super::*;

impl ExifEditorWindow {
    pub(super) fn ensure_tag_rows(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Lazily create the metadata filter input
        if self.metadata_filter_input.is_none() {
            let input = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Filter metadata...")
                    .default_value("")
            });
            let sub = cx.subscribe(&input, |this, entity, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.metadata_filter = entity.read(cx).value().to_string();
                    cx.notify();
                }
            });
            self.metadata_filter_input = Some(input);
            self.metadata_filter_subscription = Some(sub);
        }

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

    pub(super) fn build_tag_row(
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

    pub(super) fn commit_scalar_from_input(
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

    pub(super) fn commit_rational_from_inputs(
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

    pub(super) fn commit_gps_from_inputs(
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

    pub(super) fn read_rational_inputs(&self, row_id: &str, cx: &Context<Self>) -> Option<(String, String)> {
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

    pub(super) fn read_gps_inputs(
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

    pub(super) fn set_row_error(&mut self, row_id: &str, error: Option<String>) {
        if let Some(row) = self.tag_rows.iter_mut().find(|row| row.row_id == row_id) {
            row.parse_error = error;
        }
    }

    pub(super) fn clear_row(&mut self, tag_key: &str, cx: &mut Context<Self>) {
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

    pub(super) fn available_addable_tags(&self) -> Vec<&'static AddableTagDef> {
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
}
