#version 150 core

#define MAX_NUM_TOTAL_LIGHTS 16

const float PI = 3.1415926535897932384626433832795;
const float PI_2 = 1.57079632679489661923;

layout (std140) uniform cb_FragmentArgs {
	int u_LightCount;
};

layout (std140) uniform u_Lights {
	struct {
		vec4 propagation;
		vec4 center;
		vec4 color;
	} light[MAX_NUM_TOTAL_LIGHTS];
};

layout (std140) uniform cb_MaterialArgs {
	struct {
		vec4 u_Emissive;
		vec4 u_Effect;
	} material[1];
};

in VertexData {
	vec4 Position;
	vec3 Normal;
	mat3 TBN;
	vec2 TexCoord;
	flat int PrimIndex;
}v_In;

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

	float f = clamp(u_Effect.x * 2, 0, 1);
	float e = clamp(abs(cos(r - u_Effect.y) + sin(dy - 2 * u_Effect.y)), 0, 1);

	vec4 color = u_Emissive * e * f;

	vec3 normal = v_In.TBN * vec3(dx, dy, sqrt(1 - r));

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
