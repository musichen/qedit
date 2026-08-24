use crate::TitleBar;
use editor::{Editor, EditorEvent};
use gpui::{App, Context, Window};
use image_viewer::ImageView;
use language::LineEnding;
use project::image_store::{ImageFormat, ImageMetadata};
use settings::Settings;

impl TitleBar {
    pub(crate) fn refresh_status_data(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let active_item = self
            .active_pane
            .as_ref()
            .and_then(|pane| pane.read(cx).active_item());

        self.native_toolbar_state.status_encoding = None;
        self.native_toolbar_state.status_line_ending = None;
        self.native_toolbar_state.status_toolchain = None;
        self.native_toolbar_state.status_image_info = None;
        self.native_toolbar_state.active_editor_subscription = None;
        self.native_toolbar_state.active_image_subscription = None;

        if let Some(item) = active_item {
            if let Some(editor) = item.act_as::<Editor>(cx) {
                self.native_toolbar_state.active_editor_subscription =
                    Some(
                        cx.subscribe_in(&editor, window, |_this, _editor, event, _window, cx| {
                            if matches!(
                                event,
                                EditorEvent::SelectionsChanged { .. } | EditorEvent::BufferEdited
                            ) {
                                cx.notify();
                            }
                        }),
                    );

                let (encoding, line_ending) = editor.update(cx, |editor, cx| {
                    let mut encoding = None;
                    let mut line_ending = None;

                    if let Some(buffer) = editor.buffer().read(cx).as_singleton() {
                        let buffer = buffer.read(cx);
                        let active_encoding = buffer.encoding();
                        if active_encoding != encoding_rs::UTF_8 || buffer.has_bom() {
                            let mut text = active_encoding.name().to_string();
                            if buffer.has_bom() {
                                text.push_str(" (BOM)");
                            }
                            encoding = Some(text);
                        }

                        let current_line_ending = buffer.line_ending();
                        if current_line_ending != LineEnding::Unix {
                            line_ending = Some(current_line_ending.label().to_string());
                        }
                    }

                    (encoding, line_ending)
                });

                self.native_toolbar_state.status_encoding = encoding;
                self.native_toolbar_state.status_line_ending = line_ending;
            }

            if let Some(toolchain) = self.right_item_view::<toolchain_selector::ActiveToolchain>() {
                self.native_toolbar_state.status_toolchain = toolchain
                    .read(cx)
                    .active_toolchain_name()
                    .map(ToOwned::to_owned);
            }

            if let Some(image_view) = item.act_as::<ImageView>(cx) {
                if let Some(metadata) = image_view.read(cx).image_metadata(cx) {
                    self.native_toolbar_state.status_image_info =
                        Some(Self::format_image_metadata(&metadata, cx));
                } else {
                    self.native_toolbar_state.active_image_subscription =
                        Some(cx.observe(&image_view, |title_bar, image_view, cx| {
                            if let Some(metadata) = image_view.read(cx).image_metadata(cx) {
                                title_bar.native_toolbar_state.status_image_info =
                                    Some(Self::format_image_metadata(&metadata, cx));
                                cx.notify();
                            }
                        }));
                }
            }
        }
    }

    fn format_image_metadata(metadata: &ImageMetadata, cx: &App) -> String {
        let settings = image_viewer::ImageViewerSettings::get_global(cx);
        let mut components = Vec::new();
        components.push(format!("{}x{}", metadata.width, metadata.height));
        components.push(util::size::format_file_size(
            metadata.file_size,
            matches!(settings.unit, image_viewer::ImageFileSizeUnit::Decimal),
        ));
        components.push(
            match metadata.format {
                ImageFormat::Png => "PNG",
                ImageFormat::Jpeg => "JPEG",
                ImageFormat::Gif => "GIF",
                ImageFormat::WebP => "WebP",
                ImageFormat::Tiff => "TIFF",
                ImageFormat::Bmp => "BMP",
                ImageFormat::Ico => "ICO",
                ImageFormat::Avif => "Avif",
                _ => "Unknown",
            }
            .to_string(),
        );
        components.join(" • ")
    }
}
