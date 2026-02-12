use super::*;

impl ExifEditorWindow {
    pub(super) fn render_tag_field(&self, row: &TagEditorRow, cx: &mut Context<Self>) -> Field {
        let label = row.display_name.clone();

        let editor = match &row.kind {
            TagEditorKind::Scalar {
                scalar_kind, input, ..
            } => {
                let tag_key_for_clear = row.tag_key.clone();
                let input_widget = Input::new(input).w_full().suffix(
                    Button::new((ElementId::from("clear-inline"), row.row_id.clone()))
                        .ghost()
                        .xsmall()
                        .icon(IconName::CircleX)
                        .tab_stop(false)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.clear_row(&tag_key_for_clear, cx);
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
                    .child(div().text_sm().text_color(cx.theme().muted_foreground).child("/"))
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
                            .text_color(cx.theme().muted_foreground)
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
            let error_color = cx.theme().danger_foreground;
            field = field.description_fn(move |_, _| {
                div().text_color(error_color).child(error_text.clone())
            });
        }

        field
    }

    pub(super) fn render_metadata_editor(&self, cx: &mut Context<Self>) -> AnyElement {
    let query = self.metadata_filter.trim().to_ascii_lowercase();
    let fields: Vec<Field> = self
        .tag_rows
        .iter()
        .filter(|row| {
            if query.is_empty() {
                return true;
            }
            row.display_name.to_ascii_lowercase().contains(&query)
                || row.tag_key.to_ascii_lowercase().contains(&query)
        })
        .map(|row| self.render_tag_field(row, cx))
        .collect();

    let has_photo = self.state.active_photo.is_some();

    let filter_input = self.metadata_filter_input.clone();

        div()
        .id(SharedString::from("metadata-pane"))
        .w_1_3()
        .h_full()
        .bg(cx.theme().background)
        .border_1()
        .border_color(cx.theme().border)
        .child(
            v_flex()
                .h_full()
                .w_full()
                .child(
                    div()
                        .w_full()
                        .p_2()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .child(
                            if let Some(ref input) = filter_input {
                                Input::new(input)
                                    .w_full()
                                    .small()
                                    .prefix(IconName::Search)
                                    .into_any_element()
                            } else {
                                div().into_any_element()
                            },
                        ),
                )
                .child(
                    div()
                        .id(SharedString::from("metadata-scroll"))
                        .flex_1()
                        .w_full()
                        .overflow_y_scrollbar()
                        .p_2()
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
                ),
        )
        .into_any_element()
    }
}
