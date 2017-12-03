use conrod::{self, event, widget, Colorable, Positionable, Widget};
use conrod::widget::primitive::text::Style;
use std::io;
use super::{Error, Screen, theme};
use super::conrod_gfx;
use core::resource::ResourceLoader;
use gfx::{Encoder, Factory, Resources, CommandBuffer};
use gfx::handle::{ShaderResourceView, RenderTargetView};
use frontend::render::formats;

widget_ids!(
	#[derive(Clone)]
	struct Ids {
	// HELP
	help_text,

	// HUD
	text, canvas, rounded_rectangle,

	simulation_count_label,
	count_label,
	elapsed_label,
	simulation_dt_label,
	dt_label,
	speed_factor_label,
	duration_smooth_label,
	fps_label,
	simulation_population_label,
	simulation_extinctions_label,

	simulation_count_value,
	count_value,
	elapsed_value,
	simulation_dt_value,
	dt_value,
	speed_factor_value,
	duration_smooth_value,
	fps_value,
	simulation_population_value,
	simulation_extinctions_value,

});

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
	style_label: Style,
	style_value: Style,
	ids: Ids,
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

impl Screen {
	fn draw_widgets<'e>(&self,
						ui: &'e mut conrod::Ui,
						root_window_id: widget::Id,
						style_label: Style,
						style_value: Style,
						ids: Ids) -> conrod::UiCell<'e> {
		let mut widgets = ui.set_widgets();

		match self {
			&Screen::Help => {
				let help_text = "Text help";
				widget::Canvas::new()
					.pad(10.0)
					.color(conrod::color::CHARCOAL.alpha(0.4))
					.middle_of(root_window_id)
					.scroll_kids_vertically()
					.set(ids.canvas, &mut widgets);

				widget::Text::new(&help_text)
					.middle_of(ids.canvas)
					.with_style(style_label)
					.set(ids.text, &mut widgets);
			}
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
				let mut txt_with_label =
					|label_id: widget::id::Id,
					 value_id: widget::id::Id,
					 label: &str, value: &str| {
						widget::Text::new(label)
							.mid_bottom_of(root_window_id)
							.with_style(style_value)
							.set(label_id, &mut widgets);

						widget::Text::new(value)
							.mid_bottom_of(root_window_id)
							.with_style(style_value)
							.set(value_id, &mut widgets);
					};

				txt_with_label(ids.simulation_count_label, ids.simulation_count_value,
							   "Sim Frames:", &format!("{}", frame_update.simulation.count));
			}
		};
		widgets
	}
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


		let style_label = Style {
			color: Some(conrod::color::WHITE),
			font_size: Some(20),
			..Default::default()
		};
		let style_value = Style {
			color: Some(conrod::color::WHITE),
			font_size: Some(20),
			..Default::default()
		};

		let ids = Ids::new(ui.widget_id_generator());

		Ok(Ui {
			factory,
			renderer,
			ui: Box::new(ui),
			image_map,
			hidpi_factor,
			style_label,
			style_value,
			ids,
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
		let widgets = screen.draw_widgets(&mut self.ui,
										  window_id,
										  self.style_label.clone(),
										  self.style_value.clone(),
										  self.ids.clone());
		let primitives = widgets.draw();
		self.renderer.fill(encoder, dims, primitives, &self.image_map);
		self.renderer.draw(self.factory, encoder, &self.image_map);
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