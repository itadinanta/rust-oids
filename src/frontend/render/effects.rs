use gfx;
use gfx::traits::FactoryExt;

use core::resource;
use frontend::render::formats;
use frontend::render::Result;

pub type RenderColorFormat = formats::RenderColorFormat;
pub type ScreenColorFormat = formats::ScreenColorFormat;

gfx_defines! {
	vertex BlitVertex {
		pos: [f32; 2] = "a_Pos",
		tex_coord: [f32; 2] = "a_TexCoord",
	}
	pipeline postprocess {
		vbuf: gfx::VertexBuffer<BlitVertex> = (),
		src: gfx::TextureSampler<[f32; 4]> = "t_Source",
		dst: gfx::RenderTarget<RenderColorFormat> = "o_Color",
	}
	constant SmoothFragmentArgs {
        exp_alpha: f32 = "u_ExpAlpha",
    }
	pipeline smooth {
		vbuf: gfx::VertexBuffer<BlitVertex> = (),
		value: gfx::TextureSampler<[f32; 4]> = "t_Value",
		acc: gfx::TextureSampler<[f32; 4]> = "t_Acc",
		fragment_args: gfx::ConstantBuffer<SmoothFragmentArgs> = "cb_FragmentArgs",
		dst: gfx::RenderTarget<RenderColorFormat> = "o_Smooth",
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
		dst: gfx::RenderTarget<RenderColorFormat> = "o_Color",
	}
	pipeline compose {
		vbuf: gfx::VertexBuffer<BlitVertex> = (),
		src1: gfx::TextureSampler<[f32; 4]> = "t_Source1",
		src2: gfx::TextureSampler<[f32; 4]> = "t_Source2",
		dst: gfx::RenderTarget<ScreenColorFormat> = "o_Color",
	}
}

use std::marker::PhantomData;

pub struct PostLighting<R: gfx::Resources, C: gfx::CommandBuffer<R>> {
	vertex_buffer: gfx::handle::Buffer<R, BlitVertex>,
	index_buffer_slice: gfx::Slice<R>,
	nearest_sampler: gfx::handle::Sampler<R>,
	linear_sampler: gfx::handle::Sampler<R>,

	resolved: formats::RenderSurface<R>,
	resolve_msaa_pso: gfx::pso::PipelineState<R, postprocess::Meta>,

	ping_pong_half: [formats::RenderSurface<R>; 2],
	ping_pong_full: [formats::RenderSurface<R>; 2],

	mips: Vec<formats::RenderSurface<R>>,
	luminance_smooth: formats::RenderSurface<R>,
	luminance_acc: formats::RenderSurface<R>,

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
	pub fn new<F>(factory: &mut F, res: &resource::ResourceLoader<u8>, w: u16, h: u16) -> Result<PostLighting<R, C>>
	where
		F: gfx::Factory<R>, {
		let full_screen_triangle = vec![
			BlitVertex {
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
			},
		];

		let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&full_screen_triangle, ());

		let nearest_sampler = factory.create_sampler(gfx::texture::SamplerInfo::new(
			gfx::texture::FilterMethod::Scale,
			gfx::texture::WrapMode::Clamp,
		));

		let linear_sampler = factory.create_sampler(gfx::texture::SamplerInfo::new(
			gfx::texture::FilterMethod::Bilinear,
			gfx::texture::WrapMode::Clamp,
		));

		let tone_map_vertex_args = factory.create_constant_buffer(1);
		let smooth_fragment_args = factory.create_constant_buffer(1);

		macro_rules! load_pipeline_simple {
			($v:expr, $f:expr, $s:ident) => { factory.create_pipeline_simple(
					&res.load(concat!("shaders/effects/", $v, ".vert"))?,
					&res.load(concat!("shaders/effects/", $f, ".frag"))?,
					$s::new())}
		};

		let tone_map_pso = load_pipeline_simple!("luminance", "exposure_tone_map", tone_map)?;
		let resolve_msaa_pso = load_pipeline_simple!("identity", "msaa4x_resolve", postprocess)?;
		let highlight_pso = load_pipeline_simple!("identity", "clip_luminance", postprocess)?;
		let blur_h_pso = load_pipeline_simple!("identity", "gaussian_blur_horizontal", postprocess)?;
		let blur_v_pso = load_pipeline_simple!("identity", "gaussian_blur_vertical", postprocess)?;
		let blit_pso = load_pipeline_simple!("identity", "simple_blit", postprocess)?;
		let smooth_pso = load_pipeline_simple!("identity", "exponential_smooth", smooth)?;
		let average_pso = load_pipeline_simple!("identity", "quad_smooth", postprocess)?;
		let compose_pso = load_pipeline_simple!("identity", "compose_2", compose)?;

		let resolved = factory.create_render_target::<RenderColorFormat>(w, h)?;

		let ping_pong_half = [
			factory.create_render_target::<RenderColorFormat>(
				w / 2,
				h / 2,
			)?,
			factory.create_render_target::<RenderColorFormat>(
				w / 2,
				h / 2,
			)?,
		];

		let ping_pong_full = [
			factory.create_render_target::<RenderColorFormat>(w, h)?,
			factory.create_render_target::<RenderColorFormat>(w, h)?,
		];

		let mut mips: Vec<formats::RenderSurface<R>> = Vec::new();
		let mut w2 = w;
		let mut h2 = h;

