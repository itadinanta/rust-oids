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

	o_Smooth = max(u_ExpAlpha * value, vec4(0.))
			+ max((1. - u_ExpAlpha) * acc, vec4(0.));
}
