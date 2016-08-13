#version 150 core

uniform sampler2D t_VertexLuminance;

layout (std140) uniform cb_VertexArgs {
	float u_White;
	float u_Black;
};

in vec2 a_Pos;
in vec2 a_TexCoord;
out vec2 v_TexCoord;
out float v_Exposure;

void main() {
	v_TexCoord = a_TexCoord;
	float luminance = dot(vec3(0.2126, 0.7152, 0.0722),
			texture(t_VertexLuminance, vec2(0.5, 0.5)).rgb);

	// todo: interpolate flat
	v_Exposure = 1.0 / (u_Black + (u_White * luminance));
	gl_Position = vec4(a_Pos, 0.0, 1.0);
}
