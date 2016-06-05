use gfx;
use gfx::traits::FactoryExt;

pub static VERTEX_SRC: &'static [u8] = b"

#version 150 core

in vec2 a_Pos;
in vec2 a_TexCoord;
out vec2 v_TexCoord;

void main() {
    v_TexCoord = a_TexCoord;
    gl_Position = vec4(a_Pos, 0.0, 1.0);
}

";

pub static SIMPLE_BLIT_SRC: &'static [u8] = b"

#version 150 core

uniform sampler2D t_Source;

in vec2 v_TexCoord;
out vec4 o_Color;

void main() {
	o_Color = texture(t_Source, v_TexCoord, 0);
}

";

pub static EXPOSURE_BLIT_SRC: &'static [u8] = b"

#version 150 core

layout (std140) uniform cb_FragmentArgs {
    float u_Exposure;
};

uniform sampler2D t_Source;

in vec2 v_TexCoord;
out vec4 o_Color;

void main() {
	o_Color = u_Exposure * texture(t_Source, v_TexCoord, 0);
}

";

pub static DOWNSAMPLE_COLOR_SRC: &'static [u8] = b"

#version 150 core

uniform sampler2D t_Source;

in vec2 v_TexCoord;
out vec4 Target0;

void main() {
	float x1 = v_TexCoord.x * 2;
	float x2 = x1 + 1;
	float y1 = v_TexCoord.y * 2;
	float y2 = x1 + 1;
    Target0 = (texelFetch(t_source, ivec2(x1, y1), 0)
	         + texelFetch(t_source, ivec2(x1, y2), 0)
	         + texelFetch(t_source, ivec2(x2, y1), 0)
	         + texelFetch(t_source, ivec2(x2, y2), 0)) / 4.0;
}

";

pub type HDRColorFormat = (gfx::format::R16_G16_B16_A16, gfx::format::Float);
pub type ColorFormat = gfx::format::Rgba8;

gfx_defines!{
	vertex BlitVertex {
		pos: [f32; 2] = "a_Pos",
		tex_coord: [f32; 2] = "a_TexCoord",
	}
	
	constant FragmentArgs {
        exposure: f32 = "u_Exposure",
    }
	
	pipeline blit {
		vbuf: gfx::VertexBuffer<BlitVertex> = (),
		fragment_args: gfx::ConstantBuffer<FragmentArgs> = "cb_FragmentArgs",
		src: gfx::TextureSampler<[f32; 4]> = "t_Source",
		dst: gfx::RenderTarget<ColorFormat> = "o_Color",
	}
}

use std::marker::PhantomData;
pub struct Blit<R: gfx::Resources, C: gfx::CommandBuffer<R>> {
	vertex_buffer: gfx::handle::Buffer<R, BlitVertex>,
	index_buffer_slice: gfx::Slice<R>,
	fragment_args: gfx::handle::Buffer<R, FragmentArgs>,
	sampler: gfx::handle::Sampler<R>,
	pso: gfx::pso::PipelineState<R, blit::Meta>,
	_buffer: PhantomData<C>,
}

impl<R: gfx::Resources, C: gfx::CommandBuffer<R>> Blit<R, C> {
	pub fn new<F>(factory: &mut F) -> Blit<R, C>
		where F: gfx::Factory<R> {

		let full_screen_triangle = vec![BlitVertex {
			                                pos: [-1., -1.],
			                                tex_coord: [0., 0.],
		                                },
		                                BlitVertex {
			                                pos: [-1., 3.],
			                                tex_coord: [0., 2.],
		                                },
		                                BlitVertex {
			                                pos: [3., -1.],
			                                tex_coord: [2., 0.],
		                                }];

		let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&full_screen_triangle, ());

		Blit {
			vertex_buffer: vertex_buffer,
			index_buffer_slice: slice,
			fragment_args: factory.create_constant_buffer(1),
			sampler: factory.create_sampler_linear(),
			pso: factory.create_pipeline_simple(VERTEX_SRC, EXPOSURE_BLIT_SRC, blit::new()).unwrap(),
			_buffer: PhantomData,
		}
	}
	pub fn tone_map(&self,
	                encoder: &mut gfx::Encoder<R, C>,
	                hdr_srv: gfx::handle::ShaderResourceView<R, [f32; 4]>,
	                color_target: gfx::handle::RenderTargetView<R, ColorFormat>) {

		encoder.update_constant_buffer(&self.fragment_args, &FragmentArgs { exposure: 3.0 });

		encoder.draw(&self.index_buffer_slice,
		             &self.pso,
		             &blit::Data {
			             vbuf: self.vertex_buffer.clone(),
			             fragment_args: self.fragment_args.clone(),
			             src: (hdr_srv.clone(), self.sampler.clone()),
			             dst: color_target.clone(),
		             });
	}
}
