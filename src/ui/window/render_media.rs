use super::*;

impl ExifEditorWindow {
    pub(super) fn render_upload_box(&self, cx: &mut Context<Self>) -> AnyElement {
        let drop_bg = cx.theme().drop_target;
        let drop_border = cx.theme().drag_border;
        div()
            .id(SharedString::from("upload-drop-zone"))
            .w_full()
            .h_full()
            .min_h(px(560.0))
            .p_4()
            .flex()
            .items_center()
            .justify_center()
            .bg(cx.theme().secondary)
            .border_1()
            .border_color(cx.theme().border)
            .can_drop(|value, _, _| value.is::<ExternalPaths>())
            .drag_over::<ExternalPaths>(move |style, _, _, _| {
                style.bg(drop_bg).border_color(drop_border)
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
                    .text_color(cx.theme().foreground)
                    .child(Icon::new(IconName::FolderOpen).large())
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Upload Photos"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("Drag photos here or click to browse"),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn render_carousel(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.state.photos.is_empty() {
            return self.render_upload_box(cx);
        }

        let active_index = self.state.active_photo.unwrap_or(0);
        let photo = &self.state.photos[active_index];
        let disable_nav = self.state.photos.len() <= 1;

        let drop_target = cx.theme().drop_target;

        v_flex()
            .flex_1()
            .w_full()
            .h_full()
            .gap_2()
            .p_2()
            .can_drop(|value, _, _| value.is::<ExternalPaths>())
            .drag_over::<ExternalPaths>(move |style, _, _, _| style.bg(drop_target))
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
                    .child(div().text_sm().text_color(cx.theme().muted_foreground).child(format!(
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
                div().w_full().flex_1().max_h(px(480.0)).child(
                    div()
                        .w_full()
                        .h_full()
                        .bg(cx.theme().muted)
                        .border_1()
                        .border_color(cx.theme().border)
                        .overflow_hidden()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            img(photo.path.clone())
                                .w_full()
                                .h_full()
                                .object_fit(ObjectFit::Contain)
                                .with_fallback(|| image_fallback("No preview available")),
                        ),
                ),
            )
            .child(
                h_flex().w_full().justify_start().child(
                    div()
                        .w_full()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(photo.filename.clone()),
                ),
            )
            .child(self.render_thumbnail_strip(cx))
            .into_any_element()
    }

    pub(super) fn render_thumbnail_strip(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id(SharedString::from("carousel-thumbnails"))
            .h(px(112.0))
            .w_full()
            .overflow_x_scrollbar()
            .child(h_flex().h_full().items_start().gap_2().pr_3().children(
                self.state.photos.iter().enumerate().map(|(index, photo)| {
                    let is_active = self.state.active_photo == Some(index);
                    let filename = photo.filename.clone();

                    let list_hover = cx.theme().list_hover;
                    let thumb_muted = cx.theme().muted;
                    let active_border = cx.theme().primary;
                    let inactive_border = cx.theme().border;

                    div()
                        .id(SharedString::from(format!("thumb-{index}")))
                        .w(px(96.0))
                        .h(px(96.0))
                        .flex_none()
                        .overflow_hidden()
                        .bg(thumb_muted)
                        .border_1()
                        .border_color(if is_active {
                            active_border
                        } else {
                            inactive_border
                        })
                        .cursor_pointer()
                        .hover(move |style| style.bg(list_hover))
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

    pub(super) fn render_action_row(&self, cx: &mut Context<Self>) -> AnyElement {
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
                Button::new("save-all")
                    .small()
                    .primary()
                    .icon(IconName::Check)
                    .label("Save All")
                    .disabled(!has_photos)
                    .on_click(cx.listener(|this, _, _, cx| this.save_all(cx))),
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
                Button::new("export-all")
                    .small()
                    .icon(IconName::ExternalLink)
                    .label("Export All")
                    .disabled(!has_photos)
                    .on_click(cx.listener(|this, _, _, cx| this.export_all(cx))),
            )
            .child(
                Button::new("clear-all-meta")
                    .small()
                    .danger()
                    .icon(IconName::Delete)
                    .label("Clear All")
                    .disabled(!has_photos)
                    .on_click(cx.listener(|this, _, _, cx| this.clear_all_metadata(cx))),
            )
            .child(div().flex_1())
            .child(
                Button::new("toggle-theme")
                    .ghost()
                    .small()
                    .icon(if cx.theme().mode == ThemeMode::Dark {
                        IconName::Sun
                    } else {
                        IconName::Moon
                    })
                    .on_click(cx.listener(|_this, _, window, cx| {
                        let new_mode = if cx.theme().mode == ThemeMode::Dark {
                            ThemeMode::Light
                        } else {
                            ThemeMode::Dark
                        };
                        Theme::change(new_mode, Some(window), cx);
                    })),
            )
            .into_any_element()
    }

    pub(super) fn render_left_pane(&self, cx: &mut Context<Self>) -> AnyElement {
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
            .bg(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border)
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
}
