use gfx;
use gfx::traits::FactoryExt;

extern crate cgmath;

pub static VERTEX_SRC: &'static [u8] = b"
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
        v_Out.Normal = normalize(mat3(u_Model) * a_Normal);

        mat3 viewModel = mat3(u_View) * mat3(u_Model);

        vec3 normal = normalize(viewModel * a_Normal);
        vec3 tangent = normalize(viewModel * a_Tangent);
        vec3 bitangent = cross(normal, tangent);

        v_Out.TBN = mat3(tangent, bitangent, normal);

        v_Out.TexCoord = a_TexCoord;
        gl_Position = u_Proj * u_View * v_Out.Position;
    }
";

pub static FRAGMENT_SRC: &'static [u8] = b"
    #version 150 core
    #define MAX_NUM_TOTAL_LIGHTS 512

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
        vec4 ka = vec4(0.0, 0.0, 0.1, 0.0);

        vec4 color = ka;

        float dx = v_In.TexCoord.x - 0.5;
        float dy = v_In.TexCoord.y - 0.5;

        if (dx * dx + dy * dy > 0.25) {
	        discard;
	    }
	    
	    dx *= 2;
	    dy *= 2;
	    
	    vec3 normal_map = vec3(dx, dy, sqrt(1 - dx * dx - dy * dy));
	    
		vec3 normal = normalize(v_In.TBN * normal_map);

        for (int i = 0; i < u_LightCount; i++) {
            vec4 delta = light[i].center - v_In.Position;
            float dist = length(delta);
            float inv_dist = 1. / dist;
            vec4 light_to_point_normal = delta * inv_dist;
            float intensity = dot(light[i].propagation.xyz, vec3(1., inv_dist, inv_dist * inv_dist));
            
            float lambert = max(0, dot(light_to_point_normal, vec4(normal, 0.0)));
           

			vec4 specular;
			if (lambert >= 0.0) 
			{
				// blinn-phong
                vec3 lightDir = light_to_point_normal.xyz;
                vec3 viewDir = vec3(0.0, 0.0, 1.0); // ortho, normalize(-v_In.Position.xyz); perspective

                // phong
                vec3 halfDir = normalize(lightDir + viewDir);
//	            vec3 reflectDir = reflect(-lightDir, v_In.Normal.xyz);
//	            float specAngle = max(dot(reflectDir, viewDir), 0.0);

		        float specAngle = max(dot(halfDir, normal), 0.0);
		        specular = pow(vec4(specAngle), kp);
			}
			else
			{
				specular = vec4(0.0);
			}
            color += light[i].color * intensity * (kd * lambert + ks * specular);
        }
        o_Color = color;
    }
";

/// Placeholder Color format
pub type ColorFormat = gfx::format::Rgba8;
/// Placeholder Depth Format
pub type DepthFormat = gfx::format::DepthStencil;


// placeholder
gfx_vertex_struct!(VertexPosNormal {
	pos: [f32; 3] = "a_Pos",
	normal: [f32; 3] = "a_Normal",
	tangent: [f32; 3] = "a_Tangent",
	tex_coord: [f32; 2] = "a_TexCoord",
});

pub type GFormat = [f32; 4];

pub type M44 = cgmath::Matrix4<f32>;

pub const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

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

    pipeline shaded {
        vbuf: gfx::VertexBuffer<VertexPosNormal> = (),
        camera_args: gfx::ConstantBuffer<CameraArgs> = "cb_CameraArgs",
        model_args: gfx::ConstantBuffer<ModelArgs> = "cb_ModelArgs",
        fragment_args: gfx::ConstantBuffer<FragmentArgs> = "cb_FragmentArgs",
        lights: gfx::ConstantBuffer<PointLight> = "u_Lights",
        out_ka: gfx::RenderTarget<gfx::format::Rgba8> = "o_Color",
        out_depth: gfx::DepthTarget<gfx::format::DepthStencil> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }
);

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

