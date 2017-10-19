pub mod formats;
mod effects;
mod forward;

use std::clone::Clone;
use core::resource::ResourceLoader;

use core::geometry::M44;
use core::geometry::Position;

use cgmath;
use frontend::render::forward::Vertex;

use std::convert;
use std::fmt;
use std::result;

use gfx;
use gfx::Factory;
use gfx::traits::FactoryExt;
use gfx_text;


pub struct Appearance {
	color: formats::Rgba,
	effect: [f32; 4],
}

impl Appearance {
	pub fn new(color: formats::Rgba, effect: [f32; 4]) -> Self {
		Appearance {
			color: color,
			effect: effect,
		}
	}

	pub fn rgba(color: formats::Rgba) -> Self {
		Appearance {
			color: color,
			effect: [1., 0., 0., 0.],
		}
	}
}


// pub type GFormat = Rgba;

pub const BACKGROUND: formats::Rgba = [0.01, 0.01, 0.01, 1.0];

const QUAD_VERTICES: [Vertex; 4] = [
	Vertex {
		pos: [-1.0, -1.0, 0.0],
		normal: [0.0, 0.0, 1.0],
		tangent: [1.0, 0.0, 0.0],
		tex_coord: [0.0, 0.0],
	},
	Vertex {
		pos: [1.0, -1.0, 0.0],
		normal: [0.0, 0.0, 1.0],
		tangent: [1.0, 0.0, 0.0],
		tex_coord: [1.0, 0.0],
	},
	Vertex {
		pos: [1.0, 1.0, 0.0],
		normal: [0.0, 0.0, 1.0],
		tangent: [1.0, 0.0, 0.0],
		tex_coord: [1.0, 1.0],
	},
	Vertex {
		pos: [-1.0, 1.0, 0.0],
		normal: [0.0, 0.0, 1.0],
		tangent: [1.0, 0.0, 0.0],
		tex_coord: [0.0, 1.0],
	},
];

const QUAD_INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

const BASE_VERTICES: [Vertex; 3] = [
	Vertex {
		pos: [0.0, 0.0, 0.0],
		normal: [0.0, 0.0, 1.0],
		tangent: [1.0, 0.0, 0.0],
		tex_coord: [0.5, 0.5],
	},
	Vertex {
		pos: [1.0, 0.0, 0.0],
		normal: [0.0, 0.0, 1.0],
		tangent: [1.0, 0.0, 0.0],
		tex_coord: [1.0, 0.5],
	},
	Vertex {
		pos: [0.0, 1.0, 0.0],
		normal: [0.0, 0.0, 1.0],
		tangent: [1.0, 0.0, 0.0],
		tex_coord: [0.5, 1.0],
	},
];

pub struct Camera {
	pub projection: M44,
	pub view: M44,
}

impl Camera {
	pub fn ortho(center: Position, scale: f32, ratio: f32) -> Camera {
		Camera {
			projection: {
				let hw = 0.5 * scale;
				let hh = hw / ratio;
				let near = 10.0;
				let far = -near;
				cgmath::ortho(-hw, hw, -hh, hh, near, far)
			}.into(),
			view: cgmath::Matrix4::look_at(
				cgmath::Point3::new(center.x, center.y, 1.0),
				cgmath::Point3::new(center.x, center.y, 0.0),
				cgmath::Vector3::unit_y(),
			).into(),
		}
	}
}

#[derive(Debug)]
pub enum RenderError {
	Shader(String),
	TextRenderer,
}

pub type Result<T> = result::Result<T, RenderError>;

impl<T: fmt::Display> convert::From<T> for RenderError {
	fn from(e: T) -> Self {
		RenderError::Shader(e.to_string())
	}
}

trait RenderFactoryExt<R: gfx::Resources>: gfx::traits::FactoryExt<R> {
	fn create_shader_set_with_geometry(&mut self, gs_code: &[u8], vs_code: &[u8], ps_code: &[u8])
		-> Result<gfx::ShaderSet<R>> {
		let gs = self.create_shader_geometry(gs_code)?;
		let vs = self.create_shader_vertex(vs_code)?;
		let ps = self.create_shader_pixel(ps_code)?;
		Ok(gfx::ShaderSet::Geometry(vs, gs, ps))
	}

