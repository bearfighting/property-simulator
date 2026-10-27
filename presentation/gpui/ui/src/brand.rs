use gpui_kit::{Image, ImageFormat};
use std::sync::{Arc, LazyLock};

static MARK: LazyLock<Arc<Image>> = LazyLock::new(|| {
    Arc::new(Image::from_bytes(
        ImageFormat::Png,
        include_bytes!("../../../../assets/brand/prop-orbit-mark.png").to_vec(),
    ))
});

pub(crate) fn mark() -> Arc<Image> {
    Arc::clone(&MARK)
}
