use egui::{Color32, ColorImage, Context, TextureHandle, TextureOptions};
use qrcode::{Color, QrCode};

#[derive(Default)]
pub struct QrCache {
    url: String,
    texture: Option<TextureHandle>,
}

impl QrCache {
    pub fn texture(&mut self, ctx: &Context, url: &str) -> Option<&TextureHandle> {
        if self.url != url || self.texture.is_none() {
            self.url = url.to_string();
            self.texture = QrCode::new(url.as_bytes()).ok().map(|code| {
                let width = code.width();
                let quiet = 2;
                let size = width + quiet * 2;
                let mut pixels = vec![Color32::WHITE; size * size];
                for (i, color) in code.to_colors().into_iter().enumerate() {
                    if color == Color::Dark {
                        let (x, y) = (i % width + quiet, i / width + quiet);
                        pixels[y * size + x] = Color32::BLACK;
                    }
                }
                let image = ColorImage::new([size, size], pixels);
                ctx.load_texture("remote-qr", image, TextureOptions::NEAREST)
            });
        }
        self.texture.as_ref()
    }
}