	fn create_msaa_surfaces(&mut self, width: gfx::texture::Size, height: gfx::texture::Size)
		-> Result<formats::RenderSurfaceWithDepth<R>> {
		let (_, color_resource, color_target) = self.create_msaa_render_target(
			formats::MSAA_MODE,
			width,
			height,
		)?;
		let (_, _, depth_target) = self.create_msaa_depth(formats::MSAA_MODE, width, height)?;
		Ok((color_resource, color_target, depth_target))
	}

	fn create_msaa_depth(&mut self, aa: gfx::texture::AaMode, width: gfx::texture::Size, height: gfx::texture::Size)
		-> Result<formats::DepthSurface<R>> {
		let kind = gfx::texture::Kind::D2(width, height, aa);
		let tex = self.create_texture(
			kind,
			1,
			gfx::SHADER_RESOURCE | gfx::DEPTH_STENCIL,
			gfx::memory::Usage::Data,
			Some(gfx::format::ChannelType::Float),
		)?;
		let resource = self.view_texture_as_shader_resource::<formats::RenderDepthFormat>(
			&tex,
			(0, 0),
			gfx::format::Swizzle::new(),
		)?;
		let target = self.view_texture_as_depth_stencil_trivial(&tex)?;
		Ok((tex, resource, target))
	}

	fn create_msaa_render_target(&mut self, aa: gfx::texture::AaMode, width: gfx::texture::Size, height: gfx::texture::Size)
		-> Result<formats::RenderSurface<R>> {
		let kind = gfx::texture::Kind::D2(width, height, aa);
		let tex = self.create_texture(
			kind,
			1,
			gfx::SHADER_RESOURCE | gfx::RENDER_TARGET,
			gfx::memory::Usage::Data,
			Some(gfx::format::ChannelType::Float),
		)?;
		let hdr_srv = self.view_texture_as_shader_resource::<formats::RenderColorFormat>(
			&tex,
			(0, 0),
			gfx::format::Swizzle::new(),
		)?;
		let hdr_color_buffer = self.view_texture_as_render_target(&tex, 0, None)?;
		Ok((tex, hdr_srv, hdr_color_buffer))
	}
}

impl<R: gfx::Resources, E: gfx::traits::FactoryExt<R>> RenderFactoryExt<R> for E {}

pub trait Draw {
	fn draw_triangle(&mut self, transform: &cgmath::Matrix4<f32>, p: &[Position], appearance: &Appearance);
	fn draw_quad(&mut self, transform: &cgmath::Matrix4<f32>, ratio: f32, appearance: &Appearance);
	fn draw_star(&mut self, transform: &cgmath::Matrix4<f32>, vertices: &[Position], appearance: &Appearance);
	fn draw_lines(&mut self, transform: &cgmath::Matrix4<f32>, vertices: &[Position], appearance: &Appearance);
	fn draw_debug_lines(&mut self, transform: &cgmath::Matrix4<f32>, vertices: &[Position], appearance: &Appearance);
	fn draw_ball(&mut self, transform: &cgmath::Matrix4<f32>, appearance: &Appearance);
	fn draw_text(&mut self, text: &str, screen_position: [i32; 2], text_color: formats::Rgba);
}

pub trait Renderer<R: gfx::Resources, C: gfx::CommandBuffer<R>>: Draw {
	fn setup_frame(
		&mut self, camera: &Camera, background_color: formats::Rgba, light_color: formats::Rgba,
		light_position: &[Position]
	);
	fn begin_frame(&mut self);
	fn resolve_frame_buffer(&mut self);
	fn end_frame<D: gfx::Device<Resources = R, CommandBuffer = C>>(&mut self, device: &mut D);
	fn cleanup<D: gfx::Device<Resources = R, CommandBuffer = C>>(&mut self, device: &mut D);
}

pub struct ForwardRenderer<'e, 'l, R: gfx::Resources, C: 'e + gfx::CommandBuffer<R>, F: gfx::Factory<R>, L: 'l + ResourceLoader<u8>> {
	factory: F,
	encoder: &'e mut gfx::Encoder<R, C>,

	res: &'l L,

