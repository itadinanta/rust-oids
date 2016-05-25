use gfx;
use gfx::traits::FactoryExt;

pub static VERTEX_SRC: &'static [u8] = b"
    #version 150 core

    layout (std140) uniform cb_VertexArgs {
        uniform mat4 u_Proj;
        uniform mat4 u_View;
        uniform mat4 u_Model;
    };

    in vec3 a_Pos;
    in vec3 a_Normal;
    in vec2 a_TexCoord;

    out VertexData {
        vec4 Position;
        vec3 Normal;
        vec2 TexCoord;
    } v_Out;

    void main() {
        v_Out.Position = u_Model * vec4(a_Pos, 1.0);
        v_Out.Normal = mat3(u_Model) * a_Normal;
        v_Out.TexCoord = a_TexCoord;
        gl_Position = u_Proj * u_View * v_Out.Position;
    }
";

pub static FRAGMENT_SRC: &'static [u8] = b"
    #version 150 core
    #define MAX_NUM_TOTAL_LIGHTS 512

    layout (std140) uniform cb_FragmentArgs {
        int u_LightCount;
    };

    struct Light {
        vec4 propagation;
        vec4 center;
        vec4 color;
    };

    layout (std140) uniform u_Lights {
        Light light[MAX_NUM_TOTAL_LIGHTS];
    };

    uniform sampler2D t_Ka;
    uniform sampler2D t_Kd;

    in VertexData {
        vec4 Position;
        vec3 Normal;
        vec2 TexCoord;
    } v_In;

    out vec4 o_Color;

    void main() {
        vec4 kd = texture(t_Kd, v_In.TexCoord);
        vec4 color = texture(t_Ka, v_In.TexCoord);
        for (int i = 0; i < u_LightCount; i++) {
            vec4 delta = light[i].center - v_In.Position;
            float dist = length(delta);
            float inv_dist = 1. / dist;
            vec4 light_to_point_normal = delta * inv_dist;
            float intensity = dot(light[i].propagation.xyz, vec3(1., inv_dist, inv_dist * inv_dist));
            color += kd * light[i].color * intensity * max(0, dot(light_to_point_normal, vec4(v_In.Normal, 0.)));
        }
        o_Color = color;
    }
";

/// Placeholder Color format
pub type ColorFormat = gfx::format::Rgba8;
/// Placeholder Depth Format
pub type DepthFormat = gfx::format::DepthStencil;


// placeholder
gfx_vertex_struct!( VertexPosNormal {
	pos: [f32; 3] = "a_Pos",
	normal: [f32; 3] = "a_Normal",
	tex_coord: [f32; 2] = "a_TexCoord",
});

/// holds a 1x1 texture that can be used to store constant colors
pub struct ConstantColorTexture<R: gfx::Resources> {
	texture: gfx::handle::Texture<R, gfx::format::R8_G8_B8_A8>,
	view: gfx::handle::ShaderResourceView<R, [f32; 4]>
}

impl<R: gfx::Resources> ConstantColorTexture<R> {
	/// Create a texture buffer
	pub fn new<F>(factory: &mut F) -> ConstantColorTexture<R>
		where F: gfx::Factory<R> {
		let kind = gfx::tex::Kind::D2(1, 1, gfx::tex::AaMode::Single);
		let tex = factory.create_texture::<gfx::format::R8_G8_B8_A8>(
            kind,
            1,
            gfx::SHADER_RESOURCE,
            gfx::Usage::Dynamic,
            Some(gfx::format::ChannelType::Unorm)
        ).unwrap();
		let levels = (0, tex.get_info().levels - 1);
		let view = factory.view_texture_as_shader_resource::<gfx::format::Rgba8>(&tex, levels, gfx::format::Swizzle::new()).unwrap();
		ConstantColorTexture {
			texture: tex,
			view: view
		}
	}
}

pub struct ColorBuffer<R: gfx::Resources> {
	pub color: gfx::handle::RenderTargetView<R, ColorFormat>,
	pub output_depth: gfx::handle::DepthStencilView<R, DepthFormat>
}

pub type GFormat = [f32; 4];

