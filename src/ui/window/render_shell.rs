use super::*;

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
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.render_left_pane(cx))
            .child(Divider::vertical().color(cx.theme().border))
            .child(self.render_metadata_editor(cx))
            .children(self.render_map_popup(cx))
            .children(self.render_add_tag_popup(cx))
            .children(self.render_datetime_popup(cx))
    }
}