	frame_buffer: gfx::handle::RenderTargetView<R, formats::ScreenColorFormat>,
	depth: gfx::handle::DepthStencilView<R, formats::RenderDepthFormat>,

	hdr_srv: gfx::handle::ShaderResourceView<R, formats::Rgba>,
	hdr_color: gfx::handle::RenderTargetView<R, formats::RenderColorFormat>,

	_quad_vertices: gfx::handle::Buffer<R, Vertex>,
	quad_indices: gfx::Slice<R>,

	base_vertices: gfx::handle::Buffer<R, Vertex>,
	base_indices: gfx::Slice<R>,

	text_renderer: gfx_text::Renderer<R, F>,
	pass_forward_lighting: forward::ForwardLighting<R, C>,
	pass_effects: effects::PostLighting<R, C>,

	background_color: formats::Rgba,
}

impl<'e, 'l, R: gfx::Resources, C: gfx::CommandBuffer<R>, F: Factory<R> + Clone, L: ResourceLoader<u8>>
	ForwardRenderer<'e, 'l, R, C, F, L> {
	pub fn new(
		factory: &mut F, encoder: &'e mut gfx::Encoder<R, C>, res: &'l L,
		frame_buffer: &gfx::handle::RenderTargetView<R, formats::ScreenColorFormat>
	) -> Result<ForwardRenderer<'e, 'l, R, C, F, L>> {
		let my_factory = factory.clone();
		let (quad_vertices, quad_indices) = factory.create_vertex_buffer_with_slice(&QUAD_VERTICES, &QUAD_INDICES[..]);
		let (base_vertices, base_indices) = factory.create_vertex_buffer_with_slice(&BASE_VERTICES, ());

		let (w, h, _, _) = frame_buffer.get_dimensions();
		let (hdr_srv, hdr_color_buffer, depth_buffer) = factory.create_msaa_surfaces(w, h)?;

		let forward = forward::ForwardLighting::new(factory, res)?;
		let effects = effects::PostLighting::new(factory, res, w, h)?;
		let text_renderer = gfx_text::new(factory.clone()).build().map_err(|_| {
			RenderError::TextRenderer
		})?;

		Ok(ForwardRenderer {
			factory: my_factory,
			res: res,
			encoder: encoder,
			hdr_srv: hdr_srv,
			hdr_color: hdr_color_buffer,
			depth: depth_buffer,
			frame_buffer: frame_buffer.clone(),
			text_renderer: text_renderer,
			_quad_vertices: quad_vertices,
			quad_indices: quad_indices,
			base_vertices: base_vertices,
			base_indices: base_indices,
			pass_forward_lighting: forward,
			pass_effects: effects,
			background_color: BACKGROUND,
            /* 			light_color: BLACK,
                                                      * 			light_position: cgmath::Vector2::new(0.0, 0.0), */
		})
	}

	pub fn rebuild(&mut self) -> Result<()> {
		let factory = &mut self.factory;

		let (w, h, _, _) = self.frame_buffer.get_dimensions();
		let pass_forward_lighting = forward::ForwardLighting::new(factory, self.res)?;
		let pass_effects = effects::PostLighting::new(factory, self.res, w, h)?;
		self.pass_forward_lighting = pass_forward_lighting;
		self.pass_effects = pass_effects;
		Ok(())
	}

	pub fn resize_to(&mut self, frame_buffer: &gfx::handle::RenderTargetView<R, formats::ScreenColorFormat>)
		-> Result<()> {
		// TODO: this thing leaks?
		let (w, h, _, _) = frame_buffer.get_dimensions();
		let (hdr_srv, hdr_color_buffer, depth_buffer) = self.factory.create_msaa_surfaces(w, h)?;
		self.hdr_srv = hdr_srv;
		self.hdr_color = hdr_color_buffer;
		self.depth = depth_buffer;
		self.frame_buffer = frame_buffer.clone();
		self.pass_effects = effects::PostLighting::new(&mut self.factory, self.res, w, h)?;
		Ok(())
	}
}

