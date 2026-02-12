use super::*;

impl MetaStripWindow {
    pub(super) fn open_map_popup_for_row(&mut self, row_id: &str, tag_key: &str, cx: &mut Context<Self>) {
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

    pub(super) fn close_map_popup(&mut self, cx: &mut Context<Self>) {
        self.map_popup = None;
        cx.notify();
    }

    pub(super) fn open_map_in_browser(&mut self, cx: &mut Context<Self>) {
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

    pub(super) fn open_add_tag_popup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    pub(super) fn close_add_tag_popup(&mut self, cx: &mut Context<Self>) {
        self.add_tag_popup_open = false;
        self.add_tag_search.clear();
        self.add_tag_search_input = None;
        self.add_tag_search_subscription = None;
        cx.notify();
    }

    pub(super) fn add_tag_from_popup(&mut self, key: &str, cx: &mut Context<Self>) {
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

    pub(super) fn render_add_tag_popup(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.add_tag_popup_open {
            return None;
        }

        let available = self.available_addable_tags();

        let tag_list: Vec<AnyElement> = available
            .iter()
            .map(|def| {
                let key = def.key;
                let list_hover_bg = cx.theme().list_hover;
                h_flex()
                    .id(SharedString::from(format!("add-tag-{key}")))
                    .w_full()
                    .px_3()
                    .py_1()
                    .gap_3()
                    .items_center()
                    .cursor_pointer()
                    .hover(move |style| style.bg(list_hover_bg))
                    .rounded_sm()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.add_tag_from_popup(key, cx);
                    }))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(cx.theme().foreground)
                            .child(def.display_name),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(def.category.as_str()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
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
                .bg(cx.theme().background)
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
                        .bg(cx.theme().popover)
                        .border_1()
                        .border_color(cx.theme().border)
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
                                        .text_color(cx.theme().foreground)
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
                                        .bg(cx.theme().secondary)
                                        .border_1()
                                        .border_color(cx.theme().border)
                                        .rounded_sm()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
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
                                                .text_color(cx.theme().muted_foreground)
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
                                .text_color(cx.theme().muted_foreground)
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

    pub(super) fn open_datetime_popup(
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

        let initial_date = NaiveDate::from_ymd_opt(
            yr.trim().parse().unwrap_or(2025),
            mo.trim().parse().unwrap_or(1),
            dy.trim().parse().unwrap_or(1),
        )
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());

        let date_picker = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx)
                .date_format("%Y:%m:%d");
            picker.set_date(initial_date, window, cx);
            picker
        });

        let hour = cx.new(|cx| InputState::new(window, cx).default_value(hr));
        let minute = cx.new(|cx| InputState::new(window, cx).default_value(mi));
        let second = cx.new(|cx| InputState::new(window, cx).default_value(se));

        self.datetime_popup = Some(DateTimePopupState {
            tag_key: tag_key.to_string(),
            date_picker,
            hour,
            minute,
            second,
        });

        cx.notify();
    }

    pub(super) fn close_datetime_popup(&mut self, cx: &mut Context<Self>) {
        self.datetime_popup = None;
        cx.notify();
    }

    pub(super) fn commit_datetime_popup(&mut self, cx: &mut Context<Self>) {
        let Some(photo_index) = self.state.active_photo else {
            self.datetime_popup = None;
            cx.notify();
            return;
        };

        let Some(popup) = &self.datetime_popup else {
            return;
        };

        let picker_state = popup.date_picker.read(cx);
        let date = match picker_state.date() {
            Date::Single(Some(d)) => d,
            _ => {
                self.status = "No date selected".to_string();
                self.datetime_popup = None;
                cx.notify();
                return;
            }
        };

        let yr = date.year();
        let mo = date.month();
        let dy = date.day();
        let hr = popup.hour.read(cx).value().to_string();
        let mi = popup.minute.read(cx).value().to_string();
        let se = popup.second.read(cx).value().to_string();

        let formatted = format!(
            "{:04}:{:02}:{:02} {:0>2}:{:0>2}:{:0>2}",
            yr,
            mo,
            dy,
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

    pub(super) fn render_datetime_popup(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let popup = self.datetime_popup.as_ref()?;

        Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bottom_0()
                .bg(cx.theme().background)
                .opacity(0.96)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    v_flex()
                        .w(px(440.0))
                        .p_4()
                        .gap_3()
                        .bg(cx.theme().popover)
                        .border_1()
                        .border_color(cx.theme().border)
                        .rounded_md()
                        .child(
                            div()
                                .text_lg()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(cx.theme().foreground)
                                .child("Set Date & Time"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("Tag: {}", popup.tag_key)),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Date"))
                                .child(DatePicker::new(&popup.date_picker).w(px(200.0))),
                        )
                        .child(
                            h_flex()
                                .w_full()
                                .gap_2()
                                .items_end()
                                .child(
                                    v_flex()
                                        .gap_1()
                                        .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Hour"))
                                        .child(Input::new(&popup.hour).w(px(52.0))),
                                )
                                .child(
                                    v_flex()
                                        .gap_1()
                                        .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Min"))
                                        .child(Input::new(&popup.minute).w(px(52.0))),
                                )
                                .child(
                                    v_flex()
                                        .gap_1()
                                        .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Sec"))
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
    pub(super) fn render_map_popup(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let popup = self.map_popup.as_ref()?;
        let fallback_text_color = cx.theme().muted_foreground;

        Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bottom_0()
                .bg(cx.theme().background)
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
                        .bg(cx.theme().popover)
                        .border_1()
                        .border_color(cx.theme().border)
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
                                .bg(cx.theme().secondary)
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded_sm()
                                .overflow_hidden()
                                .child(
                                    img(popup.static_map_url())
                                        .w_full()
                                        .h_full()
                                        .object_fit(ObjectFit::Cover)
                                        .with_fallback(move || {
                                            div()
                                                .size_full()
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .text_sm()
                                                .text_color(fallback_text_color)
                                                .child("Map preview unavailable")
                                                .into_any_element()
                                        }),
                                ),
                        )
                        .child(
                            div()
                                .p_2()
                                .bg(cx.theme().secondary)
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded_sm()
                                .child(popup.osm_url()),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
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
