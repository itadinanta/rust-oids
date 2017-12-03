use conrod;
use conrod::event;
use conrod::widget;
use std::io;
use app;
use super::Error;
use super::conrod_gfx;
use super::theme;
use core::resource::ResourceLoader;
use gfx::{Encoder, Factory, Resources, CommandBuffer};
use gfx::handle::{ShaderResourceView, RenderTargetView};
use frontend::render::formats;

pub struct Ui<'f, R, F>
	where
		R: Resources,
		F: Factory<R> + 'f,
{
	factory: &'f mut F,
	renderer: conrod_gfx::Renderer<R>,
	ui: Box<conrod::Ui>,
	image_map: conrod::image::Map<(ShaderResourceView<R, [f32; 4]>, (u32, u32))>,
	hidpi_factor: f64,
	events: Vec<event::Input>,
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

impl<'f, R, F> Ui<'f, R, F> where
	R: Resources,
	F: Factory<R> + 'f, {
	pub fn new<'e, L>(res: &L,
	                  factory: &'e mut F,
	                  frame_buffer: &RenderTargetView<R, formats::ScreenColorFormat>,
	                  hidpi_factor: f64) -> Result<Ui<'f, R, F>, Error>
		where L: ResourceLoader<u8>, 'e: 'f {
		let renderer = conrod_gfx::Renderer::new(factory, frame_buffer, hidpi_factor).unwrap();
		let image_map = conrod::image::Map::new();
		let (w, h, _, _) = frame_buffer.get_dimensions();
		let mut ui = conrod::UiBuilder::new([w as f64, h as f64]).theme(theme::default_theme()).build();

		Self::load_font(res, &mut ui.fonts, "fonts/FreeSans.ttf")?;

		Ok(Ui {
			factory,
			renderer,
			ui: Box::new(ui),
			image_map,
			hidpi_factor,
			events: Vec::new(),
		})
	}

	pub fn resize_to(&mut self,
	                 frame_buffer: &RenderTargetView<R, formats::ScreenColorFormat>)
	                 -> Result<(), Error> {
		let hidpi_factor = self.hidpi_factor;
		self.renderer = conrod_gfx::Renderer::new(self.factory, frame_buffer, hidpi_factor).unwrap();
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

	pub fn draw_screen<C>(&mut self, screen: &Screen, encoder: &mut Encoder<R, C>)
		where C: CommandBuffer<R> {
		let dims = (self.ui.win_w as f32, self.ui.win_h as f32);
		let window_id = self.ui.window.clone();

		let widgets = Self::draw_screen_widgets(&mut self.ui, window_id, screen);
		let primitives = widgets.draw();
		self.renderer.fill(encoder, dims, primitives, &self.image_map);
		self.renderer.draw(self.factory, encoder, &self.image_map);
	}

	fn draw_screen_widgets<'e>(ui: &'e mut conrod::Ui, window_id: widget::Id, screen: &Screen) -> conrod::UiCell<'e> {
		use conrod::{self, widget, Colorable, Positionable, Widget};
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
				let ids = Ids::new(ui.widget_id_generator());
				let mut widgets = ui.set_widgets();

				widget::Canvas::new()
					.pad(10.0)
					.color(conrod::color::CHARCOAL.alpha(0.4))
					.middle_of(window_id)
					.scroll_kids_vertically()
					.set(ids.canvas, &mut widgets);

				widget::Text::new(&frame_info)
					.mid_bottom_of(ids.canvas)
					.color(conrod::color::WHITE)
					.font_size(20)
					.set(ids.text, &mut widgets);

				widgets
			}
		}
	}

	pub fn push_event(&mut self, event: event::Input) {
		self.events.push(event);
	}

	pub fn handle_events(&mut self) {
		for event in &self.events {
			self.ui.handle_event(event.clone())
		}
		self.events.clear();
	}
}