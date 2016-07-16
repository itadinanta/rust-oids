use gfx;
use gfx::traits::FactoryExt;

extern crate cgmath;
extern crate gfx_text;

pub static LIGHTING_VERTEX_SRC: &'static [u8] = b"

#version 150 core

layout (std140) uniform cb_CameraArgs {
    uniform mat4 u_Proj;
    uniform mat4 u_View;
};

layout (std140) uniform cb_ModelArgs {
    uniform mat4 u_Model;
};

in vec3 a_Pos;
in vec3 a_Normal;
in vec3 a_Tangent;
in vec2 a_TexCoord;

out VertexData {
    vec4 Position;
    vec3 Normal;
    mat3 TBN;
    vec2 TexCoord;
} v_Out;

void main() {
    v_Out.Position = u_Model * vec4(a_Pos, 1.0);
    mat3 model = mat3(u_Model);
	vec3 normal = normalize(model * a_Normal);

    v_Out.Normal = normal;
	vec3 tangent = normalize(model * a_Tangent);
	vec3 bitangent = cross(normal, tangent);

    v_Out.TBN = mat3(tangent, bitangent, normal);

    v_Out.TexCoord = a_TexCoord;
    gl_Position = u_Proj * u_View * v_Out.Position;
}

";
const MAX_NUM_TOTAL_LIGHTS: usize = 16;

pub static LIGHTING_BALL_FRAGMENT_SRC: &'static [u8] = b"

#version 150 core
#define MAX_NUM_TOTAL_LIGHTS 16

const float PI = 3.1415926535897932384626433832795;
const float PI_2 = 1.57079632679489661923;

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

layout (std140) uniform cb_MaterialArgs {
    uniform vec4 u_Emissive;
};

in VertexData {
    vec4 Position;
    vec3 Normal;
    mat3 TBN;
    vec2 TexCoord;
} v_In;

out vec4 o_Color;

void main() {
    vec4 kd = vec4(0.2, 0.2, 0.2, 1.0);
    vec4 ks = vec4(1.0, 1.0, 1.0, 1.0);
    vec4 kp = vec4(64.0, 32.0, 64.0, 64.0);
    vec4 ka = vec4(0.0, 0.0, 0.01, 0.0);

    vec4 color = (ka + u_Emissive);

    float dx = v_In.TexCoord.x - 0.5;
    float dy = v_In.TexCoord.y - 0.5;
	float r = dx * dx + dy * dy;
	vec3 normal_map = vec3(0., 0., 1.);
    if (r > 0.25) {
        discard;
    } 
    else {
	    dx *= 2;
	    dy *= 2;

		float bump = sqrt(1. - dx * dx - dy * dy);
		normal_map = vec3(dx, dy, bump);
	};
	
	vec3 normal = v_In.TBN * normal_map;

	for (int i = 0; i < u_LightCount; i++) {
		vec4 delta = light[i].center - v_In.Position;
		float dist = length(delta);
		float inv_dist = 1. / dist;
		vec4 light_to_point_normal = delta * inv_dist;
		float intensity = dot(light[i].propagation.xyz, vec3(1., inv_dist, inv_dist * inv_dist));

		float lambert = max(0, dot(light_to_point_normal, vec4(normal, 0.0)));

		vec4 specular;
		if (lambert >= 0.0) {
//			Blinn-Phong:
			vec3 lightDir = light_to_point_normal.xyz;
            vec3 viewDir = vec3(0.0, 0.0, 1.0); // ortho, normalize(-v_In.Position.xyz); perspective
            vec3 halfDir = normalize(lightDir + viewDir); // can be done in vertex shader
			float specAngle = max(dot(halfDir, normal), 0.0);
//			Phong:
//	        vec3 reflectDir = reflect(-lightDir, v_In.Normal.xyz);
//	        float specAngle = max(dot(reflectDir, viewDir), 0.0);
			specular = pow(vec4(specAngle), kp);
		}
		else {
			specular = vec4(0.0);
		}
		color += light[i].color * intensity * (kd * lambert + ks * specular);
	}
	// gl_FragDepth = bump;
	o_Color = color;
}

";

pub static LIGHTING_POLY_FRAGMENT_SRC: &'static [u8] = b"

#version 150 core
#define MAX_NUM_TOTAL_LIGHTS 16

const float PI = 3.1415926535897932384626433832795;
const float PI_2 = 1.57079632679489661923;

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

layout (std140) uniform cb_MaterialArgs {
    uniform vec4 u_Emissive;
};