impl<'e, 'l, R: gfx::Resources, C: gfx::CommandBuffer<R>, F: Factory<R>, L: ResourceLoader<u8>> Draw
	for ForwardRenderer<'e, 'l, R, C, F, L> {
	fn draw_star(&mut self, transform: &cgmath::Matrix4<f32>, vertices: &[Position], appearance: &Appearance) {
		let mut v: Vec<_> = vertices
			.iter()
			.map(|v| {
				Vertex {
					pos: [v.x, v.y, 0.0],
					normal: [0.0, 0.0, 1.0],
					tangent: [1.0, 0.0, 0.0],
					tex_coord: [0.5 + v.x * 0.5, 0.5 + v.y * 0.5],
				}
			})
			.collect();
		let n = v.len();
		v.push(Vertex::default());

		// TODO: these can be cached
		let mut i: Vec<u16> = Vec::new();
		for k in 0..n {
			i.push(n as u16);
			i.push(((k + 1) % n) as u16);
			i.push(k as u16);
		}

		let (vertex_buffer, index_buffer) = self.factory.create_vertex_buffer_with_slice(
			v.as_slice(),
			i.as_slice(),
		);

		self.pass_forward_lighting.draw_primitives(
			forward::Shader::Wireframe,
			&mut self.encoder,
			vertex_buffer,
			&index_buffer,
			&transform,
			appearance.color,
			appearance.effect,
			&mut self.hdr_color,
			&mut self.depth,
		);
	}

	fn draw_lines(&mut self, transform: &cgmath::Matrix4<f32>, vertices: &[Position], appearance: &Appearance) {
		let v: Vec<_> = vertices
			.iter()
			.map(|v| {
				Vertex {
					pos: [v.x, v.y, 0.0],
					normal: [0.0, 0.0, 1.0],
					tangent: [1.0, 0.0, 0.0],
					tex_coord: [0.5, 0.5],
				}
			})
			.collect();
		let (vertex_buffer, index_buffer) = self.factory.create_vertex_buffer_with_slice(
			v.as_slice(),
			(),
		);

		self.pass_forward_lighting.draw_primitives(
			forward::Shader::Lines,
			&mut self.encoder,
			vertex_buffer,
			&index_buffer,
			&transform,
			appearance.color,
			appearance.effect,
			&mut self.hdr_color,
			&mut self.depth,
		);
	}

	fn draw_debug_lines(&mut self, transform: &cgmath::Matrix4<f32>, vertices: &[Position], appearance: &Appearance) {
		let v: Vec<_> = vertices
			.iter()
			.map(|v| {
				Vertex {
					pos: [v.x, v.y, 0.0],
					normal: [0.0, 0.0, 1.0],
					tangent: [1.0, 0.0, 0.0],
					tex_coord: [0.5, 0.5],
				}
			})
			.collect();
		let (vertex_buffer, index_buffer) = self.factory.create_vertex_buffer_with_slice(
			v.as_slice(),
			(),
		);

		self.pass_forward_lighting.draw_primitives(
			forward::Shader::DebugLines,
			&mut self.encoder,
			vertex_buffer,
			&index_buffer,
			&transform,
			appearance.color,
			appearance.effect,
			&mut self.hdr_color,
			&mut self.depth,
		);
	}

	fn draw_ball(&mut self, transform: &cgmath::Matrix4<f32>, appearance: &Appearance) {
		self.pass_forward_lighting.draw_primitives(
			forward::Shader::Ball,
			&mut self.encoder,
			self.base_vertices.clone(),
			&self.base_indices,
			&transform,
			appearance.color,
			appearance.effect,
			&mut self.hdr_color,
			&mut self.depth,
		);
	}

	fn draw_quad(&mut self, transform: &cgmath::Matrix4<f32>, ratio: f32, appearance: &Appearance) {
		let v = &[
			Vertex {
				pos: [-ratio, -1.0, 0.0],
				normal: [0.0, 0.0, 1.0],
				tangent: [1.0, 0.0, 0.0],
				tex_coord: [0.5 - ratio * 0.5, 0.0],
			},
			Vertex {
				pos: [ratio, -1.0, 0.0],
				normal: [0.0, 0.0, 1.0],
				tangent: [1.0, 0.0, 0.0],
				tex_coord: [0.5 + ratio * 0.5, 0.0],
			},
			Vertex {
				pos: [ratio, 1.0, 0.0],
				normal: [0.0, 0.0, 1.0],
				tangent: [1.0, 0.0, 0.0],
				tex_coord: [0.5 + ratio * 0.5, 1.0],
			},
			Vertex {
				pos: [-ratio, 1.0, 0.0],
				normal: [0.0, 0.0, 1.0],
				tangent: [1.0, 0.0, 0.0],
				tex_coord: [0.5 - ratio * 0.5, 1.0],
			},
		];

		let vertex_buffer = self.factory.create_vertex_buffer(v);

		self.pass_forward_lighting.draw_primitives(
			forward::Shader::Flat,
			&mut self.encoder,
			vertex_buffer,
			&self.quad_indices,
			&transform,
			appearance.color,
			appearance.effect,
			&mut self.hdr_color,
			&mut self.depth,
		);
	}

	fn draw_triangle(&mut self, transform: &cgmath::Matrix4<f32>, p: &[Position], appearance: &Appearance) {
		if p.len() >= 3 {
			let v = &[
				Vertex {
					pos: [p[0].x, p[0].y, 0.0],
					tex_coord: [0.5 + p[0].x * 0.5, 0.5 + p[0].y * 0.5],
					..Vertex::default()
				},
				Vertex {
					pos: [p[1].x, p[1].y, 0.0],
					tex_coord: [0.5 + p[1].x * 0.5, 0.5 + p[1].y * 0.5],
					..Vertex::default()
				},
				Vertex {
					pos: [p[2].x, p[2].y, 0.0],
					tex_coord: [0.5 + p[2].x * 0.5, 0.5 + p[2].y * 0.5],
					..Vertex::default()
				},
			];

			let (vertices, indices) = self.factory.create_vertex_buffer_with_slice(v, ());

			self.pass_forward_lighting.draw_primitives(
				forward::Shader::Wireframe,
				&mut self.encoder,
				vertices,
				&indices,
				transform,
				appearance.color,
				appearance.effect,
				&mut self.hdr_color,
				&mut self.depth,
			);
		}
	}

	fn draw_text(&mut self, text: &str, screen_position: [i32; 2], text_color: formats::Rgba) {
		self.text_renderer.add(&text, screen_position, text_color);
		self.text_renderer
			.draw(&mut self.encoder, &mut self.frame_buffer)
			.expect("Failed to write text");
	}
}