gfx_defines!(
    constant PointLight {
        propagation: [f32; 4] = "propagation",
        center: [f32; 4] = "center",
        color: [f32; 4] = "color",
    }

    constant VertexArgs {
        proj: [[f32; 4]; 4] = "u_Proj",
        view: [[f32; 4]; 4] = "u_View",
        model: [[f32; 4]; 4] = "u_Model",
    }

    constant FragmentArgs {
        light_count: i32 = "u_LightCount",
    }

    pipeline shaded {
        vbuf: gfx::VertexBuffer<VertexPosNormal> = (),
        vertex_args: gfx::ConstantBuffer<VertexArgs> = "cb_VertexArgs",
        fragment_args: gfx::ConstantBuffer<FragmentArgs> = "cb_FragmentArgs",
        lights: gfx::ConstantBuffer<PointLight> = "u_Lights",
        out_ka: gfx::RenderTarget<gfx::format::Rgba8> = "o_Color",
        out_depth: gfx::DepthTarget<gfx::format::DepthStencil> = gfx::preset::depth::LESS_EQUAL_WRITE,
        ka: gfx::TextureSampler<[f32; 4]> = "t_Ka",
        kd: gfx::TextureSampler<[f32; 4]> = "t_Kd",
    }
);

pub struct DrawShaded<R: gfx::Resources> {
	vertex: gfx::handle::Buffer<R, VertexArgs>,
	fragment: gfx::handle::Buffer<R, FragmentArgs>,
	lights: gfx::handle::Buffer<R, PointLight>,
	pso: gfx::pso::PipelineState<R, shaded::Meta>,
	sampler: gfx::handle::Sampler<R>,
	ka: ConstantColorTexture<R>,
	kd: ConstantColorTexture<R>
}

impl<R: gfx::Resources> DrawShaded<R> {
	pub fn new<F>(factory: &mut F) -> DrawShaded<R>
		where R: gfx::Resources,
		      F: gfx::Factory<R> {
		let lights = factory.create_constant_buffer(512);
		let vertex = factory.create_constant_buffer(1);
		let fragment = factory.create_constant_buffer(1);
		let pso = factory.create_pipeline_simple(VERTEX_SRC, FRAGMENT_SRC, shaded::new())
		                 .unwrap();

		let sampler =
			factory.create_sampler(gfx::tex::SamplerInfo::new(gfx::tex::FilterMethod::Scale,
			                                                  gfx::tex::WrapMode::Clamp));

		DrawShaded {
			vertex: vertex,
			fragment: fragment,
			lights: lights,
			pso: pso,
			ka: ConstantColorTexture::new(factory),
			kd: ConstantColorTexture::new(factory),
			sampler: sampler
		}
	}

	fn draw<C: gfx::CommandBuffer<R>>(&self,
	                                  target: &ColorBuffer<R>,
	                                  encoder: &mut gfx::Encoder<R, C>,
	                                  lights: &Vec<PointLight>) {

		let count = lights.len();
		while lights.len() < 512 {
			lights.push(PointLight {
				propagation: [0., 0., 0., 0.],
				color: [0., 0., 0., 0.],
				center: [0., 0., 0., 0.]
			})
		}
		encoder.update_buffer(&self.lights, &lights[..], 0).unwrap();

		encoder.update_constant_buffer(&self.vertex,
		                               &VertexArgs {
			                               proj: projection,
			                               view: view,
			                               model: transform
		                               });

		encoder.update_constant_buffer(&self.fragment, &FragmentArgs { light_count: count as i32 });

		let ka = e.ka.to_view(&self.ka, encoder);
		let kd = e.kd.to_view(&self.kd, encoder);

		encoder.draw(&e.slice,
		             &self.pso,
		             &shaded::Data {
			             vbuf: e.buffer.clone(),
			             fragment_args: self.fragment.clone(),
			             vertex_args: self.vertex.clone(),
			             lights: self.lights.clone(),
			             out_ka: target.color.clone(),
			             out_depth: target.output_depth.clone(),
			             ka: (ka, self.sampler.clone()),
			             kd: (kd, self.sampler.clone())
		             });
	}
}

fn pad(x: [f32; 3]) -> [f32; 4] {
	[x[0], x[1], x[2], 0.]
}