pub struct DrawShaded<R: gfx::Resources> {
	camera: gfx::handle::Buffer<R, CameraArgs>,
	model: gfx::handle::Buffer<R, ModelArgs>,
	fragment: gfx::handle::Buffer<R, FragmentArgs>,
	lights: gfx::handle::Buffer<R, PointLight>,
	pso: gfx::pso::PipelineState<R, shaded::Meta>,
}

impl<R: gfx::Resources> DrawShaded<R> {
	pub fn new<F>(factory: &mut F) -> DrawShaded<R>
		where R: gfx::Resources,
		      F: gfx::Factory<R> {
		let lights = factory.create_constant_buffer(512);
		let camera = factory.create_constant_buffer(1);
		let model = factory.create_constant_buffer(1);
		let fragment = factory.create_constant_buffer(1);
		let pso = factory.create_pipeline_simple(VERTEX_SRC, FRAGMENT_SRC, shaded::new())
		                 .unwrap();

		DrawShaded {
			camera: camera,
			model: model,
			fragment: fragment,
			lights: lights,
			pso: pso,
		}
	}

	pub fn begin_frame<C: gfx::CommandBuffer<R>>(&self,
	                                             encoder: &mut gfx::Encoder<R, C>,
	                                             target: &gfx::handle::RenderTargetView<R, ColorFormat>,
	                                             depth: &gfx::handle::DepthStencilView<R, DepthFormat>) {
		// clear
		encoder.clear(&target, BLACK);
		encoder.clear_depth(&depth, 1.0f32);
	}

	pub fn end_frame<C: gfx::CommandBuffer<R>, D: gfx::Device<Resources = R, CommandBuffer = C>>(&self,
	                                           encoder: &mut gfx::Encoder<R, C>,
	                                           device: &mut D) {
		encoder.flush(device);
	}

	pub fn cleanup<C: gfx::CommandBuffer<R>, D: gfx::Device<Resources = R, CommandBuffer = C>>(&self, device: &mut D) {
		device.cleanup();
	}

	pub fn setup<C: gfx::CommandBuffer<R>>(&self,
	                                       encoder: &mut gfx::Encoder<R, C>,
	                                       camera: &Camera,
	                                       lights: &Vec<PointLight>) {

		let mut lights_buf = lights.clone();

		let count = lights_buf.len();
		while lights_buf.len() < 512 {
			lights_buf.push(PointLight {
				propagation: [0., 0., 0., 0.],
				color: [0., 0., 0., 0.],
				center: [0., 0., 0., 0.],
			})
		}
		// only one draw call per frame just to prove the point
		encoder.update_buffer(&self.lights, &lights_buf[..], 0).unwrap();

		encoder.update_constant_buffer(&self.camera,
		                               &CameraArgs {
			                               proj: camera.projection.into(),
			                               view: camera.view.into(),
		                               });

		encoder.update_constant_buffer(&self.fragment, &FragmentArgs { light_count: count as i32 });
	}

	pub fn draw<C: gfx::CommandBuffer<R>>(&self,
	                                      encoder: &mut gfx::Encoder<R, C>,
	                                      vertices: &gfx::handle::Buffer<R, VertexPosNormal>,
	                                      indices: &gfx::Slice<R>,
	                                      transform: &M44,
	                                      color: &gfx::handle::RenderTargetView<R, ColorFormat>,
	                                      output_depth: &gfx::handle::DepthStencilView<R, DepthFormat>) {

		encoder.update_constant_buffer(&self.model, &ModelArgs { model: transform.clone().into() });

		encoder.draw(&indices,
		             &self.pso,
		             &shaded::Data {
			             vbuf: vertices.clone(),
			             fragment_args: self.fragment.clone(),
			             camera_args: self.camera.clone(),
			             model_args: self.model.clone(),
			             lights: self.lights.clone(),
			             out_ka: color.clone(),
			             out_depth: output_depth.clone(),
		             });
	}
}
