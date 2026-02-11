use exif_editor::ui::window::open_metastrip_window;
use gpui::{App, Application};
use gpui_component_assets::Assets;

fn main() {
    Application::new().with_assets(Assets).run(|cx: &mut App| {
        gpui_component::init(cx);
        open_metastrip_window(cx);
        cx.activate(true);
    });
}