in VertexData {
    vec4 Position;
    vec3 Normal;
    mat3 TBN;
    vec2 TexCoord;
} v_In;

out vec4 o_Color;

void main() {
    vec4 kd = vec4(0.2, 0.2, 0.2, 1.0);
    vec4 ks = vec4(1.0, 1.0, 1.0, 1.0);
    vec4 kp = vec4(64.0, 32.0, 64.0, 64.0);
    vec4 ka = vec4(0.0, 0.0, 0.01, 0.0);

    vec4 color = (ka + u_Emissive);

    float dx = v_In.TexCoord.x - 0.5;
    float dy = v_In.TexCoord.y - 0.5;
	float r = dx * dx + dy * dy;
	vec3 normal_map = vec3(0., 0., 1.);
    if (r > 0.25) {
        discard;
    } 
    else {
	    dx *= 2;
	    dy *= 2;

		float bump = sqrt(1. - dx * dx - dy * dy);
		normal_map = vec3(dx, dy, bump);
	};
	
	vec3 normal = v_In.TBN * normal_map;

	for (int i = 0; i < u_LightCount; i++) {
		vec4 delta = light[i].center - v_In.Position;
		float dist = length(delta);
		float inv_dist = 1. / dist;
		vec4 light_to_point_normal = delta * inv_dist;
		float intensity = dot(light[i].propagation.xyz, vec3(1., inv_dist, inv_dist * inv_dist));

		float lambert = max(0, dot(light_to_point_normal, vec4(normal, 0.0)));

		vec4 specular;
		if (lambert >= 0.0) {
//			Blinn-Phong:
			vec3 lightDir = light_to_point_normal.xyz;
            vec3 viewDir = vec3(0.0, 0.0, 1.0); // ortho, normalize(-v_In.Position.xyz); perspective
            vec3 halfDir = normalize(lightDir + viewDir); // can be done in vertex shader
			float specAngle = max(dot(halfDir, normal), 0.0);
//			Phong:
//	        vec3 reflectDir = reflect(-lightDir, v_In.Normal.xyz);
//	        float specAngle = max(dot(reflectDir, viewDir), 0.0);
			specular = pow(vec4(specAngle), kp);
		}
		else {
			specular = vec4(0.0);
		}
		color += light[i].color * intensity * (kd * lambert + ks * specular);
	}
	// gl_FragDepth = bump;
	o_Color = color;
}

";


gfx_vertex_struct!(VertexPosNormal {
	pos: [f32; 3] = "a_Pos",
	normal: [f32; 3] = "a_Normal",
	tangent: [f32; 3] = "a_Tangent",
	tex_coord: [f32; 2] = "a_TexCoord",
});

pub type Vertex = VertexPosNormal;
pub type HDRColorFormat = (gfx::format::R16_G16_B16_A16, gfx::format::Float);
pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

pub type GFormat = [f32; 4];

pub type M44 = cgmath::Matrix4<f32>;

gfx_defines!(
    constant PointLight {
        propagation: [f32; 4] = "propagation",
        center: [f32; 4] = "center",
        color: [f32; 4] = "color",
    }

    constant CameraArgs {
        proj: [[f32; 4]; 4] = "u_Proj",
        view: [[f32; 4]; 4] = "u_View",
    }

    constant ModelArgs {
        model: [[f32; 4]; 4] = "u_Model",
    }

    constant FragmentArgs {
        light_count: i32 = "u_LightCount",
    }

	constant MaterialArgs {
		emissive: [f32; 4] = "u_Emissive",
	}

    pipeline shaded {
        vbuf: gfx::VertexBuffer<VertexPosNormal> = (),
        camera_args: gfx::ConstantBuffer<CameraArgs> = "cb_CameraArgs",
        model_args: gfx::ConstantBuffer<ModelArgs> = "cb_ModelArgs",
        fragment_args: gfx::ConstantBuffer<FragmentArgs> = "cb_FragmentArgs",
        material_args: gfx::ConstantBuffer<MaterialArgs> = "cb_MaterialArgs",
        lights: gfx::ConstantBuffer<PointLight> = "u_Lights",
        color_target: gfx::RenderTarget<HDRColorFormat> = "o_Color",
        depth_target: gfx::DepthTarget<gfx::format::DepthStencil> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }
);

