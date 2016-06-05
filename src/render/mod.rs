mod blit;
mod forward;

use gfx;

extern crate cgmath;
extern crate gfx_text;

pub type HDRColorFormat = (gfx::format::R16_G16_B16_A16, gfx::format::Float);
pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

pub type GFormat = [f32; 4];

pub type M44 = cgmath::Matrix4<f32>;

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

pub trait Renderer<R: gfx::Resources, C: gfx::CommandBuffer<R>, D: gfx::Device<Resources = R, CommandBuffer = C>>
    : Draw {
	fn setup(&mut self, camera: &Camera);
	fn begin_frame(&mut self);
	fn end_frame(&mut self, device: &mut D);
	fn cleanup(&mut self, device: &mut D);
}

pub struct ForwardRenderer<'e, R: gfx::Resources, C: 'e + gfx::CommandBuffer<R>, F: gfx::Factory<R>> {
	encoder: &'e gfx::Encoder<R, C>,
	color: gfx::handle::RenderTargetView<R, ColorFormat>,
	depth: gfx::handle::DepthStencilView<R, DepthFormat>,

	vertex_buffer: gfx::handle::Buffer<R, Vertex>,
	index_buffer_slice: gfx::Slice<R>,

	text_renderer: gfx_text::Renderer<R, F>,
	pass_forward_lighting: forward::ForwardLighting<R, C>, // 	pass_blit: blit::Blit<R>,
}

use gfx::Factory;
use gfx::traits::FactoryExt;

pub impl<'e, R: gfx::Resources, C: gfx::CommandBuffer<R>, F: Factory<R>> ForwardRenderer<'e, R, C, F> {
	pub fn new(factory: &mut F,
	           encoder: &'e gfx::Encoder<R, C>,
	           color: &gfx::handle::RenderTargetView<R, ColorFormat>,
	           depth: &gfx::handle::DepthStencilView<R, DepthFormat>)
	           -> ForwardRenderer<'e, R, C, F> {
		let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&QUAD, ());

		let lights: Vec<forward::PointLight> = vec![forward::PointLight {
			                                            propagation: [0.3, 0.5, 0.4, 0.0],
			                                            center: [-15.0, -5.0, 1.0, 1.0],
			                                            color: [1.0, 0.0, 0.0, 1.0],
		                                            },
		                                            forward::PointLight {
			                                            propagation: [0.5, 0.5, 0.5, 0.0],
			                                            center: [10.0, 10.0, 2.0, 1.0],
			                                            color: [0.9, 0.9, 0.8, 1.0],
		                                            }];

		ForwardRenderer {
			encoder: encoder,
			color: color.clone(),
			depth: depth.clone(),
			text_renderer: gfx_text::new(*factory).build().unwrap(),
			vertex_buffer: vertex_buffer,
			index_buffer_slice: slice,
			pass_forward_lighting: forward::ForwardLighting::new(factory),
		}
	}
}

pub impl<'e, R: gfx::Resources, C: gfx::CommandBuffer<R>, F: gfx::Factory<R>> Draw for ForwardRenderer<'e, R, C, F> {
	pub fn draw_quad(&mut self, transform: &cgmath::Matrix4<f32>) {
		self.pass_forward_lighting.draw_triangles(&mut self.encoder,
		                                          &self.vertex_buffer,
		                                          &self.index_buffer_slice,
		                                          transform.into(),
		                                          &mut self.color,
		                                          &mut self.depth);
	}

	pub fn draw_text(&mut self, text: &str, screen_position: [i32; 2], text_color: [f32; 4]) {
		self.text_renderer.add(&text, screen_position, text_color);
		self.text_renderer.draw(&mut self.encoder, &mut self.color).unwrap();
	}
}

pub impl<'e, R: gfx::Resources,
		C: gfx::CommandBuffer<R>,
		F: gfx::Factory<R>,
		D: gfx::Device<Resources = R, CommandBuffer = C>> Renderer<R, C, D>
		for ForwardRenderer<'e, R, C, F> {
	pub fn setup(&mut self, camera: &Camera) {
		let lights: Vec<forward::PointLight> = vec![forward::PointLight {
			                                            propagation: [0.3, 0.5, 0.4, 0.0],
			                                            center: [-15.0, -5.0, 1.0, 1.0],
			                                            color: [1.0, 0.0, 0.0, 1.0],
		                                            },
		                                            forward::PointLight {
			                                            propagation: [0.5, 0.5, 0.5, 0.0],
			                                            center: [10.0, 10.0, 2.0, 1.0],
			                                            color: [0.9, 0.9, 0.8, 1.0],
		                                            }];

		self.pass_forward_lighting.setup(&mut self.encoder, camera.view, camera.projection, &lights);
	}

	pub fn begin_frame(&mut self) {
		self.encoder.clear(&self.color, BLACK);
	    self.encoder.clear_depth(&self.depth, 1.0f32);
	}


	pub fn end_frame(&mut self, device: &mut D) {
		self.encoder.flush(device);
	}

	pub fn cleanup(&mut self, device: &mut D) {
		device.cleanup();
	}
}
