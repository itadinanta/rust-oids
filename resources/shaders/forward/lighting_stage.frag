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
	flat int PrimIndex;
} v_In;

out vec4 o_Color;

void main() {
	vec4 kd = vec4(0.2, 0.2, 0.2, 1.0);
    vec4 ks = vec4(1.0, 1.0, 1.0, 1.0);
	vec4 kp = vec4(64.0, 32.0, 64.0, 1.0);


	float dx = 2 * clamp(v_In.TexCoord.x, 0, 1) - 1;
	float dy = 2 * clamp(v_In.TexCoord.y, 0, 1) - 1;
	float r = min(1, dx * dx + dy * dy);

	vec4 u_Emissive = material[v_In.PrimIndex].u_Emissive;
	vec4 u_Effect = material[v_In.PrimIndex].u_Effect;

    float a = 2 * PI * (r - u_Effect.y);
	float f = u_Effect.x;

    vec4 color = u_Emissive;

	dx += (1 - r) * cos(a * 25.0) * f;
	dy += (1 - r) * sin(a * 19.1) * f;
	r = min(1, dx * dx + dy * dy);
	vec3 normal = v_In.TBN * normalize(vec3(dx, dy, sqrt(1 - r)));

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
		color += light[i].color * intensity * (kd * lambert + ks * specular);
	}
	// gl_FragDepth = bump;
	o_Color = color;
}
