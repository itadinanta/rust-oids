use conrod::{self, event, widget, Colorable, Positionable, Sizeable, Widget};
use conrod::widget::primitive::text::Style;
use std::io;
use super::{Error, Screen, theme};
use super::conrod_gfx;
use core::resource::ResourceLoader;
use gfx::{Encoder, Factory, Resources, CommandBuffer};
use gfx::handle::{ShaderResourceView, RenderTargetView};
use frontend::render::formats;


#[derive(Clone, Debug)]
pub struct WidgetIdGroup {
	panel_row_id: widget::Id,
	panel_id: widget::Id,
	label_id: widget::Id,
	value_id: widget::Id,
}

#[derive(Clone, Debug)]
pub struct Ids {
	help_canvas: widget::Id,
	help_text: widget::Id,

	hud_canvas: widget::Id,
	hud_labels: Vec<WidgetIdGroup>,
}

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
						ids: &Ids) -> conrod::UiCell<'e> {
		let mut widgets = ui.set_widgets();

		match self {
			&Screen::Help => {
				let help_text = "Text help";
				widget::Canvas::new()
					.pad(10.0)
					.color(conrod::color::CHARCOAL.alpha(0.4))
					.middle_of(root_window_id)
					.scroll_kids_vertically()
					.set(ids.help_canvas, &mut widgets);

				widget::Text::new(&help_text)
					.middle_of(ids.help_canvas)
					.with_style(style_label)
					.set(ids.help_text, &mut widgets);
			}
			&Screen::Main(ref frame_update) => {
				let splits = ids.hud_labels.iter()
					.map(|&WidgetIdGroup { panel_row_id, .. }|
						(panel_row_id, widget::Canvas::new().color(conrod::color::TRANSPARENT)))
					.collect::<Vec<_>>();
				;
				widget::Canvas::new()
					.pad(50.0)
					.color(conrod::color::TRANSPARENT)
					.kid_area_w_of(root_window_id)
					.mid_top()
					.flow_down(&splits)
					.set(ids.hud_canvas, &mut widgets);
				let mut ids_iter = ids.hud_labels.iter();
				let mut txt_with_label = |label: &str, value: &str| {
					let WidgetIdGroup { panel_id, label_id, value_id, panel_row_id } = ids_iter.next().unwrap().clone();

					widget::Canvas::new()
						.mid_left_of(panel_row_id)
						.pad(10.0)
						.color(conrod::color::CHARCOAL.alpha(0.4))
						.w(300.0)
						.h(60.0)
						.set(panel_id, &mut widgets);

					widget::Text::new(label)
						.mid_left_of(panel_id)
						.with_style(style_label)
						.set(label_id, &mut widgets);

					widget::Text::new(value)
						.mid_right_of(panel_id)
						.with_style(style_value)
						.set(value_id, &mut widgets);
				};


				txt_with_label("Sim Frames", &format!("{}", frame_update.simulation.count));
				txt_with_label("Vid Frames", &format!("{}", frame_update.count));
				txt_with_label("Elapsed", &format!("{:.3}", frame_update.elapsed));
				txt_with_label("Sim dt", &format!("{:.3}", frame_update.simulation.dt));
				txt_with_label("Vid dt", &format!("{:.3}", frame_update.dt));
				txt_with_label(">>", &format!("x{}", frame_update.speed_factor));
				txt_with_label("Avg dt", &format!("{:.3}", frame_update.duration_smooth));
				txt_with_label("FPS", &format!("{:.1}", frame_update.fps));
				txt_with_label("Population", &format!("{}", frame_update.simulation.population));
				txt_with_label("Extinctions", &format!("{}", frame_update.simulation.extinctions));
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
			color: Some(conrod::color::LIGHT_GRAY),
			font_size: Some(14),
			..Default::default()
		};
		let style_value = Style {
			color: Some(conrod::color::GREEN),
			font_size: Some(14),
			..Default::default()
		};

		let ids = Ids {
			help_canvas: ui.widget_id_generator().next(),
			help_text: ui.widget_id_generator().next(),

			hud_canvas: ui.widget_id_generator().next(),
			hud_labels: (0..10)
				.map(|_| {
					WidgetIdGroup {
						panel_row_id: ui.widget_id_generator().next(),
						panel_id: ui.widget_id_generator().next(),
						label_id: ui.widget_id_generator().next(),
						value_id: ui.widget_id_generator().next(),
					}
				})
				.collect()
		};

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
										  &self.ids.clone());
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