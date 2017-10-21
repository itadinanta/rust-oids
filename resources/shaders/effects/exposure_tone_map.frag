#version 150 core

uniform sampler2D t_Source;

in vec2 v_TexCoord;
in float v_Exposure;
out vec4 o_Color;

void main() {
	vec4 src = texture(t_Source, v_TexCoord, 0);
	vec4 linear_color = v_Exposure * src;
	// TODO: this should be done by RGBA pixelformat
	vec4 gamma_corrected = pow(linear_color, vec4(1/1.2));
	o_Color = vec4(gamma_corrected.rgb, src.a);
}