impl<'e, 'l, R: gfx::Resources, C: 'e + gfx::CommandBuffer<R>, F: Factory<R>, L: ResourceLoader<u8>> Renderer<R, C>
	for ForwardRenderer<'e, 'l, R, C, F, L> {
	fn setup_frame(
		&mut self, camera: &Camera, background_color: formats::Rgba, light_color: formats::Rgba,
		light_position: &[Position]
	) {
		self.background_color = background_color;
		// 		self.light_color = light_color;
		// 		self.light_position = light_position;
		let mut lights: Vec<forward::PointLight> = Vec::new();

		lights.push(forward::PointLight {
			propagation: [0.3, 0.5, 0.4, 0.0],
			center: [-15.0, -5.0, 1.0, 1.0],
			color: [0.3, 0.0, 0.0, 1.0],
		});
		for p in light_position {
			lights.push(forward::PointLight {
				propagation: [0.2, 0.8, 0.1, 0.1],
				center: [p.x, p.y, 2.0, 1.0],
				color: light_color,
			});
		}

		self.pass_forward_lighting.setup(
			&mut self.encoder,
			camera.projection,
			camera.view,
			&lights,
		);
	}

	fn begin_frame(&mut self) {
		self.encoder.clear(&self.hdr_color, self.background_color);
		self.encoder.clear_depth(&self.depth, 1.0f32);
		self.encoder.clear(
			&self.frame_buffer,
			self.background_color,
		);
	}

	fn resolve_frame_buffer(&mut self) {
		self.pass_effects.apply_all(
			&mut self.encoder,
			self.hdr_srv.clone(),
			self.frame_buffer.clone(),
		);
	}

	fn end_frame<D: gfx::Device<Resources = R, CommandBuffer = C>>(&mut self, device: &mut D) {
		self.encoder.flush(device);
	}

	fn cleanup<D: gfx::Device<Resources = R, CommandBuffer = C>>(&mut self, device: &mut D) {
		device.cleanup();
	}
}
