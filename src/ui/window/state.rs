use super::*;

impl MetaStripWindow {
    pub(super) fn new(focus_handle: FocusHandle) -> Self {
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
            metadata_filter: String::new(),
            metadata_filter_input: None,
            metadata_filter_subscription: None,
        }
    }


    pub(super) fn on_root_key_down(
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
}
