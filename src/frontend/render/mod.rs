pub mod formats;
mod effects;
#[macro_use]
mod forward;

use std::clone::Clone;
use core::resource::ResourceLoader;

use core::geometry::M44;
use core::geometry::Position;

use cgmath;
use frontend::render::forward::VertexIndex;
use frontend::render::forward::PrimitiveIndex;
use frontend::render::forward::Vertex;

use std::convert;
use std::fmt;
use std::result;

use gfx;
use gfx::Factory;
use gfx::traits::FactoryExt;

#[derive(Clone, PartialEq)]
pub struct Appearance {
	color: formats::Rgba,
	effect: formats::Float4,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Style {
	Ball = 0,
	Flat = 1,
	Wireframe = 2,
	Lines = 3,
	DebugLines = 4,
	Count = 5,
}

impl Appearance {
	pub fn new(color: formats::Rgba, effect: formats::Float4) -> Self {
		Appearance {
			color,
			effect,
		}
	}

	pub fn rgba(color: formats::Rgba) -> Self {
		Appearance {
			color,
			effect: [1., 0., 0., 0.],
		}
	}
}

// pub type GFormat = Rgba;

pub const BACKGROUND: formats::Rgba = [0.01, 0.01, 0.01, 1.0];

const QUAD_VERTICES: [Vertex; 4] = [
	new_vertex!([-1.0, -1.0, 0.0], [0.0, 0.0]),
	new_vertex!([1.0, -1.0, 0.0], [1.0, 0.0]),
	new_vertex!([1.0, 1.0, 0.0], [1.0, 1.0]),
	new_vertex!([-1.0, 1.0, 0.0], [0.0, 1.0]),
];

const QUAD_INDICES: [VertexIndex; 6] = [0, 1, 2, 0, 2, 3];

const TRI_VERTICES: [Vertex; 3] = [
	new_vertex!([0.0, 0.0, 0.0], [0.5, 0.5]),
	new_vertex!([1.0, 0.0, 0.0], [1.0, 0.5]),
	new_vertex!([0.0, 1.0, 0.0], [0.5, 1.0]),
];

const TRI_INDICES: [VertexIndex; 3] = [0, 1, 2];

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
	ShaderMismatch(Style, Style),
	PrimitiveIndexOverflow,
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

#[derive(Clone)]
pub struct PrimitiveBatch {
	shader: Style,
	vertices: Vec<Vertex>,
	indices: Vec<VertexIndex>,
	transforms: Vec<M44>,
	appearances: Vec<Appearance>,
}

pub trait Draw {
	fn draw_triangle(&mut self, style: Option<Style>, transform: M44, p: &[Position], appearance: Appearance);
	fn draw_quad(&mut self, style: Option<Style>, transform: M44, ratio: f32, appearance: Appearance);
	fn draw_star(&mut self, style: Option<Style>, transform: M44, vertices: &[Position], appearance: Appearance);
	fn draw_lines(&mut self, style: Option<Style>, transform: M44, vertices: &[Position], appearance: Appearance);
	fn draw_ball(&mut self, style: Option<Style>, transform: M44, appearance: Appearance);
}

pub trait DrawBatch {
	fn draw_batch(&mut self, batch: PrimitiveBatch);
}

pub trait PrimitiveSequence {
	// Optimized batch
	fn push_batch(&mut self, batch: PrimitiveBatch) -> Result<()>;
	// Single entry.
	// TODO: do I want to maintain both?
	fn push_primitive(&mut self,
					  shader: Style,
					  vertices: Vec<Vertex>,
					  indices: Vec<VertexIndex>,
					  transform: M44,
					  appearance: Appearance) -> Result<()>;
}

impl<T> Draw for T where T: PrimitiveSequence {
	fn draw_star(&mut self, style: Option<Style>, transform: M44, vertices: &[Position], appearance: Appearance) {
		let mut v: Vec<_> = vertices
			.iter()
			.map(|v| Vertex::new([v.x, v.y, 0.0], [0.5 + v.x * 0.5, 0.5 + v.y * 0.5]))
			.collect();
		let n = v.len();
		v.push(Vertex::default());

		let mut i: Vec<VertexIndex> = Vec::new();
		for k in 0..n {
			i.push(n as VertexIndex);
			i.push(((k + 1) % n) as VertexIndex);
			i.push(k as VertexIndex);
		}

		self.push_primitive(style.unwrap_or(Style::Wireframe), v, i, transform, appearance)
			.expect("Unable to draw star")
	}

