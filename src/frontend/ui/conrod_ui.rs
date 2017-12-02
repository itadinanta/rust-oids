use conrod;
use std::io;
use std::borrow::BorrowMut;
use app;
use super::Error;
use super::conrod_gfx;
use super::theme;
use core::resource::ResourceLoader;
use gfx::{Encoder, Factory, Resources, CommandBuffer};
use gfx::handle::{ShaderResourceView, RenderTargetView};
use frontend::render::formats;
use conrod::event;

pub struct Ui<R: Resources> {
	renderer: conrod_gfx::Renderer<R>,
	ui: Box<conrod::Ui>,
	image_map: conrod::image::Map<(ShaderResourceView<R, [f32; 4]>, (u32, u32))>,
	hidpi_factor: f64,
}

impl From<conrod::text::font::Error> for Error {
	fn from(_: conrod::text::font::Error) -> Error {
		Error::FontLoader
	}
}

impl From<io::Error> for Error {
	fn from(_: io::Error) -> Error {
		Error::ResourceLoader
	}
}

#[derive(Clone)]
pub enum Screen {
	Main(app::FrameUpdate),
}

impl<R> Ui<R> where R: Resources {
	pub fn new<F, L>(res: &L,
					 factory: &mut F,
					 frame_buffer: &RenderTargetView<R, formats::ScreenColorFormat>,
					 hidpi_factor: f64) -> Result<Ui<R>, Error>
		where F: Factory<R>,
			  L: ResourceLoader<u8> {
		let renderer = conrod_gfx::Renderer::new(factory, frame_buffer, hidpi_factor).unwrap();
		let image_map = conrod::image::Map::new();
		let (w, h, _, _) = frame_buffer.get_dimensions();
		let mut ui = conrod::UiBuilder::new([w as f64, h as f64]).theme(theme::default_theme()).build();

		Self::load_font(res, &mut ui.fonts, "fonts/FreeSans.ttf")?;

		Ok(Ui {
			renderer,
			ui: Box::new(ui),
			image_map,
			hidpi_factor,
		})
	}

	pub fn resize_to<F>(&mut self, frame_buffer: &RenderTargetView<R, F>)
						-> Result<(), Error> {
		let (w, h, _, _) = frame_buffer.get_dimensions();
		self.ui = Box::new(conrod::UiBuilder::new([w as f64, h as f64]).theme(theme::default_theme()).build());
		Ok(())
	}

	fn load_font<L>(res: &L, map: &mut conrod::text::font::Map, key: &str) ->
	Result<conrod::text::font::Id, Error>
		where L: ResourceLoader<u8> {
		let font_bytes = res.load(key)?;
		let font_collection = conrod::text::FontCollection::from_bytes(font_bytes);
		let default_font = font_collection.into_font().ok_or(Error::FontLoader)?;
		let id = map.insert(default_font);
		Ok(id)
	}

	pub fn draw_screen<C, F>(&mut self, screen: &Screen, factory: &mut F, encoder: &mut Encoder<R, C>)
		where F: Factory<R>, C: CommandBuffer<R> {
		use conrod::{self, widget, Colorable, Positionable, Widget};
		let dims = (self.ui.win_w as f32, self.ui.win_h as f32);
		match screen {
			&Screen::Main(ref frame_update) => {
				let frame_info = format!(
					"F: {},{} E: {:.3} FT: {:.3},{:.3}(x{}) SFT: {:.3} FPS: {:.1} P: {} X: {}",
					frame_update.simulation.count,
					frame_update.count,
					frame_update.elapsed,
					frame_update.simulation.dt,
					frame_update.dt,
					frame_update.speed_factor,
					frame_update.duration_smooth,
					frame_update.fps,
					frame_update.simulation.population,
					frame_update.simulation.extinctions,
				);
				widget_ids!( struct Ids { text, canvas, rounded_rectangle });
				let ids = Ids::new(self.ui.widget_id_generator());

				let window_id = self.ui.window.clone();
				let ui = &mut self.ui.set_widgets();

				widget::Canvas::new()
					.pad(10.0)
					.color(conrod::color::CHARCOAL.alpha(0.4))
					.middle_of(window_id)
					.scroll_kids_vertically()
					.set(ids.canvas, ui);

				let full_width = ui.w_of(window_id).unwrap_or_default();
				widget::RoundedRectangle::fill([full_width, 100.0], 5.0)
					.color(conrod::color::BLACK.alpha(0.5))
					.middle_of(ids.canvas)
					.set(ids.rounded_rectangle, ui);

				widget::Text::new(&frame_info)
					.middle_of(ids.rounded_rectangle)
					.color(conrod::color::WHITE)
					.font_size(20)
					.set(ids.text, ui);

				let primitives = ui.draw();
				self.renderer.fill(encoder, dims, primitives, &self.image_map);
				self.renderer.draw(factory, encoder, &self.image_map);
			}
		}
	}

	pub fn handle_event(&mut self, event: event::Input) {
		self.ui.handle_event(event)
	}
}