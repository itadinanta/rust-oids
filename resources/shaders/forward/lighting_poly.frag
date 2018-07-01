#version 150 core

#define MAX_NUM_TOTAL_LIGHTS 16
#define MAX_NUM_SHAPES 256

const float PI = 3.1415926535897932384626433832795;
const float PI_2 = 1.57079632679489661923;

struct Material {
    vec4 u_Emissive;
    vec4 u_Effect;
};

struct Light {
    vec4 propagation;
    vec4 center;
    vec4 color;
};

layout (std140) uniform cb_FragmentArgs {
	int u_LightCount;
};

layout (std140) uniform cb_MaterialArgs {
    Material material[MAX_NUM_SHAPES];
};

layout (std140) uniform u_Lights {
    Light light[MAX_NUM_TOTAL_LIGHTS];
};

in VertexData {
	vec4 Position;
	vec3 Normal;
	mat3 TBN;
	vec2 TexCoord;
	vec3 BaryCoord;
	flat int PrimIndex;
} v_In;

out vec4 o_Color;

const float EDGE_WIDTH = 0.25;
const float SPOKE_WIDTH = 0.1;
const float BASE_ALPHA = 0.0;
const float NORMAL_SLOPE = 0.6;
const float EFFECT_BIAS = 0.5;
const float EFFECT_GAIN = 4.0;
const float DIFFUSE_GAIN = 0.25;

void main() {
	vec4 kd = vec4(0.2, 0.2, 0.2, 1.0);
    vec4 ks = vec4(1.0, 1.0, 1.0, 1.0);
	vec4 kp = vec4(64.0, 32.0, 64.0, 1.0);

	float dx = 2 * clamp(v_In.TexCoord.x, 0, 1) - 1;
	float dy = 2 * clamp(v_In.TexCoord.y, 0, 1) - 1;
	float r = min(1, dx * dx + dy * dy);

	vec4 u_Emissive = material[v_In.PrimIndex].u_Emissive;
	vec4 u_Effect = material[v_In.PrimIndex].u_Effect;

	float f = clamp(u_Effect.x * 2, 0, 1);
	float e = clamp(abs(cos(r - u_Effect.y) + sin(dy - 2 * u_Effect.y)), 0, 1);

	float r_mask = smoothstep(1, 1 - EDGE_WIDTH, r); // soft edge
	float h_mask = clamp(1 - r / f, 0, 1) * smoothstep(SPOKE_WIDTH * e, 0, pow(r, f) * min(v_In.BaryCoord.y, v_In.BaryCoord.z)); // insets highlight

	vec4 color_diffuse = vec4(u_Emissive.rgb, clamp(f, 0, 1))  * DIFFUSE_GAIN;
	vec4 color_lambert = vec4(0,0,0,1);
	vec4 color_specular = vec4(0,0,0,1);
	vec4 highlight_color = u_Emissive * (e + EFFECT_BIAS) * f * EFFECT_GAIN;

	vec3 normal = v_In.TBN * normalize(vec3(dx, dy, NORMAL_SLOPE * sqrt(1 - r)));

	for (int i = 0; i < u_LightCount; i++) {
		vec4 delta = light[i].center - v_In.Position;
		float dist = length(delta);
		float inv_dist = 1. / dist;
		vec4 light_to_point_normal = delta * inv_dist;
		float intensity = dot(light[i].propagation.xyz,
				vec3(1., inv_dist, inv_dist * inv_dist));
		float lambert = max(0, dot(light_to_point_normal, vec4(normal, 0.0)));

		vec4 specular;
		if (lambert >= 0.0) {
//			Blinn-Phong:
			vec3 lightDir = light_to_point_normal.xyz;
			vec3 viewDir = vec3(0.0, 0.0, 1.0); // ortho, normalize(-v_In.Position.xyz); perspective
			vec3 halfDir = normalize(lightDir + viewDir); // can be done in vertex shader
			float specAngle = max(dot(halfDir, normal), 0.0);
			specular = pow(vec4(specAngle), kp);
		} else {
			specular = vec4(0.0);
		}
		vec4 light_intensity = light[i].color * intensity;
		color_lambert += light_intensity * kd * lambert;
		color_specular += light_intensity * ks * specular;
	}

	vec4 solid_color = color_diffuse + color_lambert + color_specular;

	o_Color.rgb = r_mask * (h_mask * highlight_color.rgb + solid_color.rgb); //(color.rgb * color.a + h_mask * highlight_color.rgb) * r_mask;
	o_Color.a = r_mask * color_diffuse.a; // clamp(r_mask * BASE_ALPHA * color.a, 0, 1);
}