	fn draw_lines(&mut self, style: Option<Style>, transform: M44, vertices: &[Position], appearance: Appearance) {
		let v: Vec<_> = vertices
			.iter()
			.map(|v| Vertex::new([v.x, v.y, 0.0], [0.5, 0.5]))
			.collect();

		let i: Vec<_> = (0..vertices.len() as VertexIndex).collect();

		self.push_primitive(style.unwrap_or(Style::Lines), v, i, transform, appearance)
			.expect("Unable to draw lines");
	}

	fn draw_ball(&mut self, style: Option<Style>, transform: M44, appearance: Appearance) {
		self.push_primitive(style.unwrap_or(Style::Ball), TRI_VERTICES.to_vec(), TRI_INDICES.to_vec(), transform, appearance)
			.expect("Unable to draw ball");
	}

	fn draw_quad(&mut self, style: Option<Style>, transform: M44, ratio: f32, appearance: Appearance) {
		let v = vec![
			Vertex::new([-ratio, -1.0, 0.0], [0.5 - ratio * 0.5, 0.0]),
			Vertex::new([ratio, -1.0, 0.0], [0.5 + ratio * 0.5, 0.0]),
			Vertex::new([ratio, 1.0, 0.0], [0.5 + ratio * 0.5, 1.0]),
			Vertex::new([-ratio, 1.0, 0.0], [0.5 - ratio * 0.5, 1.0]),
		];

		self.push_primitive(style.unwrap_or(Style::Flat), v, QUAD_INDICES.to_vec(), transform, appearance)
			.expect("Unable to draw quad");
	}

	fn draw_triangle(&mut self, style: Option<Style>, transform: M44, p: &[Position], appearance: Appearance) {
		if p.len() >= 3 {
			let v = vec![
				Vertex::new([p[0].x, p[0].y, 0.0], [0.5 + p[0].x * 0.5, 0.5 + p[0].y * 0.5]),
				Vertex::new([p[1].x, p[1].y, 0.0], [0.5 + p[1].x * 0.5, 0.5 + p[1].y * 0.5]),
				Vertex::new([p[2].x, p[2].y, 0.0], [0.5 + p[2].x * 0.5, 0.5 + p[2].y * 0.5]),
			];

			let i = vec![0, 1, 2];

			self.push_primitive(style.unwrap_or(Style::Wireframe), v, i, transform, appearance)
				.expect("Unable to draw triangle");
		}
	}
}

pub struct PrimitiveBuffer {}

impl PrimitiveBatch {
	pub fn new() -> PrimitiveBatch {
		PrimitiveBatch {
			shader: Style::Flat,
			vertices: Vec::new(),
			indices: Vec::new(),
			transforms: Vec::new(),
			appearances: Vec::new(),
		}
	}
}

impl PrimitiveSequence for PrimitiveBatch {
	fn push_batch(&mut self, mut batch: PrimitiveBatch) -> Result<()> {
		self.push_primitive_buffers(batch.shader, batch.vertices, batch.indices)?;
		self.transforms.append(&mut batch.transforms);
		self.appearances.append(&mut batch.appearances);
		Ok(())
	}

	fn push_primitive(&mut self,
					  shader: Style,
					  vertices: Vec<Vertex>,
					  indices: Vec<VertexIndex>,
					  transform: M44,
					  appearance: Appearance) -> Result<()>
	{
		self.push_primitive_buffers(shader, vertices, indices)?;
		self.transforms.push(transform);
		self.appearances.push(appearance);
		Ok(())
	}
}

impl PrimitiveBatch {
	pub fn draw_primitives<E>(&self) -> Result<()> {
		Ok(())
	}