use std::marker::PhantomData;
pub struct ForwardLighting<R: gfx::Resources, C: gfx::CommandBuffer<R>> {
	camera: gfx::handle::Buffer<R, CameraArgs>,
	model: gfx::handle::Buffer<R, ModelArgs>,
	fragment: gfx::handle::Buffer<R, FragmentArgs>,
	material: gfx::handle::Buffer<R, MaterialArgs>,
	lights: gfx::handle::Buffer<R, PointLight>,
	ball_pso: gfx::pso::PipelineState<R, shaded::Meta>,
	poly_pso: gfx::pso::PipelineState<R, shaded::Meta>,
	_buffer: PhantomData<C>,
}

impl<R: gfx::Resources, C: gfx::CommandBuffer<R>> ForwardLighting<R, C> {
	pub fn new<F>(factory: &mut F) -> ForwardLighting<R, C>
		where F: gfx::Factory<R> {
		let lights = factory.create_constant_buffer(MAX_NUM_TOTAL_LIGHTS);
		let camera = factory.create_constant_buffer(1);
		let model = factory.create_constant_buffer(1);
		let fragment = factory.create_constant_buffer(1);
		let material = factory.create_constant_buffer(1);
		let ball_pso = factory.create_pipeline_simple(LIGHTING_VERTEX_SRC,
		                                              LIGHTING_BALL_FRAGMENT_SRC,
		                                              shaded::new())
		                      .unwrap();
		let poly_pso = factory.create_pipeline_simple(LIGHTING_VERTEX_SRC,
		                                              LIGHTING_POLY_FRAGMENT_SRC,
		                                              shaded::new())
		                      .unwrap();


		ForwardLighting {
			camera: camera,
			model: model,
			fragment: fragment,
			material: material,
			lights: lights,
			ball_pso: ball_pso,
			poly_pso: poly_pso,
			_buffer: PhantomData,
		}
	}

	pub fn setup(&self,
	             encoder: &mut gfx::Encoder<R, C>,
	             camera_projection: M44,
	             camera_view: M44,
	             lights: &Vec<PointLight>) {

		let mut lights_buf = lights.clone();

		let count = lights_buf.len();
		while lights_buf.len() < MAX_NUM_TOTAL_LIGHTS {
			lights_buf.push(PointLight {
				propagation: [0., 0., 0., 0.],
				color: [0., 0., 0., 0.],
				center: [0., 0., 0., 0.],
			})
		}

		encoder.update_buffer(&self.lights, &lights_buf[..], 0).unwrap();

		encoder.update_constant_buffer(&self.camera,
		                               &CameraArgs {
			                               proj: camera_projection.into(),
			                               view: camera_view.into(),
		                               });

		encoder.update_constant_buffer(&self.fragment, &FragmentArgs { light_count: count as i32 });
	}

	pub fn draw_triangles(&self,
	                      encoder: &mut gfx::Encoder<R, C>,
	                      vertices: &gfx::handle::Buffer<R, VertexPosNormal>,
	                      indices: &gfx::Slice<R>,
	                      transform: &M44,
	                      color: [f32; 4],
	                      color_buffer: &gfx::handle::RenderTargetView<R, HDRColorFormat>,
	                      depth_buffer: &gfx::handle::DepthStencilView<R, DepthFormat>) {
		self.draw_triangles_pso(encoder,
		                        &self.ball_pso,
		                        vertices,
		                        indices,
		                        transform,
		                        color,
		                        color_buffer,
		                        depth_buffer);
	}

	fn draw_triangles_pso(&self,
	                      encoder: &mut gfx::Encoder<R, C>,
	                      pso: &gfx::PipelineState<R, shaded::Meta>,
	                      vertices: &gfx::handle::Buffer<R, VertexPosNormal>,
	                      indices: &gfx::Slice<R>,
	                      transform: &M44,
	                      color: [f32; 4],
	                      color_buffer: &gfx::handle::RenderTargetView<R, HDRColorFormat>,
	                      depth_buffer: &gfx::handle::DepthStencilView<R, DepthFormat>) {

		encoder.update_constant_buffer(&self.model, &ModelArgs { model: transform.clone().into() });

		encoder.update_constant_buffer(&self.material, &MaterialArgs { emissive: color });

		encoder.draw(&indices,
		             &pso,
		             &shaded::Data {
			             vbuf: vertices.clone(),
			             fragment_args: self.fragment.clone(),
			             material_args: self.material.clone(),
			             camera_args: self.camera.clone(),
			             model_args: self.model.clone(),
			             lights: self.lights.clone(),
			             color_target: color_buffer.clone(),
			             depth_target: depth_buffer.clone(),
		             });
	}
}
