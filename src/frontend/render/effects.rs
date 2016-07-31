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

pub static LUMINANCE_VERTEX_SRC: &'static [u8] = b"

#version 150 core

uniform sampler2D t_VertexLuminance;

layout (std140) uniform cb_VertexArgs {
    float u_White;
    float u_Black;
};

in vec2 a_Pos;
in vec2 a_TexCoord;
out vec2 v_TexCoord;
out float v_Exposure;

void main() {
    v_TexCoord = a_TexCoord;
    float luminance = dot(vec3(0.2126, 0.7152, 0.0722), texture(t_VertexLuminance, vec2(0.5, 0.5)).rgb);

	// todo: interpolate flat
    v_Exposure =  1.0 / (u_Black + (u_White * luminance));
    gl_Position = vec4(a_Pos, 0.0, 1.0);
}
";


pub static EXPOSURE_TONE_MAP_SRC: &'static [u8] = b"
#version 150 core

uniform sampler2D t_Source;

in vec2 v_TexCoord;
in float v_Exposure;
out vec4 o_Color;

void main() {
	vec4 linear_color = v_Exposure * texture(t_Source, v_TexCoord, 0);
	o_Color = vec4(linear_color.rgb, 1.0);
}
";

pub static EXPONENTIAL_SMOOTH_SRC: &'static [u8] = b"

#version 150 core

uniform sampler2D t_Value;
uniform sampler2D t_Acc;

layout (std140) uniform cb_FragmentArgs {
    float u_ExpAlpha;
};

in vec2 v_TexCoord;
out vec4 o_Smooth;

