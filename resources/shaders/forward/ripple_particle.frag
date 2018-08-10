// ripple_particle.frag
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
	vec3 Normal; // unused
	mat3 TBN; // unused
	vec2 TexCoord;
	vec3 BaryCoord;
	flat int PrimIndex;
}v_In;

out vec4 o_Color;

void main() {
	vec4 u_Emissive = material[v_In.PrimIndex].u_Emissive;
	vec4 u_Effect = material[v_In.PrimIndex].u_Effect;

    float intensity = u_Effect.x;
    float phase = u_Effect.y;
    float frequency = u_Effect.z;
    float ratio = u_Effect.w;

	float dx = 2 * clamp(v_In.TexCoord.x, 0, 1) - 1;
	float dy = 2 * clamp(v_In.TexCoord.y, 0, 1) - 1;
	float r = min(1, sqrt(dx * dx + dy * dy * ratio));

	float e = intensity; // or something
	float w = cos((phase - r) * frequency);
	float f = exp(-r) * w * w * float(r < 1);

	vec4 color = u_Emissive * e * f;
	o_Color.rgb = color.rgb * color.a;
	o_Color.a = 0;
}