	fn push_primitive_buffers(&mut self,
							  shader: Style,
							  mut vertices: Vec<Vertex>,
							  mut indices: Vec<VertexIndex>) -> Result<()>
	{
		self.shader = shader;
		let primitive_offset = self.transforms.len();
		if primitive_offset > PrimitiveIndex::max_value() as usize {
			Err(RenderError::PrimitiveIndexOverflow)
		} else {
			let vertex_offset = self.vertices.len() as VertexIndex;
			for v in &mut vertices {
				v.primitive_index = primitive_offset as PrimitiveIndex;
			}
			for i in &mut indices {
				*i += vertex_offset;
			}
			self.indices.append(&mut indices);
			self.vertices.append(&mut vertices);
			Ok(())
		}
	}
}

pub trait Overlay<R, F, C>
	where R: gfx::Resources, C: gfx::CommandBuffer<R>, F: Factory<R> {
	fn overlay<O>(&mut self, callback: O) where O: FnMut(&mut F, &mut gfx::Encoder<R, C>);
}

pub trait Renderer<R: gfx::Resources, C: gfx::CommandBuffer<R>>: Draw {
	fn setup_frame(
		&mut self, camera: &Camera, background_color: formats::Rgba, light_color: formats::Rgba,
		light_position: &[Position],
	);
	fn begin_frame(&mut self);
	fn resolve_frame_buffer(&mut self);
	fn end_frame<D: gfx::Device<Resources=R, CommandBuffer=C>>(&mut self, device: &mut D);
	fn cleanup<D: gfx::Device<Resources=R, CommandBuffer=C>>(&mut self, device: &mut D);
}

pub struct ForwardRenderer<'e, 'l, R: gfx::Resources, C: 'e + gfx::CommandBuffer<R>, F: gfx::Factory<R>, L: 'l + ResourceLoader<u8>> {
	factory: F,
	pub encoder: &'e mut gfx::Encoder<R, C>,
	res: &'l L,
	frame_buffer: gfx::handle::RenderTargetView<R, formats::ScreenColorFormat>,
	depth: gfx::handle::DepthStencilView<R, formats::RenderDepthFormat>,
	hdr_srv: gfx::handle::ShaderResourceView<R, formats::Rgba>,
	hdr_color: gfx::handle::RenderTargetView<R, formats::RenderColorFormat>,
	pass_forward_lighting: forward::ForwardLighting<R, C, forward::ShadedInit<'static>>,
	pass_effects: effects::PostLighting<R, C>,
	background_color: formats::Rgba,
}

impl<'e, 'l, R: gfx::Resources, C: gfx::CommandBuffer<R>, F: Factory<R> + Clone, L: ResourceLoader<u8>>
ForwardRenderer<'e, 'l, R, C, F, L> {
	pub fn new(
		factory: &mut F, encoder: &'e mut gfx::Encoder<R, C>, res: &'l L,
		frame_buffer: &gfx::handle::RenderTargetView<R, formats::ScreenColorFormat>,
	) -> Result<ForwardRenderer<'e, 'l, R, C, F, L>> {
		let my_factory = factory.clone();

		let (w, h, _, _) = frame_buffer.get_dimensions();
		let (hdr_srv, hdr_color_buffer, depth_buffer) = factory.create_msaa_surfaces(w, h)?;

		let forward = forward::ForwardLighting::new(factory, res, forward::shaded::new())?;
		let effects = effects::PostLighting::new(factory, res, w, h)?;

		Ok(ForwardRenderer {
			factory: my_factory,
			res,
			encoder,
			hdr_srv,
			hdr_color: hdr_color_buffer,
			depth: depth_buffer,
			frame_buffer: frame_buffer.clone(),
			pass_forward_lighting: forward,
			pass_effects: effects,
			background_color: BACKGROUND,
		})
	}

	pub fn rebuild(&mut self) -> Result<()> {
		let factory = &mut self.factory;

		let (w, h, _, _) = self.frame_buffer.get_dimensions();
		let pass_forward_lighting = forward::ForwardLighting::new(factory, self.res, forward::shaded::new())?;
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

impl<'e, 'l, R: gfx::Resources, C: gfx::CommandBuffer<R>, F: Factory<R>, L: ResourceLoader<u8>> DrawBatch
for ForwardRenderer<'e, 'l, R, C, F, L> {
	fn draw_batch(&mut self, mut batch: PrimitiveBatch) {
		self.push_batch(batch)
			.expect("Could not draw batch");
	}
}

impl<'e, 'l, R: gfx::Resources, C: gfx::CommandBuffer<R>, F: Factory<R>, L: ResourceLoader<u8>> PrimitiveSequence
for ForwardRenderer<'e, 'l, R, C, F, L> {
	fn push_batch(&mut self, mut batch: PrimitiveBatch) -> Result<()> {
		let models: Vec<forward::ModelArgs> = batch.transforms.iter()
			.map(|transform| forward::ModelArgs { transform: (*transform).into() })
			.collect();
		let materials: Vec<forward::MaterialArgs> = batch.appearances.iter()
			.map(|appearance| forward::MaterialArgs { emissive: appearance.color, effect: appearance.effect })
			.collect();
		let (vertex_buffer, index_buffer) = self.factory.create_vertex_buffer_with_slice(
			batch.vertices.as_slice(),
			batch.indices.as_slice(),
		);
		self.pass_forward_lighting.draw_primitives(
			batch.shader,
			&mut self.encoder,
			vertex_buffer,
			&index_buffer,
			&models,
			&materials,
			&mut self.hdr_color,
			&mut self.depth,
		)?;

		Ok(())
	}

	fn push_primitive(&mut self,
					  shader: Style,
					  vertices: Vec<Vertex>,
					  indices: Vec<VertexIndex>,
					  transform: M44,
					  appearance: Appearance) -> Result<()> {
		let models = vec![forward::ModelArgs { transform: transform.into() }];
		let materials = vec![forward::MaterialArgs { emissive: appearance.color, effect: appearance.effect }];
		let (vertex_buffer, index_buffer) = self.factory.create_vertex_buffer_with_slice(
			vertices.as_slice(),
			indices.as_slice(),
		);
		self.pass_forward_lighting.draw_primitives(
			shader,
			&mut self.encoder,
			vertex_buffer,
			&index_buffer,
			&models,
			&materials,
			&mut self.hdr_color,
			&mut self.depth,
		)?;

		Ok(())
	}
}

impl<'e, 'l, R: gfx::Resources, C: 'e + gfx::CommandBuffer<R>, F: Factory<R>, L: ResourceLoader<u8>> Renderer<R, C>
for ForwardRenderer<'e, 'l, R, C, F, L> {
	fn setup_frame(
		&mut self, camera: &Camera, background_color: formats::Rgba, light_color: formats::Rgba,
		light_position: &[Position],
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
		).expect("Unable to setup lighting");
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

	fn end_frame<D: gfx::Device<Resources=R, CommandBuffer=C>>(&mut self, device: &mut D) {
		self.encoder.flush(device);
	}

	fn cleanup<D: gfx::Device<Resources=R, CommandBuffer=C>>(&mut self, device: &mut D) {
		device.cleanup();
	}
}

impl<'e, 'l, R: gfx::Resources, C: 'e + gfx::CommandBuffer<R>, F: Factory<R>, L: ResourceLoader<u8>> Overlay<R, F, C>
for ForwardRenderer<'e, 'l, R, C, F, L> {
	fn overlay<O>(&mut self, mut callback: O)
		where O: FnMut(&mut F, &mut gfx::Encoder<R, C>),
			  F: Factory<R> {
		callback(&mut self.factory, &mut self.encoder)
	}
}