void main() {
	vec4 value = texture(t_Value, v_TexCoord, 0);
	vec4 acc = texture(t_Acc, v_TexCoord, 0);
	
	o_Smooth = max(u_ExpAlpha * value, vec4(0.)) + max((1. - u_ExpAlpha) * acc, vec4(0.));
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

// http://learnopengl.com/#!Advanced-Lighting/Bloom
pub static GAUSSIAN_BLUR_HORIZONTAL_SRC: &'static [u8] = b"

#version 150 core

uniform sampler2D t_Source;

in vec2 v_TexCoord;
out vec4 o_Color;

const float weight[5] = float[] (0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);

void main()
{
	const float weight[5] = float[] (0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);
    vec2 tex_offset = 1.0 / textureSize(t_Source, 0); // gets size of single texel
    vec3 result = texture(t_Source, v_TexCoord).rgb * weight[0]; // current fragment's contribution

    result += texture(t_Source, v_TexCoord + vec2(tex_offset.x * 1, 0.0)).rgb * weight[1];
	result += texture(t_Source, v_TexCoord - vec2(tex_offset.x * 1, 0.0)).rgb * weight[1];

    result += texture(t_Source, v_TexCoord + vec2(tex_offset.x * 2, 0.0)).rgb * weight[2];
	result += texture(t_Source, v_TexCoord - vec2(tex_offset.x * 2, 0.0)).rgb * weight[2];

    result += texture(t_Source, v_TexCoord + vec2(tex_offset.x * 3, 0.0)).rgb * weight[3];
	result += texture(t_Source, v_TexCoord - vec2(tex_offset.x * 3, 0.0)).rgb * weight[3];

    result += texture(t_Source, v_TexCoord + vec2(tex_offset.x * 4, 0.0)).rgb * weight[4];
	result += texture(t_Source, v_TexCoord - vec2(tex_offset.x * 4, 0.0)).rgb * weight[4];

    o_Color = vec4(result, 1.0);
}
";

pub static GAUSSIAN_BLUR_VERTICAL_SRC: &'static [u8] = b"

#version 150 core

uniform sampler2D t_Source;

in vec2 v_TexCoord;
out vec4 o_Color;

const float weight[5] = float[] (0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);

void main()
{
    vec2 tex_offset = 1.0 / textureSize(t_Source, 0); // gets size of single texel
    vec3 result = texture(t_Source, v_TexCoord).rgb * weight[0]; // current fragment's contribution

    result += texture(t_Source, v_TexCoord + vec2(0.0, tex_offset.y * 1)).rgb * weight[1];
    result += texture(t_Source, v_TexCoord - vec2(0.0, tex_offset.y * 1)).rgb * weight[1];

    result += texture(t_Source, v_TexCoord + vec2(0.0, tex_offset.y * 2)).rgb * weight[2];
    result += texture(t_Source, v_TexCoord - vec2(0.0, tex_offset.y * 2)).rgb * weight[2];

    result += texture(t_Source, v_TexCoord + vec2(0.0, tex_offset.y * 3)).rgb * weight[3];
    result += texture(t_Source, v_TexCoord - vec2(0.0, tex_offset.y * 3)).rgb * weight[3];

    result += texture(t_Source, v_TexCoord + vec2(0.0, tex_offset.y * 4)).rgb * weight[4];
    result += texture(t_Source, v_TexCoord - vec2(0.0, tex_offset.y * 4)).rgb * weight[4];

    o_Color = vec4(result, 1.0);
}
";

pub static COMPOSE_2_SRC: &'static [u8] = b"

#version 150 core

uniform sampler2D t_Source1;
uniform sampler2D t_Source2;

in vec2 v_TexCoord;
out vec4 o_Color;

void main() {
	o_Color = texture(t_Source1, v_TexCoord, 0) + texture(t_Source2, v_TexCoord, 0);
}
";

pub static CLIP_LUMINANCE_SRC: &'static [u8] = b"

#version 150 core

uniform sampler2D t_Source;

in vec2 v_TexCoord;
out vec4 o_Color;

void main() {
	vec4 src = texture(t_Source, v_TexCoord, 0);
	float l = max((dot(vec3(0.2126, 0.7152, 0.0722), src.rgb) - 1.), 0.);

	o_Color = src * l;
}

";

pub static QUAD_SMOOTH_SRC: &'static [u8] = b"

#version 150 core

uniform sampler2D t_Source;

in vec2 v_TexCoord;
out vec4 o_Color;

void main() {
	vec2 d = 1.0 / textureSize(t_Source, 0); // gets size of single texel
	float x1 = v_TexCoord.x - d.x/2;
	float x2 = x1 + d.x;
	float y1 = v_TexCoord.y - d.y/2.;
	float y2 = y1 + d.y;
    o_Color = (texture(t_Source, vec2(x1, y1), 0)
	         + texture(t_Source, vec2(x1, y2), 0)
	         + texture(t_Source, vec2(x2, y1), 0)
	         + texture(t_Source, vec2(x2, y2), 0)) / 4.0;
}

";

pub type HDR = (gfx::format::R16_G16_B16_A16, gfx::format::Float);
pub type LDR = gfx::format::Srgba8;

gfx_defines!{
	vertex BlitVertex {
		pos: [f32; 2] = "a_Pos",
		tex_coord: [f32; 2] = "a_TexCoord",
	}
	pipeline postprocess {
		vbuf: gfx::VertexBuffer<BlitVertex> = (),
		src: gfx::TextureSampler<[f32; 4]> = "t_Source",
		dst: gfx::RenderTarget<HDR> = "o_Color",
	}
	constant SmoothFragmentArgs {
        exp_alpha: f32 = "u_ExpAlpha",
    }
	pipeline smooth {
		vbuf: gfx::VertexBuffer<BlitVertex> = (),
		value: gfx::TextureSampler<[f32; 4]> = "t_Value",
		acc: gfx::TextureSampler<[f32; 4]> = "t_Acc",
		fragment_args: gfx::ConstantBuffer<SmoothFragmentArgs> = "cb_FragmentArgs",
		dst: gfx::RenderTarget<HDR> = "o_Smooth",
	}
	constant ToneMapVertexArgs {
        white: f32 = "u_White",
        black: f32 = "u_Black",
    }
	pipeline tone_map {
		vbuf: gfx::VertexBuffer<BlitVertex> = (),
		vertex_luminance: gfx::TextureSampler<[f32; 4]> = "t_VertexLuminance",
		vertex_args: gfx::ConstantBuffer<ToneMapVertexArgs> = "cb_VertexArgs",
		src: gfx::TextureSampler<[f32; 4]> = "t_Source",
		dst: gfx::RenderTarget<HDR> = "o_Color",
	}
	pipeline compose {
		vbuf: gfx::VertexBuffer<BlitVertex> = (),
		src1: gfx::TextureSampler<[f32; 4]> = "t_Source1",
		src2: gfx::TextureSampler<[f32; 4]> = "t_Source2",
		dst: gfx::RenderTarget<LDR> = "o_Color",
	}
}

use std::marker::PhantomData;
type HDRRenderSurface<R> = (gfx::handle::Texture<R, gfx::format::R16_G16_B16_A16>,
                            gfx::handle::ShaderResourceView<R, [f32; 4]>,
                            gfx::handle::RenderTargetView<R, (gfx::format::R16_G16_B16_A16, gfx::format::Float)>);
pub struct PostLighting<R: gfx::Resources, C: gfx::CommandBuffer<R>> {
	vertex_buffer: gfx::handle::Buffer<R, BlitVertex>,
	index_buffer_slice: gfx::Slice<R>,
	nearest_sampler: gfx::handle::Sampler<R>,
	linear_sampler: gfx::handle::Sampler<R>,

	ping_pong_small: [HDRRenderSurface<R>; 2],
	ping_pong_large: [HDRRenderSurface<R>; 2],

	mips: Vec<HDRRenderSurface<R>>,
	luminance_smooth: HDRRenderSurface<R>,
	luminance_acc: HDRRenderSurface<R>,

	blit_pso: gfx::pso::PipelineState<R, postprocess::Meta>,
	average_pso: gfx::pso::PipelineState<R, postprocess::Meta>,

	smooth_fragment_args: gfx::handle::Buffer<R, SmoothFragmentArgs>,
	smooth_pso: gfx::pso::PipelineState<R, smooth::Meta>,

	highlight_pso: gfx::pso::PipelineState<R, postprocess::Meta>,
	blur_h_pso: gfx::pso::PipelineState<R, postprocess::Meta>,
	blur_v_pso: gfx::pso::PipelineState<R, postprocess::Meta>,

	tone_map_vertex_args: gfx::handle::Buffer<R, ToneMapVertexArgs>,
	tone_map_pso: gfx::pso::PipelineState<R, tone_map::Meta>,

	compose_pso: gfx::pso::PipelineState<R, compose::Meta>,

	_buffer: PhantomData<C>,
}

impl<R: gfx::Resources, C: gfx::CommandBuffer<R>> PostLighting<R, C> {
	pub fn new<F>(factory: &mut F, w: u16, h: u16) -> PostLighting<R, C>
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

		let nearest_sampler = factory.create_sampler(gfx::tex::SamplerInfo::new(gfx::tex::FilterMethod::Scale,
		                                                                        gfx::tex::WrapMode::Clamp));

		let linear_sampler = factory.create_sampler(gfx::tex::SamplerInfo::new(gfx::tex::FilterMethod::Bilinear,
		                                                                       gfx::tex::WrapMode::Clamp));

		let tone_map_vertex_args = factory.create_constant_buffer(1);

		let tone_map_pso = factory.create_pipeline_simple(LUMINANCE_VERTEX_SRC, EXPOSURE_TONE_MAP_SRC, tone_map::new())
		                          .unwrap();
		let highlight_pso = factory.create_pipeline_simple(VERTEX_SRC, CLIP_LUMINANCE_SRC, postprocess::new()).unwrap();

		let blur_h_pso = factory.create_pipeline_simple(VERTEX_SRC, GAUSSIAN_BLUR_HORIZONTAL_SRC, postprocess::new())
		                        .unwrap();
		let blur_v_pso = factory.create_pipeline_simple(VERTEX_SRC, GAUSSIAN_BLUR_VERTICAL_SRC, postprocess::new())
		                        .unwrap();

		let blit_pso = factory.create_pipeline_simple(VERTEX_SRC, SIMPLE_BLIT_SRC, postprocess::new())
		                      .unwrap();

		let smooth_fragment_args = factory.create_constant_buffer(1);

		let smooth_pso = factory.create_pipeline_simple(VERTEX_SRC, EXPONENTIAL_SMOOTH_SRC, smooth::new())
		                        .unwrap();
		let average_pso = factory.create_pipeline_simple(VERTEX_SRC, QUAD_SMOOTH_SRC, postprocess::new())
		                         .unwrap();

		let compose_pso = factory.create_pipeline_simple(VERTEX_SRC, COMPOSE_2_SRC, compose::new())
		                         .unwrap();

		let ping_pong_small = [factory.create_render_target::<HDR>(w / 4, h / 4).unwrap(),
		                       factory.create_render_target::<HDR>(w / 4, h / 4).unwrap()];

		let ping_pong_large = [factory.create_render_target::<HDR>(w, h).unwrap(),
		                       factory.create_render_target::<HDR>(w, h).unwrap()];

		let mut mips: Vec<HDRRenderSurface<R>> = Vec::new();
		let mut w2 = w;
		let mut h2 = h;

		while w2 > 1 || h2 > 1 {
			w2 = (w2 + 1) / 2;
			h2 = (h2 + 1) / 2;
			// println!("{}x{}", w2, h2);
			mips.push(factory.create_render_target::<HDR>(w2, h2).unwrap());
		}

		let luminance_smooth = factory.create_render_target::<HDR>(1, 1).unwrap();
		let luminance_acc = factory.create_render_target::<HDR>(1, 1).unwrap();

		PostLighting {
			vertex_buffer: vertex_buffer,
			index_buffer_slice: slice,
			nearest_sampler: nearest_sampler,
			linear_sampler: linear_sampler,

			blit_pso: blit_pso,
			average_pso: average_pso,

			smooth_fragment_args: smooth_fragment_args,
			smooth_pso: smooth_pso,

			tone_map_vertex_args: tone_map_vertex_args,
			tone_map_pso: tone_map_pso,

			highlight_pso: highlight_pso,
			blur_h_pso: blur_h_pso,
			blur_v_pso: blur_v_pso,

			compose_pso: compose_pso,

			mips: mips,
			luminance_smooth: luminance_smooth,
			luminance_acc: luminance_acc,

			ping_pong_small: ping_pong_small,
			ping_pong_large: ping_pong_large,

			_buffer: PhantomData,
		}
	}

	fn full_screen_pass(&self,
	                    encoder: &mut gfx::Encoder<R, C>,
	                    pso: &gfx::pso::PipelineState<R, postprocess::Meta>,
	                    src: &gfx::handle::ShaderResourceView<R, [f32; 4]>,
	                    dst: &gfx::handle::RenderTargetView<R, HDR>) {
		encoder.draw(&self.index_buffer_slice,
		             pso,
		             &postprocess::Data {
			             vbuf: self.vertex_buffer.clone(),
			             src: (src.clone(), self.nearest_sampler.clone()),
			             dst: (dst.clone()),
		             });
	}

	pub fn apply_all(&mut self,
	                 encoder: &mut gfx::Encoder<R, C>,
	                 raw_hdr_src: gfx::handle::ShaderResourceView<R, [f32; 4]>,
	                 color_target: gfx::handle::RenderTargetView<R, LDR>) {

		let ping_pong_large = &self.ping_pong_large[..];
		let ping_pong_small = &self.ping_pong_small[..];

		// get average lum
		let mut exposure_src = raw_hdr_src.clone();
		let mut exposure_dst;
		// this is a fold, can we do it functionally?
		for mip in &self.mips {
			exposure_dst = mip.2.clone();
			self.full_screen_pass(encoder, &self.average_pso, &exposure_src, &exposure_dst);
			exposure_src = mip.1.clone();
			// println!("{:?}", mip.0.clone());
		}

		// Exponential smoothing
		encoder.update_constant_buffer(&self.smooth_fragment_args,
		                               &SmoothFragmentArgs { exp_alpha: 0.05 });
		encoder.draw(&self.index_buffer_slice,
		             &self.smooth_pso,
		             &smooth::Data {
			             vbuf: self.vertex_buffer.clone(),
			             fragment_args: self.smooth_fragment_args.clone(),
			             value: (exposure_src, self.nearest_sampler.clone()),
			             acc: (self.luminance_acc.1.clone(), self.nearest_sampler.clone()),
			             dst: self.luminance_smooth.2.clone(),
		             });

		// Tone mapping
		encoder.update_constant_buffer(&self.tone_map_vertex_args,
		                               &ToneMapVertexArgs {
			                               white: 4.0,
			                               black: 0.5,
		                               });
		encoder.draw(&self.index_buffer_slice,
		             &self.tone_map_pso,
		             &tone_map::Data {
			             vbuf: self.vertex_buffer.clone(),
			             vertex_luminance: (self.luminance_smooth.1.clone(),
			                                self.nearest_sampler.clone()),
			             vertex_args: self.tone_map_vertex_args.clone(),
			             src: (raw_hdr_src, self.nearest_sampler.clone()),
			             dst: ping_pong_large[0].2.clone(),
		             });

		// blits smoothed luminance to "acc" buffer. TODO: pingpong
		self.full_screen_pass(encoder,
		                      &self.blit_pso,
		                      &self.luminance_smooth.1,
		                      &self.luminance_acc.2);
		// Bloom
		// 1. extract high luminance
		self.full_screen_pass(encoder,
		                      &self.highlight_pso,
		                      &ping_pong_large[0].1,
		                      &ping_pong_small[0].2);
		// 2. horizontal 4x, 9x9 gaussian blur
		self.full_screen_pass(encoder,
		                      &self.blur_h_pso,
		                      &ping_pong_small[0].1,
		                      &ping_pong_small[1].2);
		// 2. vertical 4x, 9x9 gaussian blur
		self.full_screen_pass(encoder,
		                      &self.blur_v_pso,
		                      &ping_pong_small[1].1,
		                      &ping_pong_small[0].2);

		// compose tone mapped + bloom and resolve
		encoder.draw(&self.index_buffer_slice,
		             &self.compose_pso,
		             &compose::Data {
			             vbuf: self.vertex_buffer.clone(),
			             src1: (ping_pong_large[0].1.clone(), self.nearest_sampler.clone()),
			             // src1: (self.mips[10].1.clone(), self.nearest_sampler.clone()),
			             src2: (ping_pong_small[0].1.clone(), self.linear_sampler.clone()),
			             dst: color_target.clone(),
		             });
	}
}
