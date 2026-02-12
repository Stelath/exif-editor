use exif_editor::http::ReqwestClient;
use exif_editor::ui::window::open_metastrip_window;
use gpui::{App, Application};
use gpui_component_assets::Assets;

fn main() {
    Application::new()
        .with_assets(Assets)
        .with_http_client(ReqwestClient::new())
        .run(|cx: &mut App| {
            gpui_component::init(cx);
            open_metastrip_window(cx);
            cx.activate(true);
        });
}
