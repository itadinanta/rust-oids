mod effects;
mod forward;

use gfx;

extern crate cgmath;
extern crate gfx_text;

pub type HDRColorFormat = (gfx::format::R16_G16_B16_A16, gfx::format::Float);
pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

pub type GFormat = [f32; 4];

pub type M44 = cgmath::Matrix4<f32>;

pub const BRIGHT: [f32; 4] = [2.0, 3.0, 4.0, 1.0];
pub const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];


use self::forward::Vertex;

const QUAD: [Vertex; 6] = [Vertex {
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
	                           pos: [-1.0, -1.0, 0.0],
	                           normal: [0.0, 0.0, 1.0],
	                           tangent: [1.0, 0.0, 0.0],
	                           tex_coord: [0.0, 0.0],
                           },
                           Vertex {
	                           pos: [-1.0, 1.0, 0.0],
	                           normal: [0.0, 0.0, 1.0],
	                           tangent: [1.0, 0.0, 0.0],
	                           tex_coord: [0.0, 1.0],
                           },
                           Vertex {
	                           pos: [1.0, 1.0, 0.0],
	                           normal: [0.0, 0.0, 1.0],
	                           tangent: [1.0, 0.0, 0.0],
	                           tex_coord: [1.0, 1.0],
                           }];

pub struct Camera {
	pub projection: M44,
	pub view: M44,
}

impl Camera {
	pub fn ortho(center: cgmath::Point2<f32>, scale: f32, ratio: f32) -> Camera {
		Camera {
			projection: {
					let hw = 0.5 * scale;
					let hh = hw / ratio;
					let near = 10.0;
					let far = -near;
					cgmath::ortho(-hw, hw, -hh, hh, near, far)
				}
				.into(),
			view: cgmath::Matrix4::look_at(cgmath::Point3::new(center.x, center.y, 1.0),
			                               cgmath::Point3::new(center.x, center.y, 0.0),
			                               cgmath::Vector3::unit_y())
				.into(),
		}
	}
}

pub trait Draw {
	fn draw_quad(&mut self, transform: &cgmath::Matrix4<f32>);
	fn draw_text(&mut self, text: &str, screen_position: [i32; 2], text_color: [f32; 4]);
}

pub trait Renderer<R: gfx::Resources, C: gfx::CommandBuffer<R>>: Draw {
	fn setup(&mut self, camera: &Camera);
	fn begin_frame(&mut self);
	fn resolve_frame_buffer(&mut self);
	fn end_frame<D: gfx::Device<Resources = R, CommandBuffer = C>>(&mut self, device: &mut D);
	fn cleanup<D: gfx::Device<Resources = R, CommandBuffer = C>>(&mut self, device: &mut D);
}

pub struct ForwardRenderer<'e, R: gfx::Resources, C: 'e + gfx::CommandBuffer<R>, F: gfx::Factory<R>> {
	encoder: &'e mut gfx::Encoder<R, C>,

	frame_buffer: gfx::handle::RenderTargetView<R, ColorFormat>,
	depth: gfx::handle::DepthStencilView<R, DepthFormat>,

	hdr_srv: gfx::handle::ShaderResourceView<R, [f32; 4]>,
	hdr_color: gfx::handle::RenderTargetView<R, HDRColorFormat>,

	vertex_buffer: gfx::handle::Buffer<R, Vertex>,
	index_buffer_slice: gfx::Slice<R>,

	text_renderer: gfx_text::Renderer<R, F>,
	pass_forward_lighting: forward::ForwardLighting<R, C>,
	pass_effects: effects::PostLighting<R, C>,
}

use gfx::Factory;
use gfx::traits::FactoryExt;

use std::clone::Clone;
impl<'e, R: gfx::Resources, C: gfx::CommandBuffer<R>, F: Factory<R> + Clone> ForwardRenderer<'e, R, C, F> {
	pub fn new(factory: &mut F,
	           encoder: &'e mut gfx::Encoder<R, C>,
	           frame_buffer: &gfx::handle::RenderTargetView<R, ColorFormat>,
	           depth: &gfx::handle::DepthStencilView<R, DepthFormat>)
	           -> ForwardRenderer<'e, R, C, F>
		where F: Factory<R> {
		let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&QUAD, ());

		let (w, h, _, _) = frame_buffer.get_dimensions();

		let (_, hdr_srv, hdr_color_buffer) = factory.create_render_target(w, h).unwrap();

		ForwardRenderer {
			encoder: encoder,
			hdr_srv: hdr_srv,
			hdr_color: hdr_color_buffer,
			depth: depth.clone(),
			frame_buffer: frame_buffer.clone(),
			text_renderer: gfx_text::new(factory.clone()).build().unwrap(),
			vertex_buffer: vertex_buffer,
			index_buffer_slice: slice,
			pass_forward_lighting: forward::ForwardLighting::new(factory),
			pass_effects: effects::PostLighting::new(factory, w, h),
		}
	}
}

impl<'e, R: gfx::Resources, C: gfx::CommandBuffer<R>, F: Factory<R>> Draw for ForwardRenderer<'e, R, C, F> {
	fn draw_quad(&mut self, transform: &cgmath::Matrix4<f32>) {
		self.pass_forward_lighting.draw_triangles(&mut self.encoder,
		                                          &self.vertex_buffer,
		                                          &self.index_buffer_slice,
		                                          transform.into(),
		                                          &mut self.hdr_color,
		                                          &mut self.depth);
	}

	fn draw_text(&mut self, text: &str, screen_position: [i32; 2], text_color: [f32; 4]) {
		self.text_renderer.add(&text, screen_position, text_color);
		self.text_renderer.draw(&mut self.encoder, &mut self.frame_buffer).unwrap();
	}
}

impl<'e, R: gfx::Resources, C: 'e + gfx::CommandBuffer<R>, F: Factory<R>> Renderer<R, C> for ForwardRenderer<'e,
                                                                                                             R,
                                                                                                             C,
                                                                                                             F> {
	fn setup(&mut self, camera: &Camera) {
		let lights: Vec<forward::PointLight> = vec![forward::PointLight {
			                                            propagation: [0.3, 0.5, 0.4, 0.0],
			                                            center: [-15.0, -5.0, 1.0, 1.0],
			                                            color: [1.0, 0.0, 0.0, 1.0],
		                                            },
		                                            forward::PointLight {
			                                            propagation: [0.1, 0.7, 0.1, 0.1],
			                                            center: [10.0, 10.0, 2.0, 1.0],
			                                            color: [4., 4., 4., 1.],
		                                            }];

		self.pass_forward_lighting.setup(&mut self.encoder, camera.view, camera.projection, &lights);
	}

	fn begin_frame(&mut self) {
		self.encoder.clear(&self.hdr_color, BRIGHT);
		self.encoder.clear_depth(&self.depth, 1.0f32);
		self.encoder.clear(&self.frame_buffer, BLACK);
	}

	fn resolve_frame_buffer(&mut self) {
		self.pass_effects.apply_all(&mut self.encoder,
		                            self.hdr_srv.clone(),
		                            self.frame_buffer.clone());
	}

	fn end_frame<D: gfx::Device<Resources = R, CommandBuffer = C>>(&mut self, device: &mut D) {
		self.encoder.flush(device);
	}

	fn cleanup<D: gfx::Device<Resources = R, CommandBuffer = C>>(&mut self, device: &mut D) {
		device.cleanup();
	}
}