		while w2 > 1 || h2 > 1 {
			w2 = (w2 + 1) / 2;
			h2 = (h2 + 1) / 2;
			mips.push(factory.create_render_target::<RenderColorFormat>(w2, h2)?);
		}

		let luminance_smooth = factory.create_render_target::<RenderColorFormat>(1, 1)?;
		let luminance_acc = factory.create_render_target::<RenderColorFormat>(1, 1)?;

		Ok(PostLighting {
			vertex_buffer,
			index_buffer_slice: slice,
			nearest_sampler,
			linear_sampler,

			blit_pso,
			average_pso,

			smooth_fragment_args,
			smooth_pso,

			tone_map_vertex_args,
			tone_map_pso,

			highlight_pso,
			blur_h_pso,
			blur_v_pso,
			resolve_msaa_pso,

			resolved,
			compose_pso,

			mips,
			luminance_smooth,
			luminance_acc,

			ping_pong_half,
			ping_pong_full,

			_buffer: PhantomData,
		})
	}

	fn full_screen_pass(
		&self, encoder: &mut gfx::Encoder<R, C>, pso: &gfx::pso::PipelineState<R, postprocess::Meta>,
		src: &gfx::handle::ShaderResourceView<R, [f32; 4]>, dst: &gfx::handle::RenderTargetView<R, RenderColorFormat>
	) {
		encoder.draw(
			&self.index_buffer_slice,
			pso,
			&postprocess::Data {
				vbuf: self.vertex_buffer.clone(),
				src: (src.clone(), self.nearest_sampler.clone()),
				dst: (dst.clone()),
			},
		);
	}

	pub fn apply_all(
		&mut self, encoder: &mut gfx::Encoder<R, C>, raw_hdr_src: gfx::handle::ShaderResourceView<R, [f32; 4]>,
		color_target: gfx::handle::RenderTargetView<R, ScreenColorFormat>
	) {
		let ping_pong_full = &self.ping_pong_full[..];
		let ping_pong_half = &self.ping_pong_half[..];

		// blits smoothed luminance to "acc" buffer. TODO: pingpong
		self.full_screen_pass(
			encoder,
			&self.resolve_msaa_pso,
			&raw_hdr_src,
			&self.resolved.2,
		);

		// get average lum
		let mut exposure_src = self.resolved.1.clone();
		let mut exposure_dst;
		// this is a fold, can we do it functionally?
		for mip in &self.mips {
			exposure_dst = mip.2.clone();
			self.full_screen_pass(encoder, &self.average_pso, &exposure_src, &exposure_dst);
			exposure_src = mip.1.clone();
			// println!("{:?}", mip.0.clone());
		}

		// Exponential smoothing
		encoder.update_constant_buffer(
			&self.smooth_fragment_args,
			&SmoothFragmentArgs { exp_alpha: 0.05 },
		);
		encoder.draw(
			&self.index_buffer_slice,
			&self.smooth_pso,
			&smooth::Data {
				vbuf: self.vertex_buffer.clone(),
				fragment_args: self.smooth_fragment_args.clone(),
				value: (exposure_src, self.nearest_sampler.clone()),
				acc: (self.luminance_acc.1.clone(), self.nearest_sampler.clone()),
				dst: self.luminance_smooth.2.clone(),
			},
		);

		// Tone mapping
		encoder.update_constant_buffer(
			&self.tone_map_vertex_args,
			&ToneMapVertexArgs {
				white: 4.0,
				black: 0.5,
			},
		);
		encoder.draw(
			&self.index_buffer_slice,
			&self.tone_map_pso,
			&tone_map::Data {
				vbuf: self.vertex_buffer.clone(),
				vertex_luminance: (
					self.luminance_smooth.1.clone(),
					self.nearest_sampler.clone(),
				),
				vertex_args: self.tone_map_vertex_args.clone(),
				src: (self.resolved.1.clone(), self.nearest_sampler.clone()),
				dst: ping_pong_full[0].2.clone(),
			},
		);

		// blits smoothed luminance to "acc" buffer for next frame. TODO: CPU?
		self.full_screen_pass(
			encoder,
			&self.blit_pso,
			&self.luminance_smooth.1,
			&self.luminance_acc.2,
		);
		// Bloom
		// 1. extract high luminance
		self.full_screen_pass(
			encoder,
			&self.highlight_pso,
			&ping_pong_full[0].1,
			&ping_pong_half[0].2,
		);
		// 2. horizontal 4x, 9x9 gaussian blur
		self.full_screen_pass(
			encoder,
			&self.blur_h_pso,
			&ping_pong_half[0].1,
			&ping_pong_half[1].2,
		);
		// 2. vertical 4x, 9x9 gaussian blur
		self.full_screen_pass(
			encoder,
			&self.blur_v_pso,
			&ping_pong_half[1].1,
			&ping_pong_half[0].2,
		);

		// compose tone mapped + bloom and resolve
		encoder.draw(
			&self.index_buffer_slice,
			&self.compose_pso,
			&compose::Data {
				vbuf: self.vertex_buffer.clone(),
				// original
				src1: (ping_pong_full[0].1.clone(), self.nearest_sampler.clone()),
				// bloom
				src2: (ping_pong_half[0].1.clone(), self.linear_sampler.clone()),
				dst: color_target.clone(),
			},
		);
	}
}
