use conrod;
use super::conrod_gfx;

struct Ui {

}

impl Ui {
	fn new() {
		let mut ui_renderer = ui::conrod_gfx::Renderer::new(&mut factory, &frame_buffer, window.hidpi_factor() as f64).unwrap();
		let ui_image_map = conrod::image::Map::new();

		impl From<conrod::text::font::Error> for ui::Error {
			fn from(_: conrod::text::font::Error) -> ui::Error {
				ui::Error::FontLoader
			}
		}

		impl From<io::Error> for ui::Error {
			fn from(_: io::Error) -> ui::Error {
				ui::Error::ResourceLoader
			}
		}

		let mut ui = conrod::UiBuilder::new([w as f64, h as f64]).theme(ui::theme::default_theme()).build();

		fn load_font<R>(res: &R, map: &mut conrod::text::font::Map, key: &str) ->
		Result<conrod::text::font::Id, ui::Error>
			where R: ResourceLoader<u8> {
			let font_bytes = res.load(key)?;
			let font_collection = conrod::text::FontCollection::from_bytes(font_bytes);
			let default_font = font_collection.into_font().ok_or(ui::Error::FontLoader)?;
			let id = map.insert(default_font);
			Ok(id)
		}

		load_font(&res, &mut ui.fonts, "fonts/FreeSans.ttf")
			.expect("Could not find default font");
	}
}