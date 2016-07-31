mod effects;
mod forward;

use gfx;

extern crate cgmath;
extern crate gfx_text;

pub type HDRColorFormat = (gfx::format::R16_G16_B16_A16, gfx::format::Float);
pub type ColorFormat = gfx::format::Srgba8;
pub type DepthFormat = gfx::format::DepthStencil;

pub type GFormat = [f32; 4];

pub type M44 = cgmath::Matrix4<f32>;

pub const BACKGROUND: [f32; 4] = [0.01, 0.01, 0.01, 1.0];
pub const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

use self::forward::Vertex;

const QUAD_VERTICES: [Vertex; 4] = [Vertex {
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
                                    }];

const QUAD_INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

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
	fn draw_star(&mut self, transform: &cgmath::Matrix4<f32>, vertices: &[cgmath::Vector2<f32>], color: [f32; 4]);
	fn draw_ball(&mut self, transform: &cgmath::Matrix4<f32>, color: [f32; 4]);
	fn draw_text(&mut self, text: &str, screen_position: [i32; 2], text_color: [f32; 4]);
}

pub trait Renderer<R: gfx::Resources, C: gfx::CommandBuffer<R>>: Draw {
	fn setup(&mut self, camera: &Camera, background_color: [f32; 4], light_color: [f32; 4]);
	fn begin_frame(&mut self);
	fn resolve_frame_buffer(&mut self);
	fn end_frame<D: gfx::Device<Resources = R, CommandBuffer = C>>(&mut self, device: &mut D);
	fn cleanup<D: gfx::Device<Resources = R, CommandBuffer = C>>(&mut self, device: &mut D);
}

pub struct ForwardRenderer<'e, R: gfx::Resources, C: 'e + gfx::CommandBuffer<R>, F: gfx::Factory<R>> {
	factory: F,
	encoder: &'e mut gfx::Encoder<R, C>,

	frame_buffer: gfx::handle::RenderTargetView<R, ColorFormat>,
	depth: gfx::handle::DepthStencilView<R, DepthFormat>,

	hdr_srv: gfx::handle::ShaderResourceView<R, [f32; 4]>,
	hdr_color: gfx::handle::RenderTargetView<R, HDRColorFormat>,

	quad_vertices: gfx::handle::Buffer<R, Vertex>,
	quad_indices: gfx::Slice<R>,

	text_renderer: gfx_text::Renderer<R, F>,
	pass_forward_lighting: forward::ForwardLighting<R, C>,
	pass_effects: effects::PostLighting<R, C>,

	background_color: [f32; 4],
	light_color: [f32; 4],
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
		let my_factory = factory.clone();
		let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&QUAD_VERTICES, &QUAD_INDICES[..]);

		let (w, h, _, _) = frame_buffer.get_dimensions();

		let (_, hdr_srv, hdr_color_buffer) = factory.create_render_target(w, h).unwrap();

		ForwardRenderer {
			factory: my_factory,
			encoder: encoder,
			hdr_srv: hdr_srv,
			hdr_color: hdr_color_buffer,
			depth: depth.clone(),
			frame_buffer: frame_buffer.clone(),
			text_renderer: gfx_text::new(factory.clone()).build().unwrap(),
			quad_vertices: vertex_buffer,
			quad_indices: slice,
			pass_forward_lighting: forward::ForwardLighting::new(factory),
			pass_effects: effects::PostLighting::new(factory, w, h),
			background_color: BACKGROUND,
			light_color: BLACK,
		}
	}

	pub fn resize_to(&mut self,
	                 frame_buffer: &gfx::handle::RenderTargetView<R, ColorFormat>,
	                 depth: &gfx::handle::DepthStencilView<R, DepthFormat>) {
		// TODO: this thing leaks?

		let factory = &mut self.factory;

		let (w, h, _, _) = frame_buffer.get_dimensions();
		let (_, hdr_srv, hdr_color_buffer) = factory.create_render_target(w, h).unwrap();

		self.hdr_srv = hdr_srv;
		self.hdr_color = hdr_color_buffer;
		self.depth = depth.clone();
		self.frame_buffer = frame_buffer.clone();
		self.pass_effects = effects::PostLighting::new(factory, w, h);
	}
}

impl<'e, R: gfx::Resources, C: gfx::CommandBuffer<R>, F: Factory<R>> Draw for ForwardRenderer<'e, R, C, F> {
	fn draw_star(&mut self, transform: &cgmath::Matrix4<f32>, vertices: &[cgmath::Vector2<f32>], color: [f32; 4]) {
		let mut v: Vec<_> = vertices.iter()
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
		v.push(Vertex {
			pos: [0.0, 0.0, 0.0],
			normal: [0.0, 0.0, 1.0],
			tangent: [1.0, 0.0, 0.0],
			tex_coord: [0.5, 0.5],
		});

		// TODO: these can be cached
		let mut i: Vec<u16> = Vec::new();
		for k in 0..n {
			i.push(n as u16);
			i.push(((k + 1) % n) as u16);
			i.push(k as u16);
		}

		let (vertex_buffer, index_buffer) = self.factory.create_vertex_buffer_with_slice(v.as_slice(), i.as_slice());

		self.pass_forward_lighting.draw_triangles(forward::Shader::Flat,
		                                          &mut self.encoder,
		                                          &vertex_buffer,
		                                          &index_buffer,
		                                          transform.into(),
		                                          color,
		                                          &mut self.hdr_color,
		                                          &mut self.depth);
	}

	fn draw_ball(&mut self, transform: &cgmath::Matrix4<f32>, color: [f32; 4]) {
		self.pass_forward_lighting.draw_triangles(forward::Shader::Ball,
		                                          &mut self.encoder,
		                                          &self.quad_vertices,
		                                          &self.quad_indices,
		                                          transform.into(),
		                                          color,
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
	fn setup(&mut self, camera: &Camera, background_color: [f32; 4], light_color: [f32; 4]) {
		self.background_color = background_color;
		self.light_color = light_color;
		let lights: Vec<forward::PointLight> = vec![forward::PointLight {
			                                            propagation: [0.3, 0.5, 0.4, 0.0],
			                                            center: [-15.0, -5.0, 1.0, 1.0],
			                                            color: [0.3, 0.0, 0.0, 1.0],
		                                            },
		                                            forward::PointLight {
			                                            propagation: [0.2, 0.8, 0.1, 0.1],
			                                            center: [10.0, 10.0, 2.0, 1.0],
			                                            color: self.light_color,
		                                            }];

		self.pass_forward_lighting.setup(&mut self.encoder, camera.projection, camera.view, &lights);
	}

	fn begin_frame(&mut self) {
		self.encoder.clear(&self.hdr_color, self.background_color);
		self.encoder.clear_depth(&self.depth, 1.0f32);
		self.encoder.clear(&self.frame_buffer, self.background_color);
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
